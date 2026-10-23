use crate::db::{Actor, Causation, JournalEntry};
use crate::plans::{Place, WorkflowId};
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use crate::turns::OperationId;

/// SQLite assigns sequence numbers while holding the write lock, so journal
/// sequence order equals commit order. Other backends must preserve this guarantee.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EventCursor(pub i64);

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
    /// Typed scope ids prevent an event from appearing in the wrong replay.
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
    #[cfg(feature = "test-support")]
    pub fn with_causation(mut self, kind: impl Into<String>, reference: impl Into<String>) -> Self {
        self.causation = Some(Causation {
            kind: kind.into(),
            reference: reference.into(),
        });
        self
    }
    #[cfg(feature = "test-support")]
    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    pub fn with_payload(mut self, v: serde_json::Value) -> Self {
        self.payload_json = v.to_string();
        self
    }
}

/// The domain-to-column step: this service converts the ids it owns.
impl From<&DurableEvent> for JournalEntry {
    fn from(e: &DurableEvent) -> Self {
        Self {
            event_id: e.event_id.clone(),
            kind: e.kind.clone(),
            project_id: e.project_id.as_ref().map(|i| i.as_str().to_owned()),
            thread_id: e.thread_id.as_ref().map(|i| i.as_str().to_owned()),
            operation_id: e.operation_id.as_ref().map(|i| i.as_str().to_owned()),
            actor: e.actor.clone(),
            causation: e.causation.clone(),
            correlation_id: e.correlation_id.clone(),
            payload_json: e.payload_json.clone(),
        }
    }
}

/// The live-only signal that moves a person's screen.
///
/// `plan_show` sends one after its card commits. It is transport state
/// that is never stored, never journaled, never replayed. `Events` delivers
/// it to every live subscriber of its thread as `Delivery::PlanShow`, which
/// the stream (`sse.rs`) frames as `plan-show`; only the tab it names opens
/// the panel or the page.
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
