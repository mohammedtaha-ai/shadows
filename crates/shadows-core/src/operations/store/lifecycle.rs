use super::transition::{Before, Transition};
use crate::db::{Actor, StorageError};
use crate::operations::{OperationId, Operations};
use crate::runtime::RuntimeInstanceId;
use crate::threads::ThreadId;
use sqlx::SqliteConnection;

impl Operations {
    pub(crate) async fn insert_pending_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        thread_id: &ThreadId,
        runtime_id: &RuntimeInstanceId,
        ts: &str,
    ) -> Result<Transition, StorageError> {
        crate::runtime::Runtime::require_active_in(conn, runtime_id).await?;
        sqlx::query(
            "INSERT INTO operation
           (id, kind, status_kind, thread_id, runtime_instance_id, created_at)
         VALUES (?, 'PlannerTurn', 'Pending', ?, ?, ?)",
        )
        .bind(op_id.as_str())
        .bind(thread_id.as_str())
        .bind(runtime_id.as_str())
        .bind(ts)
        .execute(&mut *conn)
        .await?;
        Transition::record(
            conn,
            op_id,
            Before::creating(thread_id.clone()),
            "Pending",
            Transition::event(
                "OperationCreated",
                Actor::system(),
                Some(serde_json::json!({ "kind": "PlannerTurn" })),
            ),
            ts,
        )
        .await
    }

    pub(crate) async fn started_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        expected_runtime: &RuntimeInstanceId,
        ts: &str,
    ) -> Result<Transition, StorageError> {
        let before = Before::read_before(conn, op_id).await?;
        let affected = sqlx::query(
            "UPDATE operation SET status_kind = 'Running', started_at = ?
                      WHERE id = ? AND status_kind = 'Pending' AND runtime_instance_id = ?",
        )
        .bind(ts)
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
        Transition::record(
            conn,
            op_id,
            Before::existed(before)?,
            "Running",
            Transition::event("OperationStarted", Actor::system(), None),
            ts,
        )
        .await
    }

    pub(crate) async fn completed_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        outcome: &str,
        invocation: Option<serde_json::Value>,
        ts: &str,
    ) -> Result<Transition, StorageError> {
        let before = Before::read_before(conn, op_id).await?;
        let affected = sqlx::query(
            "UPDATE operation
                        SET status_kind = 'Completed', outcome_json = ?, finished_at = ?
                      WHERE id = ? AND status_kind = 'Running'",
        )
        .bind(outcome)
        .bind(ts)
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
        let mut event = Transition::event("OperationCompleted", Actor::system(), None);
        if let Some(invocation) = invocation {
            event.payload_json = serde_json::json!({ "invocation": invocation }).to_string();
        }
        Transition::record(
            conn,
            op_id,
            Before::existed(before)?,
            "Completed",
            event,
            ts,
        )
        .await
    }

    pub(crate) async fn failed_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        stage: &str,
        reason: &str,
        ts: &str,
    ) -> Result<Transition, StorageError> {
        let before = Before::read_before(conn, op_id).await?;
        let affected = sqlx::query(
            "UPDATE operation
                        SET status_kind = 'Failed', failure_stage = ?, failure_reason = ?,
                            finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
        )
        .bind(stage)
        .bind(reason)
        .bind(ts)
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
        Transition::record(
            conn,
            op_id,
            Before::existed(before)?,
            "Failed",
            Transition::event(
                "OperationFailed",
                Actor::system(),
                Some(serde_json::json!({ "stage": stage })),
            ),
            ts,
        )
        .await
        .map(|t| t.with_detail(format!("{stage}: {reason}")))
    }

    pub(crate) async fn cancelled_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,

        ts: &str,
    ) -> Result<Transition, StorageError> {
        let before = Before::read_before(conn, op_id).await?;
        let affected = sqlx::query(
            "UPDATE operation SET status_kind = 'Cancelled', finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
        )
        .bind(ts)
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
        Transition::record(
            conn,
            op_id,
            Before::existed(before)?,
            "Cancelled",
            Transition::event("OperationCancelled", Actor::system(), None),
            ts,
        )
        .await
    }

    pub(crate) async fn request_cancellation_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        requester: Actor,
        ts: &str,
    ) -> Result<Option<Transition>, StorageError> {
        let before = Before::read_before(conn, op_id).await?;
        let affected = sqlx::query(
            "UPDATE operation
                        SET cancel_requested_at = ?, cancel_requested_by_kind = ?,
                            cancel_requested_by_id = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')
                        AND cancel_requested_at IS NULL",
        )
        .bind(ts)
        .bind(&requester.kind)
        .bind(&requester.id)
        .bind(op_id.as_str())
        .execute(&mut *conn)
        .await?
        .rows_affected();
        let before = Before::existed(before)?;
        if affected == 0 {
            return Ok(None);
        }
        let status = before.status().to_string();
        Transition::record(
            conn,
            op_id,
            before,
            &status,
            Transition::event("OperationCancellationRequested", requester, None),
            ts,
        )
        .await
        .map(Some)
    }
}
