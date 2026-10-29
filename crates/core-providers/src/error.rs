//! The refusals a provider write returns, and how a caller meets them.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ProvidersError {
    #[error("AYLUL provider not found: {0}")]
    NotFound(String),
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    #[error("{message}")]
    InvalidField { field: String, message: String },
    #[error("AYLUL key is missing: {0}")]
    KeyMissing(String),
    #[error(transparent)]
    Secrets(#[from] core_secrets::SecretsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for ProvidersError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<serde_json::Error> for ProvidersError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}
