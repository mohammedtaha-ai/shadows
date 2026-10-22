//! The refusals a library write returns, and how a caller meets them.
use crate::CoreError;
use crate::agents::AgentsError;
use crate::db::StorageError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExtensionsError {
    #[error("extension not found")]
    NotFound,
    #[error("extension name is already in use")]
    NameConflict,
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error(transparent)]
    Agent(#[from] AgentsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for ExtensionsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<serde_json::Error> for ExtensionsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<ExtensionsError> for CoreError {
    fn from(error: ExtensionsError) -> Self {
        match error {
            ExtensionsError::NotFound => Self::ExtensionNotFound,
            ExtensionsError::NameConflict => Self::ExtensionNameConflict,
            ExtensionsError::Invalid(reason) => Self::ProfileInvalid(reason),
            ExtensionsError::Agent(error) => error.into(),
            ExtensionsError::Storage(error) => error.into(),
        }
    }
}
