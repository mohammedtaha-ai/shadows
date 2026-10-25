//! Grant authority failures with stable application codes.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum GrantsError {
    #[error("the grant is unknown or revoked")]
    GrantInvalid,
    #[error("outside what the grant allows")]
    GrantScope,
    #[error("{0}")]
    GrantScopeMessage(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
impl From<sqlx::Error> for GrantsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}
impl From<serde_json::Error> for GrantsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}
