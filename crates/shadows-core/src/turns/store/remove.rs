//! Durable thread removal composes each owner in one transaction.
use core_db::{Actor, CommandContext, Storage, StorageError};
use core_grants::Grants;
use core_harness::Harness;
use core_threads::{PlanningThread, ThreadId, Threads};
impl super::TurnsStore {
    pub(in crate::turns) async fn remove_thread(
        &self,
        command: &CommandContext,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let (command, thread) = (command.clone(), thread.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let ts = Storage::now();
                    let removed =
                        if CommandContext::classify(conn, &command, "Thread", thread.as_str())
                            .await?
                            .is_some()
                        {
                            Threads::get_in(conn, &thread).await?
                        } else {
                            let removed = Threads::remove_in(conn, &thread, &ts).await?;
                            Grants::revoke_thread_grants_in(
                                conn,
                                &thread,
                                Actor::user(&command.principal_id),
                                &ts,
                            )
                            .await?;
                            Threads::record_removal_in(conn, &command, &removed, &ts).await?;
                            removed
                        };
                    Harness::remove_mode_in(conn, &thread).await?;
                    Ok(removed)
                })
            })
            .await
    }
}
