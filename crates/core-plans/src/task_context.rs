//! The bounded design context of a task in an approved version.
use crate::{PlanId, Plans, PlansError, TaskContent, WorkflowId};
use core_db::Tx;
use core_design::{LayoutPart, OwnedVia};
use core_grants::Grant;
use core_projects::ProjectId;
use serde::{Deserialize, Serialize};

// `contracts`, `part` and `via` are typed as open objects in the schema: their
// own types carry no schema derive.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<crate::Phase>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phase_dependencies: Vec<PhaseDependency>,
    pub plan: ContextPlan,
    pub task: TaskContent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(value_type = Vec<Object>)]
    pub contracts: Vec<crate::ResolvedContract>,
    pub parts: Vec<TaskPart>,
    pub unowned_paths: Vec<String>,
    pub standards: ContextStandards,
    pub map_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PhaseDependency {
    pub reference: crate::PhaseRef,
    pub target: Option<crate::PhaseEndpoint>,
    pub phase: Option<crate::Phase>,
    pub broken: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContextPlan {
    pub plan_id: PlanId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContextStandards {
    pub base: i64,
    pub additions: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskPart {
    #[schema(value_type = Object)]
    pub part: LayoutPart,
    pub paths: Vec<TaskPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskPath {
    pub path: String,
    #[schema(value_type = Object)]
    pub via: OwnedVia,
}

impl Plans {
    /// The context a grant may read: its project's approved task, after the
    /// grant's own checks.
    pub async fn task_context(
        &self,
        grant: &Grant,
        workflow: Option<&WorkflowId>,
        task: u32,
        phase: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        self.store.task_context(grant, workflow, task, phase).await
    }

    /// The one assembly of a task's context, as `tx` sees the approved
    /// version `workflow` of `project` and its layout: what a grant reads and
    /// what an attempt saves inside its own write.
    pub async fn task_context_in(
        tx: &mut Tx,
        project: &ProjectId,
        workflow: &WorkflowId,
        task: u32,
        phase: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        crate::store::PlansStore::task_context_in(tx, project, workflow, task, phase).await
    }
}
