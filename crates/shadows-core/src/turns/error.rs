//! Turn-domain refusals translated to the shared application vocabulary.
use crate::{CoreError, ErrorCode};
#[derive(Debug, thiserror::Error)]
pub(super) enum TurnsError {
    #[error("the thread has a turn running")]
    ThreadBusy,
    #[error("the waiting message was already sent or removed")]
    QueuedMessageGone,
}
impl From<TurnsError> for CoreError {
    fn from(error: TurnsError) -> Self {
        let code = match error {
            TurnsError::ThreadBusy => ErrorCode::ThreadBusy,
            TurnsError::QueuedMessageGone => ErrorCode::QueuedMessageGone,
        };
        Self::Refused {
            code,
            message: error.to_string(),
        }
    }
}
