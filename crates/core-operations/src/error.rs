//! Refusal and failure vocabulary for operations.

use core_db::StorageError;
use core_threads::ThreadsError;

#[derive(Debug, thiserror::Error)]
pub enum OperationsError {
    #[error("the thread has a turn running")]
    ThreadBusy,
    #[error(transparent)]
    Threads(#[from] ThreadsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
