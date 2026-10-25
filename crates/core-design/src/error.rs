//! Design lifecycle failures and refusals.
use core_db::StorageError;
use core_grants::GrantsError;
#[derive(Debug, thiserror::Error)]
pub enum DesignError {
    #[error("{0}")]
    StageBlocked(String),
    #[error("a new version needs its reason")]
    ReasonMissing,
    #[error("{0}")]
    InvalidCommand(String),
    #[error("{0}")]
    GrantScope(String),
    #[error(transparent)]
    Grants(#[from] core_grants::GrantsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
impl From<sqlx::Error> for DesignError {
    fn from(error: sqlx::Error) -> Self {
        StorageError::from(error).into()
    }
}
impl From<serde_json::Error> for DesignError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::from(error).into()
    }
}
impl DesignError {
    pub(crate) fn invalid_constraint(error: impl Into<Self>) -> Self {
        match error.into() {
            Self::Storage(StorageError::Constraint(message))
            | Self::Grants(GrantsError::Storage(StorageError::Constraint(message))) => {
                Self::InvalidCommand(message)
            }
            other => other,
        }
    }
}
