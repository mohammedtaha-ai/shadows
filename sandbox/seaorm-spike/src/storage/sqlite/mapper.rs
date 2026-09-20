//! Domain ↔ SeaORM row mapping. The mapping itself stays storage-side.

use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, EventId, EventKind, Operation, OperationId,
    OperationKind, OperationOutcome, OperationStatus, Principal, Project, ProjectId, ResearchArtifact,
    ResearchId, RuntimeInstanceId, TaskId, ThreadId, Timestamp, WorkflowId,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::entities::{
    CommandModel, EventModel, OperationModel, ResearchModel,
};
// Bring generated entities into scope as `project::Model` etc.
use super::entities as ent;

fn parse_ts(s: &str) -> Timestamp {
    OffsetDateTime::parse(s, &Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn fmt_ts(t: Timestamp) -> String {
    t.format(&Rfc3339).unwrap_or_else(|_| String::new())
}

pub fn project_to_active(p: &Project) -> ent::project::ActiveModel {
    ent::project::ActiveModel {
        id: sea_orm::Set(p.id.0),
        name: sea_orm::Set(p.name.clone()),
        created_at: sea_orm::Set(fmt_ts(p.created_at)),
    }
}

pub fn project_from_model(m: ent::project::Model) -> Project {
    Project {
        id: ProjectId(m.id),
        name: m.name,
        created_at: parse_ts(&m.created_at),
    }
}

fn parse_op_kind(s: &str) -> OperationKind {
    match s {
        "PlannerTurn" => OperationKind::PlannerTurn,
        "ExecutionRun" => OperationKind::ExecutionRun,
        "Verification" => OperationKind::Verification,
        _ => OperationKind::ExecutionRun,
    }
}

fn fmt_op_kind(k: OperationKind) -> &'static str {
    match k {
        OperationKind::PlannerTurn => "PlannerTurn",
        OperationKind::ExecutionRun => "ExecutionRun",
        OperationKind::Verification => "Verification",
    }
}

fn parse_status(s: &str) -> OperationStatus {
    match s {
        "Pending" => OperationStatus::Pending,
        "Running" => OperationStatus::Running,
        "Completed" => OperationStatus::Completed,
        "Failed" => OperationStatus::Failed,
        "Cancelled" => OperationStatus::Cancelled,
        _ => OperationStatus::Pending,
    }
}

fn fmt_status(s: OperationStatus) -> &'static str {
    match s {
        OperationStatus::Pending => "Pending",
        OperationStatus::Running => "Running",
        OperationStatus::Completed => "Completed",
        OperationStatus::Failed => "Failed",
        OperationStatus::Cancelled => "Cancelled",
    }
}

fn parse_outcome(s: &str) -> OperationOutcome {
    match s {
        "Success" => OperationOutcome::Success,
        "Blocked" => OperationOutcome::Blocked,
        "Rejected" => OperationOutcome::Rejected,
        _ => OperationOutcome::Success,
    }
}

fn fmt_outcome(o: OperationOutcome) -> &'static str {
    match o {
        OperationOutcome::Success => "Success",
        OperationOutcome::Blocked => "Blocked",
        OperationOutcome::Rejected => "Rejected",
    }
}

pub fn operation_to_active(op: &Operation) -> ent::operation::ActiveModel {
    ent::operation::ActiveModel {
        id: sea_orm::Set(op.id.0),
        kind: sea_orm::Set(fmt_op_kind(op.kind).to_string()),
        status: sea_orm::Set(fmt_status(op.status).to_string()),
        thread_id: sea_orm::Set(op.thread_id.map(|t| t.0)),
        workflow_id: sea_orm::Set(op.workflow_id.map(|w| w.0)),
        task_id: sea_orm::Set(op.task_id.map(|t| t.0)),
        runtime_instance_id: sea_orm::Set(op.runtime_instance_id.0),
        created_at: sea_orm::Set(fmt_ts(op.created_at)),
        durable_seq: sea_orm::Set(op.durable_seq as i64),
        outcome: sea_orm::Set(op.outcome.map(fmt_outcome).map(|s| s.to_string())),
    }
}

