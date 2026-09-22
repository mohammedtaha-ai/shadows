use crate::id::newtype_id;
use crate::runtime::RuntimeInstanceId;

newtype_id! {
    /// Spec §4.1. This and `RuntimeInstanceId` are the pair that first sat
    /// adjacent in one call — `mark_operation_started(op_id, expected_runtime)` —
    /// where two `String`s compiled cleanly when swapped.
    OperationId
}

/// Spec §8.3: Prepare failure ("we could not get ready") and Spawn failure
/// ("the OS refused to start it") are kept distinct in the durable record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// Resolving the harness, building the environment, readying the
    /// workspace. The process never existed. Spec §8.3.
    Prepare,
    /// The OS refused to start the process. Spec §2.7.
    Spawn,
    /// The process ran and failed.
    Run,
}

impl FailureStage {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureStage::Prepare => "Prepare",
            FailureStage::Spawn => "Spawn",
            FailureStage::Run => "Run",
        }
    }
}

/// Spec §2.7, §6.14. `thread_id` stays a plain `String` here on purpose: the
/// `ProjectId`/`ThreadId`/`ThreadEntryId` sweep is a separate change, staged
/// for reviewability, that follows this task (spec §4.1 OPEN block).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Operation {
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
