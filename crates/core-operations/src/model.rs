use core_db::newtype_id;
use core_runtime::RuntimeInstanceId;

newtype_id! {
    /// Identifies the durable operation independently of its owning runtime.
    OperationId
}

impl OperationId {
    /// Reconstructs an operation identity retained by another service.
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}

#[derive(Debug, Clone)]
pub struct OperationRecord {
    pub id: OperationId,
    pub kind: String,
    pub status_kind: String,
    pub thread_id: Option<String>,
    pub runtime_instance_id: RuntimeInstanceId,
    pub outcome_json: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub interrupt_reason: Option<String>,
    pub cancel_requested_at: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    /// Journal sequence of the operation's latest lifecycle event.
    pub seq: i64,
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub interrupted: Vec<OperationId>,
    pub anomalies: Vec<OperationId>,
}
