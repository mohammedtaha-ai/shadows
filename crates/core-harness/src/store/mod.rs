//! Private harness persistence over the shared storage handle.
mod invocation;
mod invocations;
mod limits;
mod mode;
mod permission;
mod preferences;
mod reads;
mod thread;
use core_db::Storage;
pub(super) use preferences::Remembered;
use std::sync::Arc;

pub(super) struct HarnessStore {
    pub(super) storage: Arc<Storage>,
}

impl HarnessStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
}
