//! One job: the planning thread types callers meet.

use std::path::PathBuf;

use core_db::Actor;
use core_db::newtype_id;
use core_projects::ProjectId;

newtype_id! {
    /// A planning thread has its own identity.
    ThreadId
}

newtype_id! {
    /// Distinct from the entry's `ordinal`, which orders entries
    /// within one thread and is not an identity.
    ThreadEntryId
}

/// The initial title source determines which later title writers may replace it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreatedTitle {
    /// The name the client gave; the first message and the harness replace it.
    /// A stored `plan` title remains protected from these automatic writers.
    Client,
}

impl CreatedTitle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PlanningThread {
    #[schema(value_type = Option<String>, required)]
    pub profile: Option<String>,
    pub id: ThreadId,
    pub project_id: ProjectId,
    pub title: String,
    pub status: String,
    pub created_at: String,
    /// Set when this conversation was removed; its history remains readable.
    #[schema(value_type = Option<String>, required)]
    pub removed_at: Option<String>,
    /// The CLI this conversation runs on (`agent::policy`): chosen at
    /// creation, changeable until the thread's first operation, then fixed
    /// (spec §12.6).
    pub harness: String,
    /// The thread this one was forked from, if it is a fork (spec §12.9).
    #[schema(value_type = Option<String>, required)]
    pub forked_from_thread: Option<ThreadId>,
}

/// What a thread's next turn inherits from durable state, read by the
/// Planner's Prepare step rather than supplied by the caller: a
/// client that could name a turn's directory could run it anywhere.
#[derive(Debug, Clone)]
pub struct TurnContext {
    pub profile: Option<String>,
    /// The owning project's directory; `None` when no directory is recorded.
    pub project_directory: Option<PathBuf>,
    /// The harness session this thread's turns continue, once one of them
    /// has reached its turn-end. `None` means the next turn starts a session.
    pub harness_session_id: Option<String>,
    /// The harness the thread runs on.
    pub harness: String,
    /// The owning project, whose allowed modes a turn is checked against.
    pub project_id: ProjectId,
    /// For a fork that has not yet recorded a session of its own: the source's
    /// session its first opening forks.
    pub fork_session_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ThreadEntry {
    pub attachments: Vec<super::AttachmentRef>,
    pub id: ThreadEntryId,
    pub thread_id: ThreadId,
    pub ordinal: i64,
    pub kind: ThreadEntryKind,
    /// Spec §4.2 calls this field's type `Principal`. Milestone 0 uses
    /// `db::Actor`, which already has exactly this shape (`kind` + `id`) and
    /// already answers "who did this" for durable events. Declaring a second
    /// identical struct would record one decision twice, which this project's
    /// documentation rules forbid. See the note in §4.2.
    pub author: Actor,
    pub body: String,
    pub refs: Vec<EntryRef>,
    /// A card's structured payload (§23.8): a `Subagent`'s card. `None` for
    /// every other kind.
    #[schema(
        value_type = Option<std::collections::HashMap<String, serde_json::Value>>,
        required
    )]
    pub card: Option<serde_json::Value>,
    pub created_at: String,
    /// The turn this entry belongs to (spec §12.7). `None` for entries written
    /// before entries named their turn. A fork's copied entries keep the
    /// source's operation: provenance, not something the fork can act on.
    #[schema(value_type = Option<String>, required)]
    pub operation_id: Option<String>,
}

/// Named entry fields prevent swapping values that share the same Rust type.
#[derive(Debug, Clone)]
pub struct NewThreadEntry<'a> {
    pub kind: ThreadEntryKind,
    pub author: Actor,
    pub body: &'a str,
    pub refs: &'a [EntryRef],
    /// Structured payload for a subagent or a recorded proposal; absent for plain entries.
    pub card: Option<&'a serde_json::Value>,
    /// The turn that wrote it; `None` only for an entry no turn wrote.
    pub operation_id: Option<&'a str>,
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
    /// A tool the harness ran; its body is the tool's title (§23.8).
    ToolCall,
    /// A subagent's card (§22.2); its body is the card's title and `card` the
    /// card itself (§23.8).
    Subagent,
    /// A permission the harness asked for and Shadows refused (spec §12.2).
    PermissionRefused,
    /// A plan shown in the conversation at the person's request (§13.9).
    PlanView,
    /// A plan version approved (§13.9).
    PlanApproved,
    /// A vision proposal's immutable card (§23.5).
    VisionProposal,
    MapProposal,
    /// A roadmap proposal's immutable card (§23.5).
    RoadmapProposal,
}

impl ThreadEntryKind {
    /// The stored text, which is also the wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserMessage => "UserMessage",
            Self::AgentMessage => "AgentMessage",
            Self::ToolCall => "ToolCall",
            Self::Subagent => "Subagent",
            Self::PermissionRefused => "PermissionRefused",
            Self::PlanView => "PlanView",
            Self::PlanApproved => "PlanApproved",
            Self::VisionProposal => "VisionProposal",
            Self::MapProposal => "MapProposal",
            Self::RoadmapProposal => "RoadmapProposal",
        }
    }

    /// The inverse of [`Self::as_str`]; `None` for text no variant names.
    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::UserMessage,
            Self::AgentMessage,
            Self::ToolCall,
            Self::Subagent,
            Self::PermissionRefused,
            Self::PlanView,
            Self::PlanApproved,
            Self::VisionProposal,
            Self::MapProposal,
            Self::RoadmapProposal,
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
    Operation(OperationRef),
    Decision(String),
    Research(String),
    Workflow(WorkflowRef),
    /// A message about one task of a plan (§13.9).
    Task(TaskRef),
}

// Provenance is opaque here: only the referenced entity's owner interprets it.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(as = OperationId, value_type = String, format = Uuid)]
pub struct OperationRef(String);

impl From<&str> for OperationRef {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(as = WorkflowId, value_type = String, format = Uuid)]
pub struct WorkflowRef(String);

impl From<&str> for WorkflowRef {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(as = TaskId, value_type = String, format = Uuid)]
pub struct TaskRef(String);

impl From<&str> for TaskRef {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl ThreadId {
    /// Reconstructs this owner's identifier from a retained database reference.
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}

impl ThreadEntryId {
    /// Reconstructs this owner's identifier from a retained database reference.
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}
