//! Atomic conversation forks with their permission choice.
use super::Harness;
use crate::{CoreError, Operations};
use core_db::{CommandContext, Storage};
use core_threads::{PlanningThread, ThreadEntryId, ThreadId, Threads};

impl Harness {
    pub async fn fork_thread(
        &self,
        command_id: String,
        source: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<PlanningThread, CoreError> {
        let command = CommandContext::user(
            command_id,
            "thread.fork",
            serde_json::json!({"thread_id": source, "at_entry_id": at}),
        );
        let (source, at) = (source.clone(), at.clone());
        self.store
            .storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &command, "Thread", source.as_str()).await?
                    {
                        return Ok::<_, CoreError>(
                            Threads::get_in(conn, &ThreadId::from_recorded(id)).await?,
                        );
                    }
                    let completed = Operations::fork_eligible_in(conn, &source, &at).await?;
                    let fork =
                        Threads::fork_in(conn, &command, &source, &at, completed, &Storage::now())
                            .await?;
                    Self::copy_mode_in(conn, &source, &fork.id).await?;
                    Ok(fork)
                })
            })
            .await
    }
}
