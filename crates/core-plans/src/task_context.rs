//! The bounded design context of a task in an approved version.
use crate::{PlanId, Plans, PlansError, TaskContent, WorkflowId};
use core_design::{LayoutPart, OwnedVia};
use core_grants::Grant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub plan: ContextPlan,
    pub task: TaskContent,
    pub parts: Vec<TaskPart>,
    pub unowned_paths: Vec<String>,
    pub standards: ContextStandards,
    pub map_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPlan {
    pub plan_id: PlanId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStandards {
    pub base: i64,
    pub additions: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPart {
    pub part: LayoutPart,
    pub paths: Vec<TaskPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPath {
    pub path: String,
    pub via: OwnedVia,
}

impl Plans {
    pub async fn task_context(
        &self,
        grant: &Grant,
        workflow: Option<&WorkflowId>,
        task: u32,
    ) -> Result<TaskContext, PlansError> {
        self.store.task_context(grant, workflow, task).await
    }
}
