use crate::db::{CommandContext, StorageError, classify, now};
use crate::threads::{PlanningThread, ThreadEntryId, ThreadId, Threads};
use crate::{CoreError, Operations};
impl Operations {
    pub async fn fork_thread(
        &self,
        command_id: String,
        source: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<PlanningThread, CoreError> {
        let ctx = CommandContext::user(
            command_id,
            "thread.fork",
            serde_json::json!({ "thread_id": source, "at_entry_id": at }),
        );
        let (source, at) = (source.clone(), at.clone());
        Ok(self
            .store
            .storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(id) = classify(conn, &ctx, "Thread", source.as_str()).await? {
                        return Ok(Threads::get_in(conn, &ThreadId::from_stored(id)).await?);
                    }
                    Threads::require_live_in(conn, &source).await?;
                    if Self::has_open_in(conn, &source).await? {
                        return Err(crate::threads::ThreadsError::from(StorageError::ThreadBusy));
                    }
                    let operation = Threads::fork_operation_in(conn, &source, &at).await?;
                    let completed = match operation {
                        Some(op) => Self::completed_in_snapshot(conn, &op).await?,
                        None => false,
                    };
                    Threads::fork_in(conn, &ctx, &source, &at, completed, &now()).await
                })
            })
            .await?)
    }
}
