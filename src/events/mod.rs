/// Spec §6.18. `seq` is assigned by the INSERT, which on SQLite can only run
/// while holding the write lock, so assignment order equals commit order. That
/// property is SQLite-specific — see the OPEN block in §6.18 before writing
/// backend-neutral cursor code.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EventCursor(pub i64);

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct DurableEvent {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<String>,
    pub thread_id: Option<String>,
    pub operation_id: Option<String>,
    pub actor: Actor,
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
            correlation_id: None,
            payload_json: "{}".into(),
        }
    }
    pub fn with_project(mut self, id: impl Into<String>) -> Self {
        self.project_id = Some(id.into());
        self
    }
    pub fn with_thread(mut self, id: impl Into<String>) -> Self {
        self.thread_id = Some(id.into());
        self
    }
    pub fn with_operation(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }
    pub fn with_payload(mut self, v: serde_json::Value) -> Self {
        self.payload_json = v.to_string();
        self
    }
}
