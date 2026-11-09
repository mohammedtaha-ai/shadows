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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Attempt {
    pub id: AttemptId,
    pub project_id: ProjectId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub number: u32,
    pub thread_id: ThreadId,
    pub operation_id: Option<OperationId>,
    pub model: String,
    pub state: AttemptState,
    pub commit: Option<String>,
    pub reason: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// A task as the runs see it: marked done or not, and its latest attempt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TaskRun {
    pub task: u32,
    pub done: bool,
    pub done_at: Option<String>,
    pub latest: Option<Attempt>,
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

/// What a commit did: the attempt, now `Committed`, the paths it committed,
/// and the changed paths outside `writes` it left in the folder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommitResult {
    pub attempt: Attempt,
    pub files: Vec<String>,
    pub left: Vec<String>,
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

/// What startup recovery found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoverReport {
    pub committed: u32,
    pub interrupted: u32,
}
