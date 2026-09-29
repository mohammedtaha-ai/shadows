use crate::plans::{Place, WorkflowId};
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use crate::turns::OperationId;

/// Spec §6.18. `seq` is assigned by the INSERT, which on SQLite can only run
/// while holding the write lock, so assignment order equals commit order. That
/// property is SQLite-specific — see the OPEN block in §6.18 before writing
/// backend-neutral cursor code.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EventCursor(pub i64);

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Actor {
    pub kind: String,
    pub id: String,
}

impl Actor {
    pub fn system() -> Self {
        Self {
            kind: "System".into(),
            id: "daemon".into(),
        }
    }
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            kind: "User".into(),
            id: id.into(),
        }
    }
}

/// `durable_event` carries `causation_kind` and `causation_ref` as a pair:
/// its own `CHECK ((causation_kind IS NULL) = (causation_ref IS NULL))`
/// requires both or neither. Modeling them as one `Option<Causation>` here
/// rather than two independent `Option<String>` fields makes the invalid
/// state (kind without reference, or reference without kind) unrepresentable
/// in Rust, instead of merely unrepresentable in the schema.
#[derive(Debug, Clone)]
pub struct Causation {
    pub kind: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct DurableEvent {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<ProjectId>,
    pub thread_id: Option<ThreadId>,
    pub operation_id: Option<OperationId>,
    pub actor: Actor,
    pub causation: Option<Causation>,
    pub correlation_id: Option<String>,
    pub payload_json: String,
}

impl DurableEvent {
    pub fn new(kind: impl Into<String>, actor: Actor) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            project_id: None,
            thread_id: None,
            operation_id: None,
            actor,
            causation: None,
            correlation_id: None,
            payload_json: "{}".into(),
        }
    }
    /// Typed, and that is the point: these three builders took
    /// `impl Into<String>`, so `.with_project(thread_id)` compiled and silently
    /// scoped an event to the wrong entity. One event row is visible through
    /// several scopes (§4.2), so a misscoped event is not a cosmetic error — it
    /// is a row that appears in the wrong replay and is missing from the right
    /// one, with nothing failing anywhere.
    pub fn with_project(mut self, id: &ProjectId) -> Self {
        self.project_id = Some(id.clone());
        self
    }
    pub fn with_thread(mut self, id: &ThreadId) -> Self {
        self.thread_id = Some(id.clone());
        self
    }
    pub fn with_operation(mut self, id: &OperationId) -> Self {
        self.operation_id = Some(id.clone());
        self
    }
    pub fn with_causation(mut self, kind: impl Into<String>, reference: impl Into<String>) -> Self {
        self.causation = Some(Causation {
            kind: kind.into(),
            reference: reference.into(),
        });
        self
    }
    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    pub fn with_payload(mut self, v: serde_json::Value) -> Self {
        self.payload_json = v.to_string();
        self
    }
}

/// One job: the live-only signal that moves a person's screen (spec §13.9).
///
/// `plan_show` sends one after its card commits. It is transport state
/// (§2.10): never stored, never journaled, never replayed. The thread's
/// stream (`sse.rs`) turns it into a `plan-show` frame for every live
/// subscriber; only the tab it names opens the panel or the page.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UiSignal {
    pub thread_id: ThreadId,
    /// The tab that sent the turn, as it sent it; `None` when it named none.
    pub target_tab: Option<String>,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task_number: Option<u32>,
    pub place: Place,
}
