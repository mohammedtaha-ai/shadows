//! Plan lifecycle refusals, converted to the application error vocabulary.
use super::{PlanId, Problem, WorkflowId};
use crate::{CoreError, ErrorCode};
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlansError {
    #[error("the plan version is frozen; start a new version to change it")]
    WorkflowFrozen,
    #[error("the plan is not valid: {}", problems(.0))]
    PlanInvalid(Vec<Problem>),
    #[error("{0}")]
    PlanLinkInvalid(String),
    #[error("{0}")]
    TaskNotInPlan(String),
    #[error("the source is not the thread's latest version, {0}")]
    NotLatestVersion(WorkflowId),
    #[error("a new version needs its reason")]
    ReasonMissing,
    #[error("plan {0} is archived; a person can unarchive it")]
    PlanArchived(PlanId),
}
fn problems(list: &[Problem]) -> String {
    list.iter()
        .map(|p| p.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

impl From<PlansError> for CoreError {
    fn from(error: PlansError) -> Self {
        let message = error.to_string();
        match error {
            PlansError::PlanInvalid(problems) => Self::ValidationFailed {
                message,
                problems: problems.into_iter().map(|p| p.message).collect(),
            },
            PlansError::ReasonMissing => Self::ReasonMissing,
            PlansError::NotLatestVersion(_) => Self::Unavailable(message),
            PlansError::WorkflowFrozen => Self::Refused {
                code: ErrorCode::WorkflowFrozenImmutable,
                message,
            },
            _ => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
        }
    }
}
