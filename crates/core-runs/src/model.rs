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
/// a done mark, the group of never-attempted tasks that may start now. Empty
/// for a mark taken back or on a version that admits no new work. MCP only;
/// no route serves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DoneMark {
    pub task: TaskRun,
    pub next: Vec<ReadyTask>,
}

/// One task of a ready group: it never ran, is not done, and Start allows it now.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ReadyTask {
    pub task: u32,
    pub title: String,
    #[schema(required)]
    pub suggested_model: Option<String>,
}

/// Tasks of one approved version that may start now, ascending by number, no
/// two of whose writes may meet. An observation, not a reservation: each
/// Start checks admission again.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ReadyGroup {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub tasks: Vec<ReadyTask>,
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

/// One file of an attempt's commit: its line counts, its patch cut at 2,000
/// lines with a marker, and the same patch read into hunks (none for a binary
/// file), whose lines stop at 2,000 in all.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct FileDiff {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub patch: String,
    pub hunks: Vec<shadows_agent::events::DiffHunk>,
}

/// The facts of an executor's notice to its Planner, the card the
/// conversation draws: which task and attempt, on which model, how it ended,
/// and the executor's report (its last message when it failed).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AttemptNotice {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    /// The task's title, or `T{task}` when it has none.
    pub title: String,
    pub attempt: u32,
    pub attempt_id: AttemptId,
    pub model: String,
    pub outcome: AttemptState,
    pub commit: Option<String>,
    pub report: Option<String>,
    pub reason: Option<String>,
    /// The attempt carried the task's third and last addition.
    pub last_addition: bool,
    /// The executor's contract conflict as it stood when the notice was
    /// first resolved; left out of an ordinary notice. A report from the
    /// executor, never the person's approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<TaskConflict>,
}

/// One piece of a conflict's evidence: a project-relative path, an optional
/// one-based line, and what it shows. A reference, not copied source.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    utoipa::ToSchema,
)]
pub struct ConflictEvidence {
    /// A path inside the project, as `project_files` names it.
    pub path: String,
    /// The one-based line it is about, if one.
    #[serde(default)]
    #[schema(required)]
    pub line: Option<u32>,
    /// What this place shows, in a sentence or two.
    pub summary: String,
}

/// What an executor reports when its task cannot be built as its contract
/// says: what the contract or instructions expect, what it observed instead,
/// and where.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct ConflictInput {
    /// The idempotency key: the same key and input answer the first report.
    pub command_id: String,
    /// What the contract or the task's instructions expect, 1 to 2,000
    /// characters.
    pub expected: String,
    /// What you observed instead, 1 to 2,000 characters.
    pub observed: String,
    /// 1 to 10 places that show it, in the order to read them.
    pub evidence: Vec<ConflictEvidence>,
}

/// Whether the Planner's notice of a conflict was enqueued on its
/// conversation. `delivered` is a durable enqueue, not a reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDelivery {
    Pending,
    Delivered,
    NoPlanner,
}

impl ConflictDelivery {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::NoPlanner => "no_planner",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        [Self::Pending, Self::Delivered, Self::NoPlanner]
            .into_iter()
            .find(|delivery| delivery.as_str() == text)
    }
}

/// An attempt's accepted contract conflict, with the attempt's current state
/// and whether it still holds its scope. Accepting it asked the executor to
/// end its turn; only `attempt_state` says whether it did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct TaskConflict {
    pub attempt_id: AttemptId,
    pub project_id: ProjectId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub expected: String,
    pub observed: String,
    pub evidence: Vec<ConflictEvidence>,
    pub attempt_state: AttemptState,
    /// The attempt still holds its scope: its executor may still write.
    pub scope_held: bool,
    pub delivery: ConflictDelivery,
    /// When the conflict was accepted.
    pub created_at: String,
}

/// One file as an attempt's commit holds it; deleted or binary files have no text.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AttemptFile {
    pub path: String,
    pub text: Option<String>,
    pub cut: bool,
}

/// The record a task leaves when it stops on the cap: the chain of attempts
/// from its Start, the commands its executor ran, and the Planner's account
/// of what stopped the work. The two Planner fields are `null` until it
/// writes them. No route serves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskLesson {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub title: String,
    pub attempts: Vec<LessonAttempt>,
    pub commands: Vec<LessonCommand>,
    pub stopped_by: Option<String>,
    pub plan_should_have_said: Option<String>,
    pub recorded_at: String,
}

/// One attempt of the chain, oldest first; `addition` is what was missing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LessonAttempt {
    pub number: u32,
    pub addition: Option<String>,
    pub commit: Option<String>,
    pub report: Option<String>,
}

/// One shell call the executor ran: its title, status and error, no output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LessonCommand {
    pub command: String,
    pub status: String,
    pub error: Option<String>,
}

/// What startup recovery found: how many running attempts it interrupted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoverReport {
    pub interrupted: u32,
}
