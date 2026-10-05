//! Values in a project's latest-plan dependency map (§16.8).

use super::{PlanId, PlanState, WorkflowId, WorkflowState};
use crate::projects::ProjectId;

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PlanMap {
    pub project_id: ProjectId,
    pub plans: Vec<MapPlan>,
    pub links: Vec<MapLink>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MapPlan {
    pub plan_id: PlanId,
    pub project_id: ProjectId,
    pub project_name: String,
    pub plan_state: PlanState,
    pub workflow_id: Option<WorkflowId>,
    pub title: String,
    pub goal: String,
    pub version: Option<i64>,
    pub state: Option<WorkflowState>,
    pub task_count: u32,
    pub removed: bool,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MapLink {
    pub plan_id: PlanId,
    pub after: PlanId,
    pub count: u32,
    pub broken: bool,
}
