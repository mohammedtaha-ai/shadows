use std::path::PathBuf;

use crate::events::Actor;
use crate::id::newtype_id;
use crate::operation::OperationId;
use crate::plans::{TaskId, WorkflowId};
use crate::project::ProjectId;

newtype_id! {
    /// Spec §4.1.
    ThreadId
}

newtype_id! {
    /// Spec §4.1. Distinct from the entry's `ordinal`, which orders entries
    /// within one thread and is not an identity.
    ThreadEntryId
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PlanningThread {
    pub id: ThreadId,
    pub project_id: ProjectId,
    pub title: String,
    pub status: String,
    pub created_at: String,
    /// The CLI this conversation runs on (`agent::policy`): chosen at
    /// creation, changeable until the thread's first operation, then fixed
    /// (spec §12.6).
    pub harness: String,
    /// The thread this one was forked from, if it is a fork (spec §12.9).
    #[schema(value_type = Option<String>, required)]
    pub forked_from_thread: Option<ThreadId>,
}

/// What a thread's next turn inherits from durable state, read by the
/// Planner's Prepare step (spec §8.3) rather than supplied by the caller: a
/// client that could name a turn's directory could run it anywhere.
#[derive(Debug, Clone)]
pub struct TurnContext {
    /// The owning project's directory. `None` for a project created before
    /// projects owned one — see `Project::directory`.
    pub project_directory: Option<PathBuf>,
    /// The harness session this thread's turns continue, once one of them
    /// has reached its turn-end. `None` means the next turn starts a session.
    pub harness_session_id: Option<String>,
    /// The harness the thread runs on (spec §12.6).
    pub harness: String,
    /// The owning project, whose allowed modes a turn is checked against.
    pub project_id: ProjectId,
    /// For a fork that has not yet recorded a session of its own: the source's
    /// session its first opening forks (spec §12.9).
    pub fork_session_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ThreadEntry {
    pub id: ThreadEntryId,
    pub thread_id: ThreadId,
    pub ordinal: i64,
    pub kind: ThreadEntryKind,
    /// Spec §4.2 calls this field's type `Principal`. Milestone 0 uses
    /// `events::Actor`, which already has exactly this shape (`kind` + `id`) and
    /// already answers "who did this" for durable events. Declaring a second
    /// identical struct would record one decision twice, which this project's
    /// documentation rules forbid. See the note in §4.2.
    pub author: Actor,
    pub body: String,
    pub refs: Vec<EntryRef>,
    pub created_at: String,
    /// The turn this entry belongs to (spec §12.7). `None` for entries written
    /// before entries named their turn. A fork's copied entries keep the
    /// source's operation: provenance, not something the fork can act on.
    #[schema(value_type = Option<String>, required)]
    pub operation_id: Option<OperationId>,
}

/// The fields of an entry being appended. A struct rather than five positional
/// parameters: `append_thread_entry` previously took
/// `(&str, &str, &str, &str, &str)`, five arguments the compiler could not tell
/// apart, and the predecessor project `shadow` died partly on one reversed
/// argument pair that 413 commits did not catch. Naming the fields at the call
/// site is what makes the mistake unwritable rather than merely unlikely.
#[derive(Debug, Clone)]
pub struct NewThreadEntry<'a> {
    pub kind: ThreadEntryKind,
    pub author: Actor,
    pub body: &'a str,
    pub refs: &'a [EntryRef],
    /// The turn that wrote it; `None` only for an entry no turn wrote.
    pub operation_id: Option<&'a OperationId>,
}

/// What an entry is (spec §4.2, closed by §13.9). The client branches on it.
/// Each variant is stored as its name, the text storage held before this was
/// an enum, so no stored row is rewritten.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub enum ThreadEntryKind {
    UserMessage,
    AgentMessage,
    /// A permission the harness asked for and Shadows refused (spec §12.2).
    PermissionRefused,
    /// A plan shown in the conversation at the person's request (§13.9).
    PlanView,
    /// A plan version approved (§13.9).
    PlanApproved,
}

impl ThreadEntryKind {
    /// The stored text, which is also the wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserMessage => "UserMessage",
            Self::AgentMessage => "AgentMessage",
            Self::PermissionRefused => "PermissionRefused",
            Self::PlanView => "PlanView",
            Self::PlanApproved => "PlanApproved",
        }
    }

    /// The inverse of [`Self::as_str`]; `None` for text no variant names.
    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::UserMessage,
            Self::AgentMessage,
            Self::PermissionRefused,
            Self::PlanView,
            Self::PlanApproved,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum EntryRef {
    /// Typed where the referenced entity's module exists. `Decision` and
    /// `Research` reference entities whose modules no milestone has created
    /// yet, and §4.1's rule is that no module is created before the task that
    /// fills it.
    Operation(OperationId),
    Decision(String),
    Research(String),
    Workflow(WorkflowId),
    /// A message about one task of a plan (§13.9).
    Task(TaskId),
}
