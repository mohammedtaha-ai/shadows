//! One job: the plan types callers meet (spec §13).
//!
//! Pure: no storage, no I/O. A plan is changed only by
//! [`apply`](super::ops::apply)ing a batch of [`PlanOp`](super::ops::PlanOp)s,
//! and judged only by [`edit_problems`](super::rules::edit_problems) and
//! [`approval_problems`](super::rules::approval_problems),
//! so every writer — the Planner's MCP tools, the HTTP routes — and every reader
//! that lists what blocks approval share one set of rules (§13.4).

use std::collections::BTreeMap;

use super::rules::Problem;
use crate::id::newtype_id;
use crate::project::ProjectId;
use crate::thread::ThreadId;

newtype_id! {
    /// Spec §13.2. One version of a plan.
    WorkflowId
}

newtype_id! {
    /// Spec §13.3. Storage identity only: tools and people name a task by its
    /// `number` within the version, never by this id.
    TaskId
}

/// Spec §13.2. A `Draft` is edited; a `Frozen` version is approved and never
/// changes again. The web client shows `Frozen` as "Approved".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub enum WorkflowState {
    Draft,
    Frozen,
}

/// Spec §13.3. How a link's `task` waits for its `after`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// The task does not start until `after` is complete.
    Needs,
    /// The task may finish now, but the acceptance items the link names only
    /// hold once `after` is complete.
    CompletesAfter,
}

impl LinkKind {
    /// The wire name, as serde writes it.
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::Needs => "needs",
            LinkKind::CompletesAfter => "completes_after",
        }
    }
}

/// How a refusal names one link: `the needs link T2 → T1`, the arrow pointing
/// from the task that waits to the task it waits for.
pub(super) fn link_name(task: u32, after: u32, kind: LinkKind) -> String {
    format!("the {} link T{task} → T{after}", kind.as_str())
}

/// Spec §13.3. One sentence someone can check, numbered within its task.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct AcceptanceItem {
    pub number: u32,
    pub text: String,
}

/// Spec §13.3. A task as the plan holds it, shown as `T{number}`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct TaskContent {
    pub number: u32,
    pub title: String,
    pub goal: String,
    /// Paths the task reads (its declared scope).
    pub reads: Vec<String>,
    /// Paths the task may change (its declared scope).
    pub writes: Vec<String>,
    pub acceptance: Vec<AcceptanceItem>,
}

/// Spec §13.3. `task` waits for `after`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct Link {
    pub task: u32,
    pub after: u32,
    pub kind: LinkKind,
    /// A few words saying what passes from `after` to `task`.
    pub label: String,
    /// The acceptance items of `task` that wait. Empty for `needs`.
    #[serde(default)]
    pub waiting_items: Vec<u32>,
}

/// One plan version's content. Tasks are keyed by their number; links keep
/// the order they were stored in, which is the order problems are reported in.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PlanContent {
    pub title: String,
    pub goal: String,
    pub tasks: BTreeMap<u32, TaskContent>,
    pub links: Vec<Link>,
}

/// One task of a stored version: its storage id beside its content.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PlanTask {
    pub id: TaskId,
    #[serde(flatten)]
    pub content: TaskContent,
}

/// The edit that set a version's current revision (§13.10's plan read).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LastEdit {
    pub revision: i64,
    pub summary: String,
    pub changed_tasks: Vec<u32>,
}

/// One stored version of a plan, as a reader sees it now (§13.2).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Plan {
    pub id: WorkflowId,
    pub thread_id: ThreadId,
    pub project_id: ProjectId,
    pub version: i64,
    pub revision: i64,
    pub state: WorkflowState,
    pub title: String,
    pub goal: String,
    /// The version this one was copied from.
    pub previous: Option<WorkflowId>,
    /// The version copied from this one, once it exists.
    pub next: Option<WorkflowId>,
    pub tasks: Vec<PlanTask>,
    pub links: Vec<Link>,
    /// What blocks approval ([`approval_problems`]) for a `Draft`; empty for
    /// a `Frozen` version.
    pub blockers: Vec<Problem>,
    /// `None` until the version is first edited.
    pub last_edit: Option<LastEdit>,
    pub frozen_at: Option<String>,
    pub created_at: String,
}

impl Plan {
    /// The content [`apply`](super::ops::apply) and the checks work on.
    pub fn content(&self) -> PlanContent {
        PlanContent {
            title: self.title.clone(),
            goal: self.goal.clone(),
            tasks: self
                .tasks
                .iter()
                .map(|t| (t.content.number, t.content.clone()))
                .collect(),
            links: self.links.clone(),
        }
    }
}

/// What an edit did, fixed when it committed, so a replay answers exactly
/// that and not the plan as it is later (§13.5).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct EditOutcome {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub revision: i64,
    pub summary: String,
    pub changed_tasks: Vec<u32>,
}

/// What `draft_start` answered: the version, which never changes as the draft
/// is edited (§13.6).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DraftStarted {
    pub workflow_id: WorkflowId,
    pub thread_id: ThreadId,
    pub version: i64,
}

/// What an approval did, fixed when it committed (§13.2, §13.5).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Approved {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub revision: i64,
    pub frozen_at: String,
}

/// A plan as a project's list shows it: its thread's latest version.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PlanListing {
    pub id: WorkflowId,
    pub thread_id: ThreadId,
    pub title: String,
    pub version: i64,
    pub state: WorkflowState,
    pub updated_at: String,
}
