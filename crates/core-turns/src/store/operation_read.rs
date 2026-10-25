use crate::model::{InvocationView, Operation, OperationId};
use core_db::StorageError;
use core_harness::Harness;
use core_operations::Operations;
use core_threads::ThreadId;
impl super::TurnsStore {
    pub async fn get_operation(&self, op: &OperationId) -> Result<Operation, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        let row = Operations::get_operation_in(&mut conn, op).await?;
        Ok(into_operation(
            row,
            Harness::invocation_in(&mut conn, op.as_str()).await?,
        ))
    }
    pub async fn list_operations_for_thread(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        let rows = Operations::list_operations_for_thread_in(&mut conn, thread).await?;
        let ids: Vec<_> = rows.iter().map(|row| row.id.as_str()).collect();
        let mut invocations = Harness::invocations_in(&mut conn, &ids).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let invocation = invocations.remove(row.id.as_str());
                into_operation(row, invocation)
            })
            .collect())
    }
}

fn into_operation(
    r: core_operations::OperationRecord,
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
