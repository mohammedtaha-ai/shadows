//! Refusals owned by project standards.
#[derive(Debug, thiserror::Error)]
pub enum StandardsError {
    #[error("{0}")]
    InvalidAdditions(String),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
