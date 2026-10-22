use crate::db::StorageError;
use crate::operations::Operations;
use crate::threads::ThreadId;
use crate::turns::model::{InvocationView, Operation, OperationId};
use sqlx::SqliteConnection;
impl super::TurnsStore<'_> {
    pub async fn get_operation(&self, op: &OperationId) -> Result<Operation, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        let row = Operations::get_operation_in(&mut conn, op).await?;
        Ok(into_operation(row, invocation_of(&mut conn, op).await?))
    }
    pub async fn list_operations_for_thread(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        let rows = Operations::list_operations_for_thread_in(&mut conn, thread).await?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let invocation = invocation_of(&mut conn, &row.id).await?;
            result.push(into_operation(row, invocation));
        }
        Ok(result)
    }
}
type InvocationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
);

fn into_view(r: InvocationRow) -> InvocationView {
    InvocationView {
        harness_kind: r.1,
        harness_version: r.2,
        agent_version: r.3,
        requested_model: r.4,
        requested_mode: r.5,
        requested_effort: r.6,
        observed_model: r.7,
        context_used: r.8,
        context_window: r.9,
    }
}

pub(super) async fn invocation_of(
    conn: &mut SqliteConnection,
    op: &OperationId,
) -> Result<Option<InvocationView>, StorageError> {
    let row: Option<InvocationRow> = sqlx::query_as(
        "SELECT operation_id, harness_kind, harness_version, agent_version,
                requested_model, requested_mode, requested_effort,
                observed_model, context_used, context_window
           FROM agent_invocation WHERE operation_id = ?",
    )
    .bind(op.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(into_view))
}

fn into_operation(
    r: crate::operations::OperationRecord,
    invocation: Option<InvocationView>,
) -> Operation {
    Operation {
        id: r.id,
        kind: r.kind,
        status_kind: r.status_kind,
        thread_id: r.thread_id,
        runtime_instance_id: r.runtime_instance_id,
        outcome_json: r.outcome_json,
        failure_stage: r.failure_stage,
        failure_reason: r.failure_reason,
        interrupt_reason: r.interrupt_reason,
        cancel_requested_at: r.cancel_requested_at,
        created_at: r.created_at,
        started_at: r.started_at,
        finished_at: r.finished_at,
        invocation,
    }
}
