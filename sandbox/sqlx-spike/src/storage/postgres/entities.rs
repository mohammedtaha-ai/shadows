//! Postgres row structs and domain mapping. Mirrors sqlite entities.

use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, EventId, EventKind, Operation, OperationId,
    OperationKind, OperationOutcome, OperationStatus, Principal, Project, ProjectId,
    ResearchArtifact, ResearchId, Timestamp,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub fn fmt_ts(t: Timestamp) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}
pub fn parse_ts(s: &str) -> Timestamp {
    OffsetDateTime::parse(s, &Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc())
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ProjectRow {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OperationRow {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub thread_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub runtime_instance_id: Uuid,
    pub created_at: OffsetDateTime,
    pub durable_seq: i64,
    pub outcome: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct EventRow {
    pub id: Uuid,
    pub durable_seq: i64,
    pub kind: String,
    pub occurred_at: OffsetDateTime,
    pub operation_id: Option<Uuid>,
    pub payload: serde_json::Value,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct CommandRow {
    pub command_id: Uuid,
    pub principal: Uuid,
    pub scope: String,
    pub operation_id: Uuid,
    pub recorded_at: OffsetDateTime,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ResearchRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub source: Option<String>,
    pub summary: String,
    pub created_at: OffsetDateTime,
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

pub fn project_from_row(r: ProjectRow) -> Project {
    Project {
        id: ProjectId(r.id),
        name: r.name,
        created_at: r.created_at,
    }
}
pub fn project_to_row(p: &Project) -> (Uuid, String, OffsetDateTime) {
    (p.id.0, p.name.clone(), p.created_at)
}

pub fn operation_from_row(r: OperationRow) -> Operation {
    Operation {
        id: OperationId(r.id),
        kind: parse_op_kind(&r.kind),
        status: parse_status(&r.status),
        thread_id: r.thread_id.map(shadows_domain::ThreadId),
        workflow_id: r.workflow_id.map(shadows_domain::WorkflowId),
        task_id: r.task_id.map(shadows_domain::TaskId),
        runtime_instance_id: shadows_domain::RuntimeInstanceId(r.runtime_instance_id),
        created_at: r.created_at,
        durable_seq: r.durable_seq as u64,
        outcome: r.outcome.as_deref().map(parse_outcome),
    }
}
pub fn operation_to_row(op: &Operation) -> OperationRow {
    OperationRow {
        id: op.id.0,
        kind: fmt_op_kind(op.kind).to_string(),
        status: fmt_status(op.status).to_string(),
        thread_id: op.thread_id.map(|t| t.0),
        workflow_id: op.workflow_id.map(|w| w.0),
        task_id: op.task_id.map(|t| t.0),
        runtime_instance_id: op.runtime_instance_id.0,
        created_at: op.created_at,
        durable_seq: op.durable_seq as i64,
        outcome: op.outcome.map(fmt_outcome).map(|s| s.to_string()),
    }
}

pub fn event_from_row(r: EventRow) -> DurableEvent {
    DurableEvent {
        id: EventId(r.id),
        durable_seq: r.durable_seq as u64,
        kind: parse_event_kind(&r.kind),
        occurred_at: r.occurred_at,
        operation_id: r.operation_id.map(OperationId),
        payload: r.payload,
    }
}
pub fn event_to_row(e: &DurableEvent) -> EventRow {
    EventRow {
        id: e.id.0,
        durable_seq: e.durable_seq as i64,
        kind: fmt_event_kind(e.kind).to_string(),
        occurred_at: e.occurred_at,
        operation_id: e.operation_id.map(|o| o.0),
        payload: e.payload.clone(),
    }
}

pub fn command_from_row(r: CommandRow) -> CommandRecord {
    CommandRecord {
        command_id: CommandId(r.command_id),
        principal: Principal(r.principal),
        scope: r.scope,
        operation_id: OperationId(r.operation_id),
        recorded_at: r.recorded_at,
    }
}
pub fn command_to_row(c: &CommandRecord) -> CommandRow {
    CommandRow {
        command_id: c.command_id.0,
        principal: c.principal.0,
        scope: c.scope.clone(),
        operation_id: c.operation_id.0,
        recorded_at: c.recorded_at,
    }
}

pub fn research_from_row(r: ResearchRow) -> ResearchArtifact {
    ResearchArtifact {
        id: ResearchId(r.id),
        project_id: ProjectId(r.project_id),
        title: r.title,
        source: r.source,
        summary: r.summary,
        created_at: r.created_at,
    }
}
pub fn research_to_row(r: &ResearchArtifact) -> ResearchRow {
    ResearchRow {
        id: r.id.0,
        project_id: r.project_id.0,
        title: r.title.clone(),
        source: r.source.clone(),
        summary: r.summary.clone(),
        created_at: r.created_at,
    }
}
