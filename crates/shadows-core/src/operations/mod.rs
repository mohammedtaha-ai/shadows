//! Durable operation lifecycle, independent of live turn execution.
mod fork;
mod model;
mod store;
#[cfg(feature = "test-support")]
mod testing;
use core_db::{Storage, StorageError};
use core_threads::ThreadId;
pub(crate) use model::OperationRecord;
pub use model::{OperationId, ReconcileReport};
use std::sync::Arc;
#[derive(Clone)]
pub struct Operations {
    store: Arc<store::OperationsStore>,
}
impl Operations {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: Arc::new(store::OperationsStore { storage }),
        }
    }
    pub async fn thread_is_busy(&self, thread: &ThreadId) -> Result<bool, StorageError> {
        let mut conn = self.store.storage.reader().acquire().await?;
        Self::has_open_in(&mut conn, thread).await
    }
}

impl Operations {
    pub async fn running_for(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<OperationId>, StorageError> {
        let mut conn = self.store.storage.reader().acquire().await?;
        Self::running_for_in(&mut conn, thread).await
    }
}
