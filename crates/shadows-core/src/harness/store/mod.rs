//! Private harness persistence over the shared storage handle.
mod invocation;
mod invocations;
mod limits;
mod mode;
mod preferences;
mod reads;
mod thread;
use crate::db::Storage;
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
