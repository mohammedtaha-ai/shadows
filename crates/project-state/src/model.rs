//! One job: the project's state as its readers receive it.

use code_record::{BuildState, ElementState};
use core_design::{DesignElement, ElementKind, PartId, Stage};
use core_plans::{PlanId, WorkflowId};
use core_projects::ProjectId;
use core_runs::AttemptState;
use serde::Serialize;
use utoipa::ToSchema;

/// The project's stage, its parts with their plans and elements, and its gaps.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct ProjectStateView {
    pub project_id: ProjectId,
    pub stage: Stage,
    /// What the stage still misses, as Design's stage view names it.
    pub missing: Vec<String>,
    pub map_revision: i64,
    /// In map order, at most 200.
    pub parts: Vec<PartState>,
    pub parts_more: u32,
    /// Active plans tied to no part, at most 20.
    pub unassigned: Vec<PlanProgress>,
    pub unassigned_more: u32,
    /// Elements in gap or unchecked, by part order then element number, at most 100.
    pub gaps: Vec<ElementGap>,
    pub gaps_more: u32,
    /// One part in full, only when asked with its id.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub part: Option<PartDetail>,
}

/// One part of the map with its plans and its elements counted by state.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PartState {
    pub part_id: PartId,
    #[schema(required)]
    pub parent: Option<PartId>,
    pub title: String,
    #[schema(required)]
    pub kind: Option<String>,
    /// At most 20.
    pub plans: Vec<PlanProgress>,
    pub plans_more: u32,
    /// Removed elements are not counted.
    pub elements: StateCounts,
}

/// An Active plan with its live version's task counts.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PlanProgress {
    pub plan_id: PlanId,
    pub title: String,
    /// The live version: the plan's highest Frozen version; none with only a Draft.
    #[schema(required)]
    pub workflow_id: Option<WorkflowId>,
    #[schema(required)]
    pub version: Option<i64>,
    /// The plan's Draft, named only: it adds nothing to the state.
    #[schema(required)]
    pub draft: Option<WorkflowId>,
    pub tasks: TaskCounts,
}

/// A live version's tasks: done, started (an attempt and not done), not started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct TaskCounts {
    pub total: u32,
    pub done: u32,
    pub started: u32,
    pub not_started: u32,
}

/// A part's elements by state; removed ones are left out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct StateCounts {
    pub planned: u32,
    pub built: u32,
    pub gap: u32,
    pub unchecked: u32,
}

/// An element in gap or unchecked, with why.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct ElementGap {
    pub element_id: String,
    pub kind: ElementKind,
    pub name: String,
    pub part_id: PartId,
    pub state: BuildState,
    pub why: String,
}

/// One part in full: its live plans' tasks and its elements.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PartDetail {
    pub part_id: PartId,
    /// At most 50.
    pub plans: Vec<PlanTasks>,
    pub plans_more: u32,
    pub elements: Vec<ElementView>,
}

/// A plan's live version with each task's line.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PlanTasks {
    pub plan_id: PlanId,
    pub title: String,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub tasks: Vec<TaskLine>,
}

/// One task: whether it is marked done, and its latest attempt's state.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct TaskLine {
    pub number: u32,
    pub title: String,
    pub done: bool,
    #[schema(required)]
    pub attempt: Option<AttemptState>,
}

/// An element at its latest revision with its state.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct ElementView {
    pub element: DesignElement,
    pub state: ElementState,
}
