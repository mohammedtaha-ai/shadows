//! Atomic thread removal with its remembered permission choice.
use super::HarnessStore;
use crate::db::{CommandContext, StorageError, now};
use crate::harness::Harness;
use crate::threads::{PlanningThread, ThreadId, Threads};

impl HarnessStore {
    pub(in crate::harness) async fn remove_thread(
        &self,
        command: &CommandContext,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let (command, thread) = (command.clone(), thread.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let removed = Threads::remove_in(conn, &command, &thread, &now()).await?;
                    Harness::remove_mode_in(conn, &thread).await?;
                    Ok(removed)
                })
            })
            .await
    }
}

impl Harness {
    pub(crate) async fn remove_thread_record(
        &self,
        command: &CommandContext,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        self.store.remove_thread(command, thread).await
    }

    #[cfg(feature = "test-support")]
    pub(crate) async fn remove_thread_record_for_test(
        storage: &std::sync::Arc<crate::db::Storage>,
        command: &CommandContext,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        HarnessStore::with_storage(storage.clone())
            .remove_thread(command, thread)
            .await
    }
}
