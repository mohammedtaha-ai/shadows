use serde::{Deserialize, Serialize};

use super::ids::{OperationId, RuntimeInstanceId, TaskId, ThreadId, Timestamp, WorkflowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    PlannerTurn,
    ExecutionRun,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationOutcome {
    Success,
    Blocked,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub thread_id: Option<ThreadId>,
    pub workflow_id: Option<WorkflowId>,
    pub task_id: Option<TaskId>,
    pub runtime_instance_id: RuntimeInstanceId,
    pub created_at: Timestamp,
    /// Monotonic, explicit ordering key — the canonical ordering for events/operations.
    /// Never use rowid or physical insertion order for this.
    pub durable_seq: u64,
    pub outcome: Option<OperationOutcome>,
}
