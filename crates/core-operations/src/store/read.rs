use sqlx::SqliteConnection;

use crate::{OperationId, OperationRecord, Operations};
use core_runtime::RuntimeInstanceId;
use core_threads::ThreadId;

use core_db::StorageError;

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

impl Operations {
    pub async fn get_operation_in(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
    ) -> Result<OperationRecord, StorageError> {
        let row: OperationRow = sqlx::query_as(
            "SELECT id, kind, status_kind, thread_id, runtime_instance_id, outcome_json,
                    failure_stage, failure_reason, interrupt_reason, cancel_requested_at,
                    created_at, started_at, finished_at
               FROM operation WHERE id = ?",
        )
        .bind(op_id.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("operation"))?;
        Ok(into_operation(row))
    }

    pub async fn list_operations_for_thread_in(
        conn: &mut SqliteConnection,
        thread_id: &ThreadId,
    ) -> Result<Vec<OperationRecord>, StorageError> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT o.id, o.kind, o.status_kind, o.thread_id, o.runtime_instance_id,
                    o.outcome_json, o.failure_stage, o.failure_reason, o.interrupt_reason,
                    o.cancel_requested_at, o.created_at, o.started_at, o.finished_at
               FROM operation o
               JOIN durable_event e
                 ON e.operation_id = o.id AND e.kind = 'OperationCreated'
              WHERE o.thread_id = ?
              ORDER BY e.seq DESC",
        )
        .bind(thread_id.as_str())
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows.into_iter().map(into_operation).collect())
    }

    pub async fn non_terminal_operations_owned_by(
        &self,
        runtime: &RuntimeInstanceId,
    ) -> Result<Vec<OperationId>, StorageError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM operation
              WHERE runtime_instance_id = ? AND status_kind IN ('Pending','Running')
              ORDER BY id",
        )
        .bind(runtime.as_str())
        .fetch_all(self.store.storage.reader())
        .await?;
        Ok(ids.into_iter().map(OperationId::from_stored).collect())
    }
}

fn into_operation(r: OperationRow) -> OperationRecord {
    OperationRecord {
        id: OperationId::from_stored(r.0),
        kind: r.1,
        status_kind: r.2,
        thread_id: r.3,
        runtime_instance_id: RuntimeInstanceId::from_recorded(r.4),
        outcome_json: r.5,
        failure_stage: r.6,
        failure_reason: r.7,
        interrupt_reason: r.8,
        cancel_requested_at: r.9,
        created_at: r.10,
        started_at: r.11,
        finished_at: r.12,
    }
}

impl Operations {
    /// The newest Pending or Running turn, including the gap before live registration.
    pub async fn pending_or_running_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<Option<OperationId>, StorageError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT o.id FROM operation o
             JOIN durable_event e ON e.operation_id=o.id AND e.kind='OperationCreated'
             WHERE o.thread_id=? AND o.status_kind IN ('Pending','Running') ORDER BY e.seq DESC",
        )
        .bind(thread.as_str())
        .fetch_optional(conn)
        .await?;
        Ok(id.map(OperationId::from_stored))
    }
}
