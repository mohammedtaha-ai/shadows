//! The refusals an agent write returns, and how a caller meets them.
use crate::CoreError;
use crate::db::StorageError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AgentsError {
    #[error("AYLUL agent not found: {0}")]
    NotFound(String),
    #[error("provider is used by: {0}")]
    ProviderInUse(String),
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for AgentsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<AgentsError> for CoreError {
    fn from(error: AgentsError) -> Self {
        match error {
            AgentsError::NotFound(slug) => Self::AylulProfileNotFound(slug),
            AgentsError::ProviderInUse(names) => Self::ProviderInUse(names),
            AgentsError::Invalid(reason) => Self::ProfileInvalid(reason),
            AgentsError::Storage(error) => error.into(),
        }
    }
}
