use shadows_domain::{
    CommandId, DurableEvent, EventId, EventKind, Operation, OperationId, OperationKind,
    OperationOutcome, OperationStatus, Principal, RuntimeInstanceId, TaskId, ThreadId, Timestamp,
    WorkflowId,
};

#[derive(Debug, Clone)]
pub struct AtomicFixture {
    pub command_id: CommandId,
    pub principal: Principal,
    pub scope: String,
    pub operation: Operation,
    pub event: DurableEvent,
    pub request_payload: serde_json::Value,
}

impl AtomicFixture {
    pub fn new(scope: impl Into<String>) -> Self {
        let operation_id = OperationId::new();
        Self {
            command_id: CommandId::new(),
            principal: Principal::new(),
            scope: scope.into(),
            operation: Operation {
                id: operation_id,
                kind: OperationKind::ExecutionRun,
                status: OperationStatus::Running,
                thread_id: Some(ThreadId::new()),
                workflow_id: Some(WorkflowId::new()),
                task_id: None::<TaskId>,
                runtime_instance_id: RuntimeInstanceId::new(),
                created_at: Timestamp::now_utc(),
                durable_seq: 0,
                outcome: Some(OperationOutcome::Success),
            },
            event: DurableEvent {
                id: EventId::new(),
                durable_seq: 0,
                kind: EventKind::OperationStarted,
                occurred_at: Timestamp::now_utc(),
                operation_id: Some(operation_id),
                payload: serde_json::json!({"source": "delta"}),
            },
            request_payload: serde_json::json!({"task_id": null, "mode": "delta"}),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomicResult {
    pub operation_id: OperationId,
    pub durable_seq: u64,
    pub inserted: bool,
}

pub(crate) fn operation_kind(value: OperationKind) -> &'static str {
    match value {
        OperationKind::PlannerTurn => "PlannerTurn",
        OperationKind::ExecutionRun => "ExecutionRun",
        OperationKind::Verification => "Verification",
    }
}

pub(crate) fn parse_operation_kind(value: &str) -> OperationKind {
    match value {
        "PlannerTurn" => OperationKind::PlannerTurn,
        "Verification" => OperationKind::Verification,
        _ => OperationKind::ExecutionRun,
    }
}

pub(crate) fn operation_status(value: OperationStatus) -> &'static str {
    match value {
        OperationStatus::Pending => "Pending",
        OperationStatus::Running => "Running",
        OperationStatus::Completed => "Completed",
        OperationStatus::Failed => "Failed",
        OperationStatus::Cancelled => "Cancelled",
    }
}

pub(crate) fn parse_operation_status(value: &str) -> OperationStatus {
    match value {
        "Pending" => OperationStatus::Pending,
        "Completed" => OperationStatus::Completed,
        "Failed" => OperationStatus::Failed,
        "Cancelled" => OperationStatus::Cancelled,
        _ => OperationStatus::Running,
    }
}

pub(crate) fn event_kind(value: EventKind) -> &'static str {
    match value {
        EventKind::OperationStarted => "OperationStarted",
        EventKind::OperationCompleted => "OperationCompleted",
        EventKind::OperationFailed => "OperationFailed",
        EventKind::CommandRecorded => "CommandRecorded",
        EventKind::ResearchCaptured => "ResearchCaptured",
        EventKind::Other => "Other",
    }
}

pub(crate) fn parse_event_kind(value: &str) -> EventKind {
    match value {
        "OperationStarted" => EventKind::OperationStarted,
        "OperationCompleted" => EventKind::OperationCompleted,
        "OperationFailed" => EventKind::OperationFailed,
        "CommandRecorded" => EventKind::CommandRecorded,
        "ResearchCaptured" => EventKind::ResearchCaptured,
        _ => EventKind::Other,
    }
}

#[cfg(feature = "seaorm-current")]
pub mod seaorm_current;

#[cfg(feature = "sqlx-current")]
pub mod sqlx_current;
