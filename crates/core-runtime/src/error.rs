//! Runtime startup failures over the shared foundation.

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
