use crate::events::{Actor, DurableEvent};
use crate::operation::{FailureStage, Operation, OperationId};
use crate::runtime::RuntimeInstanceId;
use crate::thread::ThreadId;

use super::{Storage, StorageError, now, transition::record};

/// The thirteen `operation` columns `get_operation` reads back, in select
/// order. A row alias, not a domain type: `get_operation` maps it into
/// `Operation` immediately below.
type OperationRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

impl Storage {
    /// TX #1 of the two-phase spawn. Spec §2.7: Pending is persisted before
    /// anything is spawned, so a crash between here and spawn is recoverable.
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
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO operation
                       (id, kind, status_kind, thread_id, runtime_instance_id, created_at)
                     VALUES (?, 'PlannerTurn', 'Pending', ?, ?, ?)",
                )
                .bind(op_id.as_str())
                .bind(thread_id.as_str())
                .bind(runtime_id.as_str())
                .bind(&ts)
                .execute(&mut *conn)
                .await?;

                record(
                    conn,
                    &op_id,
                    DurableEvent::new("OperationCreated", Actor::system())
                        .with_payload(serde_json::json!({ "kind": "PlannerTurn" })),
                    &ts,
                )
                .await
            })
        })
        .await?;
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
                    DurableEvent::new("OperationStarted", Actor::system()),
                    &ts,
                )
                .await
            })
        })
        .await
    }

    pub async fn mark_operation_completed(
        &self,
        op_id: &OperationId,
        outcome: serde_json::Value,
    ) -> Result<(), StorageError> {
        let (op_id, outcome, ts) = (op_id.clone(), outcome.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
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
                record(
                    conn,
                    &op_id,
                    DurableEvent::new("OperationCompleted", Actor::system()),
                    &ts,
                )
                .await
            })
        })
        .await
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
                    DurableEvent::new("OperationFailed", Actor::system())
                        .with_payload(serde_json::json!({ "stage": stage.as_str() })),
                    &ts,
                )
                .await
            })
        })
        .await
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
        self.write_txn(move |conn| {
            Box::pin(async move {
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
                if affected == 0 {
                    // Already requested, or already terminal. Spec §2.3: a
                    // repeat is idempotent and runs no process effects.
                    return Ok(());
                }
                record(
                    conn,
                    &op_id,
                    DurableEvent::new("OperationCancellationRequested", requester),
                    &ts,
                )
                .await
            })
        })
        .await
    }

    /// TX #2 of cancellation, written ONLY after the process tree is confirmed
    /// gone. Spec §2.3. The CAS refuses to overwrite a terminal state, which is
    /// what makes a natural exit win the race (§8.4 case 4 and 5).
    pub async fn mark_operation_cancelled(&self, op_id: &OperationId) -> Result<(), StorageError> {
        let (op_id, ts) = (op_id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
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
                    DurableEvent::new("OperationCancelled", Actor::system()),
                    &ts,
                )
                .await
            })
        })
        .await
    }

    pub async fn get_operation(&self, op_id: &OperationId) -> Result<Operation, StorageError> {
        let r: OperationRow = sqlx::query_as(
            "SELECT id, kind, status_kind, thread_id, runtime_instance_id, outcome_json,
                    failure_stage, failure_reason, interrupt_reason, cancel_requested_at,
                    created_at, started_at, finished_at
               FROM operation WHERE id = ?",
        )
        .bind(op_id.as_str())
        .fetch_optional(self.reader())
        .await?
        .ok_or(StorageError::NotFound("operation"))?;
        Ok(Operation {
            id: OperationId::from_stored(r.0),
            kind: r.1,
            status_kind: r.2,
            thread_id: r.3,
            runtime_instance_id: RuntimeInstanceId::from_stored(r.4),
            outcome_json: r.5,
            failure_stage: r.6,
            failure_reason: r.7,
            interrupt_reason: r.8,
            cancel_requested_at: r.9,
            created_at: r.10,
            started_at: r.11,
            finished_at: r.12,
        })
    }
}