pub fn operation_from_model(m: OperationModel) -> Operation {
    Operation {
        id: OperationId(m.id),
        kind: parse_op_kind(&m.kind),
        status: parse_status(&m.status),
        thread_id: m.thread_id.map(ThreadId),
        workflow_id: m.workflow_id.map(WorkflowId),
        task_id: m.task_id.map(TaskId),
        runtime_instance_id: RuntimeInstanceId(m.runtime_instance_id),
        created_at: parse_ts(&m.created_at),
        durable_seq: m.durable_seq as u64,
        outcome: m.outcome.as_deref().map(parse_outcome),
    }
}

fn parse_event_kind(s: &str) -> EventKind {
    match s {
        "OperationStarted" => EventKind::OperationStarted,
        "OperationCompleted" => EventKind::OperationCompleted,
        "OperationFailed" => EventKind::OperationFailed,
        "CommandRecorded" => EventKind::CommandRecorded,
        "ResearchCaptured" => EventKind::ResearchCaptured,
        _ => EventKind::Other,
    }
}

fn fmt_event_kind(k: EventKind) -> &'static str {
    match k {
        EventKind::OperationStarted => "OperationStarted",
        EventKind::OperationCompleted => "OperationCompleted",
        EventKind::OperationFailed => "OperationFailed",
        EventKind::CommandRecorded => "CommandRecorded",
        EventKind::ResearchCaptured => "ResearchCaptured",
        EventKind::Other => "Other",
    }
}

pub fn event_to_active(e: &DurableEvent) -> ent::durable_event::ActiveModel {
    ent::durable_event::ActiveModel {
        id: sea_orm::Set(e.id.0),
        durable_seq: sea_orm::Set(e.durable_seq as i64),
        kind: sea_orm::Set(fmt_event_kind(e.kind).to_string()),
        occurred_at: sea_orm::Set(fmt_ts(e.occurred_at)),
        operation_id: sea_orm::Set(e.operation_id.map(|o| o.0)),
        payload: sea_orm::Set(serde_json::to_string(&e.payload).unwrap_or_default()),
    }
}

pub fn event_from_model(m: EventModel) -> DurableEvent {
    DurableEvent {
        id: EventId(m.id),
        durable_seq: m.durable_seq as u64,
        kind: parse_event_kind(&m.kind),
        occurred_at: parse_ts(&m.occurred_at),
        operation_id: m.operation_id.map(OperationId),
        payload: serde_json::from_str(&m.payload).unwrap_or(serde_json::Value::Null),
    }
}

pub fn command_to_active(c: &CommandRecord) -> ent::command_record::ActiveModel {
    ent::command_record::ActiveModel {
        command_id: sea_orm::Set(c.command_id.0),
        principal: sea_orm::Set(c.principal.0),
        scope: sea_orm::Set(c.scope.clone()),
        operation_id: sea_orm::Set(c.operation_id.0),
        recorded_at: sea_orm::Set(fmt_ts(c.recorded_at)),
    }
}

pub fn command_from_model(m: CommandModel) -> CommandRecord {
    CommandRecord {
        command_id: CommandId(m.command_id),
        principal: Principal(m.principal),
        scope: m.scope,
        operation_id: OperationId(m.operation_id),
        recorded_at: parse_ts(&m.recorded_at),
    }
}

pub fn research_to_active(r: &ResearchArtifact) -> ent::research_artifact::ActiveModel {
    ent::research_artifact::ActiveModel {
        id: sea_orm::Set(r.id.0),
        project_id: sea_orm::Set(r.project_id.0),
        title: sea_orm::Set(r.title.clone()),
        source: sea_orm::Set(r.source.clone()),
        summary: sea_orm::Set(r.summary.clone()),
        created_at: sea_orm::Set(fmt_ts(r.created_at)),
    }
}

pub fn research_from_model(m: ResearchModel) -> ResearchArtifact {
    ResearchArtifact {
        id: ResearchId(m.id),
        project_id: ProjectId(m.project_id),
        title: m.title,
        source: m.source,
        summary: m.summary,
        created_at: parse_ts(&m.created_at),
    }
}
