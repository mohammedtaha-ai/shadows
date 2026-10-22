use crate::db::Storage;
use std::sync::Arc;
mod checks;
mod lifecycle;
mod read;
mod recovery;
mod runtime_stop;
mod transition;
pub(super) struct OperationsStore {
    pub(super) storage: Arc<Storage>,
}
