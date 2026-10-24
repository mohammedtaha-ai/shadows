use crate::runtime::RuntimeInstanceId;
use core_db::newtype_id;

newtype_id! {
    /// Identifies the durable operation independently of its owning runtime.
    OperationId
}

#[derive(Debug, Clone)]
pub(crate) struct OperationRecord {
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
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub interrupted: Vec<OperationId>,
    pub anomalies: Vec<OperationId>,
}
