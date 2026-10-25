//! The refusals a library write returns, and how a caller meets them.
use core_agents::AgentsError;
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ExtensionsError {
    #[error("extension not found")]
    NotFound,
    #[error("extension name is already in use")]
    NameConflict,
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("AYLUL is not configured")]
    NotConfigured,
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
