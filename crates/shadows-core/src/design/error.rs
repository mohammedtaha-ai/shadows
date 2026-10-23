//! Design lifecycle refusals with no plan-domain dependency.
use crate::{CoreError, ErrorCode};
#[derive(Debug, thiserror::Error)]
pub(crate) enum DesignError {
    #[error("{0}")]
    StageBlocked(String),
    #[error("a new version needs its reason")]
    ReasonMissing,
}
impl From<DesignError> for CoreError {
    fn from(error: DesignError) -> Self {
        match error {
            DesignError::StageBlocked(message) => Self::Refused {
                code: ErrorCode::StageBlocked,
                message,
            },
            DesignError::ReasonMissing => Self::ReasonMissing,
        }
    }
}
