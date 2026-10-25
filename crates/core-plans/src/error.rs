//! Plan lifecycle failures and refusals.
use super::{PlanId, Problem, WorkflowId};
#[derive(Debug, thiserror::Error)]
pub enum PlansError {
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
    #[error("{0}")]
    InvalidCommand(String),
    #[error("{0}")]
    GrantScope(String),
    #[error("the grant is unknown or revoked")]
    GrantInvalid,
    #[error(transparent)]
    Design(core_design::DesignError),
    #[error(transparent)]
    Projects(core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
fn problems(list: &[Problem]) -> String {
    list.iter()
        .map(|p| p.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

impl From<sqlx::Error> for PlansError {
    fn from(error: sqlx::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}
impl From<serde_json::Error> for PlansError {
    fn from(error: serde_json::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}
impl From<core_grants::GrantsError> for PlansError {
    fn from(error: core_grants::GrantsError) -> Self {
        match error {
            core_grants::GrantsError::GrantInvalid => Self::GrantInvalid,
            core_grants::GrantsError::GrantScope => {
                Self::GrantScope("outside what the grant allows".into())
            }
            core_grants::GrantsError::GrantScopeMessage(message) => Self::GrantScope(message),
            core_grants::GrantsError::Storage(error) => Self::Storage(error),
        }
    }
}
impl From<core_projects::ProjectsError> for PlansError {
    fn from(error: core_projects::ProjectsError) -> Self {
        match error {
            core_projects::ProjectsError::Storage(error) => Self::Storage(error),
            other => Self::Projects(other),
        }
    }
}
impl From<core_code::CodeError> for PlansError {
    fn from(error: core_code::CodeError) -> Self {
        match error {
            core_code::CodeError::InvalidCommand(message) => Self::InvalidCommand(message),
            core_code::CodeError::GrantScope(message) => Self::GrantScope(message),
            core_code::CodeError::Storage(error) => Self::Storage(error),
            core_code::CodeError::Projects(error) => error.into(),
        }
    }
}
impl From<core_design::DesignError> for PlansError {
    fn from(error: core_design::DesignError) -> Self {
        match error {
            core_design::DesignError::Storage(error) => Self::Storage(error),
            core_design::DesignError::Grants(error) => error.into(),
            core_design::DesignError::InvalidCommand(message) => Self::InvalidCommand(message),
            core_design::DesignError::GrantScope(message) => Self::GrantScope(message),
            core_design::DesignError::ReasonMissing => Self::ReasonMissing,
            other => Self::Design(other),
        }
    }
}
impl PlansError {
    pub(crate) fn invalid_constraint(error: impl Into<Self>) -> Self {
        match error.into() {
            Self::Storage(core_db::StorageError::Constraint(message)) => {
                Self::InvalidCommand(message)
            }
            other => other,
        }
    }
}
