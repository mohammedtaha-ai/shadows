use serde::{Deserialize, Serialize};

use super::ids::{EventId, OperationId, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    OperationStarted,
    OperationCompleted,
    OperationFailed,
    CommandRecorded,
    ResearchCaptured,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableEvent {
    pub id: EventId,
    pub durable_seq: u64,
    pub kind: EventKind,
    pub occurred_at: Timestamp,
    pub operation_id: Option<OperationId>,
    /// JSON payload — only for genuinely tree-shaped data (e.g. tool args).
    /// Avoid for scalar fields.
    pub payload: serde_json::Value,
}
