//! The refusals an agent write returns, and how a caller meets them.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum AgentsError {
    #[error("AYLUL agent not found: {0}")]
    NotFound(String),
    #[error("provider is used by: {0}")]
    ProviderInUse(String),
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("AYLUL is not configured")]
    NotConfigured,
    #[error(transparent)]
    Provider(#[from] core_providers::ProvidersError),
    #[error(transparent)]
    Secrets(#[from] core_secrets::SecretsError),
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for AgentsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}
