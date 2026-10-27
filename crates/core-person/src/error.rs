//! Refusals owned by person preferences.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum PersonError {
    #[error("invalid profile: person patch has no fields")]
    EmptyPatch,
    #[error("invalid profile: invalid person name")]
    InvalidName,
    #[error("invalid profile: person name is too long")]
    NameTooLong,
    #[error("invalid profile: invalid language tag")]
    InvalidLanguage,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
