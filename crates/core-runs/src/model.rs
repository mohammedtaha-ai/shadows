//! One job: the attempt types callers meet. No persistence imports.

use core_db::newtype_id;
use core_operations::OperationId;
use core_plans::{TaskParent, WorkflowId};
use core_projects::ProjectId;
use core_threads::ThreadId;

newtype_id! {
    /// One executor session working one task.
    AttemptId
}

impl AttemptId {
    /// Reconstructs this owner's identifier from a retained database reference.
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}

/// Where an attempt stands. Only `Running` has no `ended_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AttemptState {
    Running,
    Committed,
    Stopped,
    Interrupted,
    Failed,
}

impl AttemptState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Committed => "committed",
            Self::Stopped => "stopped",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        [
            Self::Running,
            Self::Committed,
            Self::Stopped,
            Self::Interrupted,
            Self::Failed,
        ]
        .into_iter()
        .find(|state| state.as_str() == text)
    }
}

/// One try at a task. `operation_id` is the operation of its turn, absent
/// until the turn starts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct Attempt {
    pub id: AttemptId,
    pub project_id: ProjectId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    /// 1 for the task's first attempt, then one more each Start.
    pub number: u32,
    /// The executor's thread; its entries are the attempt's run.
    pub thread_id: ThreadId,
    #[schema(required)]
    pub operation_id: Option<OperationId>,
    pub model: String,
    pub state: AttemptState,
    /// The SHA of the commit the executor made and reported, once accepted.
    #[schema(required)]
    pub commit: Option<String>,
    /// The executor's own account of what it did, ran and found, stored
    /// with its accepted commit.
    #[schema(required)]
    pub report: Option<String>,
    /// The Planner's addition this attempt carried on with, in the session of
    /// the attempt before it; null for an attempt Start began.
    #[schema(required)]
    pub addition: Option<String>,
    /// Why the attempt failed, when it did.
    #[schema(required)]
    pub reason: Option<String>,
    pub started_at: String,
    #[schema(required)]
    pub ended_at: Option<String>,
}

/// A task as the runs see it: marked done or not, its latest attempt, the
/// tasks it needs, whether Start may run now, and the model to offer first.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct TaskRun {
    pub task: u32,
    pub done: bool,
    #[schema(required)]
    pub done_at: Option<String>,
    #[schema(required)]
    pub latest: Option<Attempt>,
    /// In link order.
    pub needs: Vec<Need>,
    pub start: StartAnswer,
    /// The model the plan's Planner conversation last asked for; `null`
    /// without one.
    #[schema(required)]
    pub suggested_model: Option<String>,
}

/// What the Planner's done mark answers: the task as it now stands, and, for
/// a done mark, the lowest-numbered task that never ran and may start now.
/// MCP only; no route serves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DoneMark {
    pub task: TaskRun,
    pub next: Option<u32>,
}

/// Whether Start may run now, by the same checks as Start, without writing.
/// `allowed` true means `code` and `reason` are `null`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct StartAnswer {
    pub allowed: bool,
    /// The refusal's opaque id, the same a refused Start answers: `waiting`,
    /// `task_busy`, `overlap`, `not_git`, `dirty_scope`, `not_frozen`,
    /// `superseded`, `archived` or `bad_scope`.
    #[schema(required)]
    pub code: Option<String>,
    /// The text to show beside Start.
    #[schema(required)]
    pub reason: Option<String>,
}

/// A task this one waits on through a `needs` link: a task of the same
/// version, or of another plan's latest version.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct Need {
    /// A number for a task of this version; `{ plan_id, task }` for another plan's.
    pub task: TaskParent,
    pub title: String,
    /// Marked done by the person in the version the link reaches.
    pub done: bool,
}

/// One file of an attempt's commit: its line counts and its patch, cut at
/// 2,000 lines with a marker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct FileDiff {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub patch: String,
}

/// What startup recovery found: how many running attempts it interrupted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoverReport {
    pub interrupted: u32,
}
