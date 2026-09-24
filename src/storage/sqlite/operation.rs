//! One job: an operation's state transitions, each written with its journal
//! event. Reading operations back is `operation_read.rs`.

use crate::events::{Actor, DurableEvent};
use crate::operation::{FailureStage, OperationId};
use crate::runtime::RuntimeInstanceId;
use crate::thread::ThreadId;

use sqlx::SqliteConnection;

use super::operation_read::invocation_of;
use super::transition::{Before, Transition, existed, read_before, record};
use super::{Storage, StorageError, now};
use crate::agent::events::TurnObservation;

/// Inserts a `Pending` operation and its `OperationCreated` event inside the
/// caller's transaction; refused for a stopped runtime (see
/// `create_pending_operation`). The turn command (§12.7) calls it inside its
/// own transaction. The caller logs the returned transition after commit.
pub(super) async fn insert_pending(
    conn: &mut SqliteConnection,
    op_id: &OperationId,
    thread_id: &ThreadId,
    runtime_id: &RuntimeInstanceId,
    ts: &str,
) -> Result<Transition, StorageError> {
    let affected = sqlx::query(
        "INSERT INTO operation
           (id, kind, status_kind, thread_id, runtime_instance_id, created_at)
         SELECT ?, 'PlannerTurn', 'Pending', ?, r.id, ?
           FROM runtime_instance r
          WHERE r.id = ? AND r.stopped_at IS NULL",
    )
    .bind(op_id.as_str())
    .bind(thread_id.as_str())
    .bind(ts)
    .bind(runtime_id.as_str())
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(StorageError::TransitionConflict {
            expected: "a runtime that has not stopped".into(),
            found: "a stopped or unknown runtime".into(),
        });
    }
    record(
        conn,
        op_id,
        Before::creating(thread_id.clone()),
        "Pending",
        DurableEvent::new("OperationCreated", Actor::system())
            .with_payload(serde_json::json!({ "kind": "PlannerTurn" })),
        ts,
    )
    .await
}

impl Storage {
    /// TX #1 of the two-phase spawn. Spec §2.7: Pending is persisted before
    /// anything is spawned, so a crash between here and spawn is recoverable.
    ///
    /// Refused with `TransitionConflict` once the owning runtime's stop is
    /// recorded (§8.5): a stopped runtime accepts no work, and an operation
    /// born after a `Graceful` stop would make that record a lie after the
    /// fact. `stop_runtime_instance` guards the other side of the same line.
    pub async fn create_pending_operation(
        &self,
        thread_id: &ThreadId,
        runtime_instance_id: &RuntimeInstanceId,
    ) -> Result<OperationId, StorageError> {
        let id = OperationId::generate();
        let (op_id, thread_id, runtime_id, ts) = (
            id.clone(),
            thread_id.clone(),
            runtime_instance_id.clone(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(
                async move { insert_pending(conn, &op_id, &thread_id, &runtime_id, &ts).await },
            )
        })
        .await?
        .log();
        Ok(id)
    }

