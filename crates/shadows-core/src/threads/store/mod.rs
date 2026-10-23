//! Threads' private persistence over the shared database handle.

mod attachment;
mod entry;
mod fork;
mod records;
mod remove;
mod thread;
mod title;

use crate::db::Storage;
use std::sync::Arc;

pub(super) struct ThreadsStore {
    pub(super) storage: Arc<Storage>,
}

impl ThreadsStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
}
