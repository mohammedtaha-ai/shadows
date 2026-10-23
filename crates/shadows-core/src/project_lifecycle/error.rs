//! Project removal refusals use the shared application vocabulary.
use crate::{CoreError, ErrorCode};

#[derive(Debug, thiserror::Error)]
pub(super) enum ProjectLifecycleError {
    #[error("the project has planning threads; it cannot be removed")]
    ProjectHasThreads,
}

impl From<ProjectLifecycleError> for CoreError {
    fn from(error: ProjectLifecycleError) -> Self {
        Self::Refused {
            code: ErrorCode::ProjectHasThreads,
            message: error.to_string(),
        }
    }
}
