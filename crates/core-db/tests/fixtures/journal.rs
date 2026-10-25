use core_db::{Actor, Causation, JournalEntry};

pub(super) struct Event(JournalEntry);

impl Event {
    pub(super) fn new(kind: &str, actor: Actor) -> Self {
        Self(JournalEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            project_id: None,
            thread_id: None,
            operation_id: None,
            actor,
            causation: None,
            correlation_id: None,
            payload_json: "{}".into(),
        })
    }
    pub(super) fn with_project(mut self, id: &str) -> Self {
        self.0.project_id = Some(id.into());
        self
    }
    pub(super) fn with_payload(mut self, value: serde_json::Value) -> Self {
        self.0.payload_json = value.to_string();
        self
    }
    pub(super) fn with_causation(mut self, kind: &str, reference: &str) -> Self {
        self.0.causation = Some(Causation {
            kind: kind.into(),
            reference: reference.into(),
        });
        self
    }
    pub(super) fn with_correlation(mut self, id: &str) -> Self {
        self.0.correlation_id = Some(id.into());
        self
    }
}

impl From<&Event> for JournalEntry {
    fn from(event: &Event) -> Self {
        event.0.clone()
    }
}
