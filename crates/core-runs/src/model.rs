//! One job: the attempt types callers meet. No persistence imports.

use core_db::newtype_id;
use core_operations::OperationId;
use core_plans::WorkflowId;
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

/// What startup recovery found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoverReport {
    pub committed: u32,
    pub interrupted: u32,
}
