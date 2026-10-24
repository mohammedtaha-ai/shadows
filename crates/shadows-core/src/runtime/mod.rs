use core_db::newtype_id;
use core_db::{Storage, StorageError};
use std::sync::Arc;
mod store;
pub use store::StopKind;
newtype_id! {
    /// Pins an operation transition to the process that owns it.
    RuntimeInstanceId
}
pub struct Runtime {
    pub instance_id: RuntimeInstanceId,
    #[cfg(feature = "test-support")]
    storage: Arc<Storage>,
}
impl Runtime {
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
