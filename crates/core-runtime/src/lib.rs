//! Runtime identity registration over the shared database foundation.

use core_db::{Storage, StorageError};
use std::sync::Arc;
mod model;
mod store;
pub use model::{RuntimeInstanceId, StopKind};
pub struct Runtime {
    instance_id: RuntimeInstanceId,
    #[cfg(feature = "test-support")]
    storage: Arc<Storage>,
}
impl Runtime {
    pub fn instance_id(&self) -> &RuntimeInstanceId {
        &self.instance_id
    }

    pub async fn start(storage: Arc<Storage>) -> Result<Self, StorageError> {
        let instance_id = store::RuntimeStore { storage: &storage }
            .register_runtime_instance(env!("CARGO_PKG_VERSION"))
            .await?;
        Ok(Self {
            instance_id,
            #[cfg(feature = "test-support")]
            storage,
        })
    }
    #[cfg(feature = "test-support")]
    pub fn storage_for_test(&self) -> Arc<Storage> {
        self.storage.clone()
    }
    #[cfg(feature = "test-support")]
    pub async fn register_runtime_instance_for_test(
        storage: &Storage,
        version: &str,
    ) -> Result<RuntimeInstanceId, StorageError> {
        store::RuntimeStore { storage }
            .register_runtime_instance(version)
            .await
    }
}

impl Runtime {
    /// The caller composes unfinished-work checks in the same transaction.
    pub async fn stop_in(
        conn: &mut core_db::Tx,
        id: &str,
        kind: StopKind,
        ts: &str,
    ) -> Result<(), StorageError> {
        store::RuntimeStore::stop_in(conn, id, kind, ts).await
    }

    pub async fn require_active_in(
        conn: &mut core_db::Tx,
        id: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        store::RuntimeStore::require_active_in(conn, id).await
    }

    pub async fn stop_kind_in(
        conn: &mut core_db::Tx,
        id: &str,
    ) -> Result<Option<String>, StorageError> {
        store::RuntimeStore::stop_kind_in(conn, id).await
    }
}
