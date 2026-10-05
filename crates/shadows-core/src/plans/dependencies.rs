//! Task-parent identities across plan versions (§16.7).

use std::fmt;

use super::model::{AcceptanceItem, Link, PlanId, PlanState, WorkflowId, WorkflowState};
use crate::projects::ProjectId;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TaskPreview {
    pub number: u32,
    pub title: String,
    pub goal: String,
    pub acceptance: Vec<AcceptanceItem>,
    pub state: String,
}

/// The other end of an outgoing or incoming cross-plan dependency.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LinkedTask {
    pub link: Link,
    pub incoming: bool,
    pub plan_id: PlanId,
    pub project_id: Option<ProjectId>,
    pub project_name: Option<String>,
    pub workflow_id: Option<WorkflowId>,
    pub version: Option<i64>,
    pub plan_title: Option<String>,
    pub plan_state: Option<PlanState>,
    pub state: Option<WorkflowState>,
    pub task: Option<TaskPreview>,
    pub broken: Option<String>,
}

/// A local task number, or a task in another plan's latest version.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(untagged)]
pub enum TaskParent {
    Local(u32),
    Plan {
        #[schemars(with = "String")]
        plan_id: PlanId,
        task: u32,
    },
}

impl TaskParent {
    pub fn local(&self) -> Option<u32> {
        match self {
            Self::Local(number) => Some(*number),
            Self::Plan { .. } => None,
        }
    }

    pub fn number(&self) -> u32 {
        match self {
            Self::Local(number) => *number,
            Self::Plan { task, .. } => *task,
        }
    }
}

impl From<u32> for TaskParent {
    fn from(number: u32) -> Self {
        Self::Local(number)
    }
}

impl fmt::Display for TaskParent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(number) => write!(formatter, "T{number}"),
            Self::Plan { plan_id, task } => write!(formatter, "plan {plan_id} T{task}"),
        }
    }
}