    /// TX #2 of the two-phase spawn. Spec §8.3: no Operation becomes Running
    /// before its handle is registered and this compare-and-swap commits.
    /// The CAS pins id, expected status, AND owning runtime.
    pub async fn mark_operation_started(
        &self,
        op_id: &OperationId,
        expected_runtime: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        let (op_id, expected_runtime, ts) = (op_id.clone(), expected_runtime.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let before = read_before(conn, &op_id).await?;
                let affected = sqlx::query(
                    "UPDATE operation SET status_kind = 'Running', started_at = ?
                      WHERE id = ? AND status_kind = 'Pending' AND runtime_instance_id = ?",
                )
                .bind(&ts)
                .bind(op_id.as_str())
                .bind(expected_runtime.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending, owned by this runtime".into(),
                        found: "another status or another owner".into(),
                    });
                }
                record(
                    conn,
                    &op_id,
                    existed(before)?,
                    "Running",
                    DurableEvent::new("OperationStarted", Actor::system()),
                    &ts,
                )
                .await
            })
        })
        .await?
        .log();
        Ok(())
    }

    /// Completes a turn with what the harness reported during it (§12.7):
    /// the invocation's observed columns are written in this transaction,
    /// once, and the `OperationCompleted` event carries the invocation. An
    /// operation with no invocation completes without one.
    pub async fn mark_operation_completed(
        &self,
        op_id: &OperationId,
        outcome: serde_json::Value,
        observation: &TurnObservation,
    ) -> Result<(), StorageError> {
        let (op_id, outcome, seen, ts) = (
            op_id.clone(),
            outcome.to_string(),
            observation.clone(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                let before = read_before(conn, &op_id).await?;
                let affected = sqlx::query(
                    "UPDATE operation
                        SET status_kind = 'Completed', outcome_json = ?, finished_at = ?
                      WHERE id = ? AND status_kind = 'Running'",
                )
                .bind(&outcome)
                .bind(&ts)
                .bind(op_id.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Running".into(),
                        found: "another status".into(),
                    });
                }
                let as_int = |v: Option<u64>| v.and_then(|n| i64::try_from(n).ok());
                sqlx::query(
                    "UPDATE agent_invocation
                        SET native_session_id = ?, observed_model = ?, context_used = ?,
                            context_window = ?
                      WHERE operation_id = ?",
                )
                .bind(&seen.native_session_id)
                .bind(&seen.observed_model)
                .bind(as_int(seen.context_used))
                .bind(as_int(seen.context_window))
                .bind(op_id.as_str())
                .execute(&mut *conn)
                .await?;
                let mut event = DurableEvent::new("OperationCompleted", Actor::system());
                if let Some(invocation) = invocation_of(conn, &op_id).await? {
                    event = event.with_payload(serde_json::json!({ "invocation": invocation }));
                }
                record(conn, &op_id, existed(before)?, "Completed", event, &ts).await
            })
        })
        .await?
        .log();
        Ok(())
    }

    pub async fn mark_operation_failed(
        &self,
        op_id: &OperationId,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), StorageError> {
        let (op_id, reason, ts) = (op_id.clone(), reason.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let before = read_before(conn, &op_id).await?;
                let affected = sqlx::query(
                    "UPDATE operation
                        SET status_kind = 'Failed', failure_stage = ?, failure_reason = ?,
                            finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
                )
                .bind(stage.as_str())
                .bind(&reason)
                .bind(&ts)
                .bind(op_id.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending or Running".into(),
                        found: "already terminal".into(),
                    });
                }
                record(
                    conn,
                    &op_id,
                    existed(before)?,
                    "Failed",
                    DurableEvent::new("OperationFailed", Actor::system())
                        .with_payload(serde_json::json!({ "stage": stage.as_str() })),
                    &ts,
                )
                .await
                .map(|t| t.with_detail(format!("{}: {reason}", stage.as_str())))
            })
        })
        .await?
        .log();
        Ok(())
    }

    /// TX #1 of cancellation. Spec §2.3: this records that a stop was asked
    /// for. It does not stop anything and it does not make the operation
    /// terminal.
    pub async fn request_cancellation(
        &self,
        op_id: &OperationId,
        requester: Actor,
    ) -> Result<(), StorageError> {
        let (op_id, requester, ts) = (op_id.clone(), requester, now());
        let transition = self
            .write_txn(move |conn| {
                Box::pin(async move {
                    let before = read_before(conn, &op_id).await?;
                    let affected = sqlx::query(
                        "UPDATE operation
                        SET cancel_requested_at = ?, cancel_requested_by_kind = ?,
                            cancel_requested_by_id = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')
                        AND cancel_requested_at IS NULL",
                    )
                    .bind(&ts)
                    .bind(&requester.kind)
                    .bind(&requester.id)
                    .bind(op_id.as_str())
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
                    // Asked first: an operation that does not exist is not one
                    // whose cancellation was already requested.
                    let before = existed(before)?;
                    if affected == 0 {
                        // Already requested, or already terminal. Spec §2.3: a
                        // repeat is idempotent and runs no process effects.
                        return Ok(None);
                    }
                    // A request leaves the status where it was (spec §2.3).
                    let status = before.status().to_string();
                    record(
                        conn,
                        &op_id,
                        before,
                        &status,
                        DurableEvent::new("OperationCancellationRequested", requester),
                        &ts,
                    )
                    .await
                    .map(Some)
                })
            })
            .await?;
        if let Some(transition) = transition {
            transition.log();
        }
        Ok(())
    }

    /// TX #2 of cancellation, written ONLY after the process tree is confirmed
    /// gone. Spec §2.3. The CAS refuses to overwrite a terminal state, which is
    /// what makes a natural exit win the race (§8.4 case 4 and 5).
    pub async fn mark_operation_cancelled(&self, op_id: &OperationId) -> Result<(), StorageError> {
        let (op_id, ts) = (op_id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let before = read_before(conn, &op_id).await?;
                let affected = sqlx::query(
                    "UPDATE operation SET status_kind = 'Cancelled', finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
                )
                .bind(&ts)
                .bind(op_id.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending or Running".into(),
                        found: "already terminal".into(),
                    });
                }
                record(
                    conn,
                    &op_id,
                    existed(before)?,
                    "Cancelled",
                    DurableEvent::new("OperationCancelled", Actor::system()),
                    &ts,
                )
                .await
            })
        })
        .await?
        .log();
        Ok(())
    }
}
