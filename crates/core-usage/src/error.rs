//! Refusals owned by usage reporting.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    #[error("invalid profile: invalid usage period")]
    InvalidPeriod,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
