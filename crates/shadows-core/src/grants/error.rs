//! Grant authority failures with stable application codes.
use crate::db::StorageError;
use crate::{CoreError, ErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum GrantsError {
    #[error("the grant is unknown or revoked")]
    GrantInvalid,
    #[error("outside what the grant allows")]
    GrantScope,
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
impl From<GrantsError> for CoreError {
    fn from(error: GrantsError) -> Self {
        let code = match error {
            GrantsError::GrantInvalid => ErrorCode::GrantInvalid,
            GrantsError::GrantScope => ErrorCode::GrantScope,
            GrantsError::Storage(error) => return error.into(),
        };
        Self::Refused {
            code,
            message: error.to_string(),
        }
    }
}
