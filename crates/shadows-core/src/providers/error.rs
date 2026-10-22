//! The refusals a provider write returns, and how a caller meets them.
use crate::CoreError;
use crate::db::StorageError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProvidersError {
    #[error("AYLUL provider not found: {0}")]
    NotFound(String),
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

impl From<ProvidersError> for CoreError {
    fn from(error: ProvidersError) -> Self {
        match error {
            ProvidersError::NotFound(slug) => Self::AylulProviderNotFound(slug),
            ProvidersError::Storage(error) => error.into(),
        }
    }
}
