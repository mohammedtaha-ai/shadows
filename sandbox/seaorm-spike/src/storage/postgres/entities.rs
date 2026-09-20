//! SeaORM Postgres row structs and domain mapping.

use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, EventId, EventKind, Operation, OperationId,
    OperationKind, OperationOutcome, OperationStatus, Principal, Project, ProjectId,
    ResearchArtifact, ResearchId, Timestamp,
};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(sea_orm::FromQueryResult, Debug, Clone)]
pub struct ProjectRow {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
}

#[derive(sea_orm::FromQueryResult, Debug, Clone)]
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

#[derive(sea_orm::FromQueryResult, Debug, Clone)]
pub struct EventRow {
    pub id: Uuid,
    pub durable_seq: i64,
    pub kind: String,
    pub occurred_at: OffsetDateTime,
    pub operation_id: Option<Uuid>,
    pub payload: serde_json::Value,
}

#[derive(sea_orm::FromQueryResult, Debug, Clone)]
pub struct CommandRow {
    pub command_id: Uuid,
    pub principal: Uuid,
    pub scope: String,
    pub operation_id: Uuid,
    pub recorded_at: OffsetDateTime,
}

#[derive(sea_orm::FromQueryResult, Debug, Clone)]
pub struct ResearchRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub source: Option<String>,
    pub summary: String,
    pub created_at: OffsetDateTime,
}

// Mappers are the same logic as the SQLite spike — values flow through
// sea_orm::QueryRow extraction. For brevity the mappers are inlined in
// queries.rs and mod.rs in this prototype.
