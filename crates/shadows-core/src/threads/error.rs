//! Thread refusals and their stable application error mapping.
use crate::{CoreError, ErrorCode};
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ThreadsError {
    #[error("the thread's harness is fixed: it already ran a turn, or it is a fork")]
    HarnessLocked,
    #[error("only the thread's last entry, written by a completed turn, can be forked from")]
    ForkPointNotSupported,
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for ThreadsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<ThreadsError> for CoreError {
    fn from(error: ThreadsError) -> Self {
        let code = match error {
            ThreadsError::HarnessLocked => ErrorCode::HarnessLocked,
            ThreadsError::ForkPointNotSupported => ErrorCode::ForkPointNotSupported,
            ThreadsError::Storage(error) => return error.into(),
        };
        Self::Refused {
            code,
            message: error.to_string(),
        }
    }
}
