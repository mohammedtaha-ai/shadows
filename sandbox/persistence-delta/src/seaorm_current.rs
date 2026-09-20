use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr, QueryResult, Statement,
    TransactionTrait,
};
use sea_orm_migration::prelude::{MigrationName, MigrationTrait, MigratorTrait, SchemaManager};
use shadows_domain::{
    DurableEvent, EventId, Operation, OperationId, OperationOutcome, RuntimeInstanceId, TaskId,
    ThreadId, WorkflowId,
};

use crate::{
    AtomicFixture, AtomicResult, event_kind, operation_kind, operation_status, parse_event_kind,
    parse_operation_kind, parse_operation_status,
};

const RESET_SQL: &str = r#"
DROP TABLE IF EXISTS command_record, durable_event, operation, durable_cursor,
    seaql_migrations, _sqlx_migrations CASCADE
"#;

const V1_SQL: &str = r#"
CREATE TABLE durable_cursor (
    scope TEXT PRIMARY KEY,
    next_seq BIGINT NOT NULL
);
CREATE TABLE operation (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    thread_id UUID,
    workflow_id UUID,
    task_id UUID,
    runtime_instance_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    durable_seq BIGINT NOT NULL,
    outcome JSONB
);
CREATE TABLE durable_event (
    id UUID PRIMARY KEY,
    scope TEXT NOT NULL,
    durable_seq BIGINT NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    operation_id UUID,
    payload JSONB NOT NULL,
    UNIQUE (scope, durable_seq)
);
CREATE TABLE command_record (
    principal UUID NOT NULL,
    scope TEXT NOT NULL,
    command_id UUID NOT NULL,
    operation_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (principal, scope, command_id)
)
"#;

const V2_SQL: &str = r#"
ALTER TABLE command_record
ADD COLUMN request_payload JSONB NOT NULL DEFAULT '{}'::jsonb
"#;

pub struct SeaOrmStore {
    db: DatabaseConnection,
}

impl SeaOrmStore {
    pub async fn reset(url: &str) -> Result<(), DbErr> {
        let db = Database::connect(url).await?;
        db.execute_unprepared(RESET_SQL).await?;
        Ok(())
    }

    pub async fn reset_and_migrate(url: &str) -> Result<Self, DbErr> {
        Self::reset(url).await?;
        Self::migrate_latest(url).await?;
        let db = Database::connect(url).await?;
        Ok(Self { db })
    }

    pub async fn migrate_v1(url: &str) -> Result<(), DbErr> {
        let db = Database::connect(url).await?;
        Migrator::up(&db, Some(1)).await
    }

    pub async fn migrate_latest(url: &str) -> Result<(), DbErr> {
        let db = Database::connect(url).await?;
        Migrator::up(&db, None).await
    }

    pub async fn seed_v1_fixture(url: &str) -> Result<(), DbErr> {
        let db = Database::connect(url).await?;
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO command_record \
             (principal, scope, command_id, operation_id, recorded_at) \
             VALUES ($1, 'legacy', $2, $3, now())",
            [
                uuid::Uuid::new_v4().into(),
                uuid::Uuid::new_v4().into(),
                uuid::Uuid::new_v4().into(),
            ],
        ))
        .await?;
        Ok(())
    }

    pub async fn v1_fixture_survived(url: &str) -> Result<bool, DbErr> {
        let db = Database::connect(url).await?;
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS n FROM command_record WHERE scope = 'legacy' \
                 AND request_payload = '{}'::jsonb",
            ))
            .await?
            .expect("count row");
        Ok(row.try_get::<i64>("", "n")? == 1)
    }

    pub async fn migration_count(url: &str) -> Result<i64, DbErr> {
        let db = Database::connect(url).await?;
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS n FROM seaql_migrations",
            ))
            .await?
            .expect("count row");
        row.try_get("", "n")
    }

    pub async fn atomic_command(&self, fixture: AtomicFixture) -> Result<AtomicResult, DbErr> {
        self.atomic_command_inner(fixture, false).await
    }

    pub async fn atomic_command_then_fail(
        &self,
        fixture: AtomicFixture,
    ) -> Result<AtomicResult, DbErr> {
        self.atomic_command_inner(fixture, true).await
    }

    async fn atomic_command_inner(
        &self,
        fixture: AtomicFixture,
        fail_before_commit: bool,
    ) -> Result<AtomicResult, DbErr> {
        let tx = self.db.begin().await?;
        let existing = tx
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT operation_id FROM command_record \
                 WHERE principal = $1 AND scope = $2 AND command_id = $3",
                [
                    fixture.principal.0.into(),
                    fixture.scope.clone().into(),
                    fixture.command_id.0.into(),
                ],
            ))
            .await?;
        if let Some(row) = existing {
            let operation_id: uuid::Uuid = row.try_get("", "operation_id")?;
            let seq_row = tx
                .query_one_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT durable_seq FROM operation WHERE id = $1",
                    [operation_id.into()],
                ))
                .await?
                .expect("operation for command");
            let durable_seq: i64 = seq_row.try_get("", "durable_seq")?;
            tx.commit().await?;
            return Ok(AtomicResult {
                operation_id: OperationId(operation_id),
                durable_seq: durable_seq as u64,
                inserted: false,
            });
        }

        let seq_row = tx
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO durable_cursor(scope, next_seq) VALUES ($1, 1) \
                 ON CONFLICT(scope) DO UPDATE SET next_seq = durable_cursor.next_seq + 1 \
                 RETURNING next_seq",
                [fixture.scope.clone().into()],
            ))
            .await?
            .expect("sequence row");
        let durable_seq: i64 = seq_row.try_get("", "next_seq")?;
        let op = &fixture.operation;
        let outcome = op
            .outcome
            .map(|value| serde_json::to_value(value).expect("serializable outcome"));
        tx.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO operation \
             (id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
              created_at, durable_seq, outcome) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            [
                op.id.0.into(),
                operation_kind(op.kind).into(),
                operation_status(op.status).into(),
                op.thread_id.map(|v| v.0).into(),
                op.workflow_id.map(|v| v.0).into(),
                op.task_id.map(|v| v.0).into(),
                op.runtime_instance_id.0.into(),
                op.created_at.into(),
                durable_seq.into(),
                outcome.into(),
            ],
        ))
        .await?;
        let event = &fixture.event;
        tx.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO durable_event \
             (id, scope, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
            [
                event.id.0.into(),
                fixture.scope.clone().into(),
                durable_seq.into(),
                event_kind(event.kind).into(),
                event.occurred_at.into(),
                event.operation_id.map(|v| v.0).into(),
                event.payload.clone().into(),
            ],
        ))
        .await?;
        tx.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO command_record \
             (principal, scope, command_id, operation_id, recorded_at, request_payload) \
             VALUES ($1,$2,$3,$4,now(),$5)",
            [
                fixture.principal.0.into(),
                fixture.scope.into(),
                fixture.command_id.0.into(),
                op.id.0.into(),
                fixture.request_payload.into(),
            ],
        ))
        .await?;

        if fail_before_commit {
            tx.rollback().await?;
            return Err(DbErr::Custom("intentional rollback probe".into()));
        }
        tx.commit().await?;
        Ok(AtomicResult {
            operation_id: op.id,
            durable_seq: durable_seq as u64,
            inserted: true,
        })
    }

    pub async fn events_after(&self, scope: &str, seq: u64) -> Result<Vec<DurableEvent>, DbErr> {
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id, durable_seq, kind, occurred_at, operation_id, payload \
                 FROM durable_event WHERE scope = $1 AND durable_seq > $2 \
                 ORDER BY durable_seq ASC",
                [scope.into(), (seq as i64).into()],
            ))
            .await?;
        rows.into_iter().map(event_from_row).collect()
    }

    pub async fn operation(&self, id: OperationId) -> Result<Option<Operation>, DbErr> {
        self.db
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
                 created_at, durable_seq, outcome FROM operation WHERE id = $1",
                [id.0.into()],
            ))
            .await?
            .map(operation_from_row)
            .transpose()
    }
}

fn event_from_row(row: QueryResult) -> Result<DurableEvent, DbErr> {
    let kind: String = row.try_get("", "kind")?;
    let operation_id: Option<uuid::Uuid> = row.try_get("", "operation_id")?;
    Ok(DurableEvent {
        id: EventId(row.try_get("", "id")?),
        durable_seq: row.try_get::<i64>("", "durable_seq")? as u64,
        kind: parse_event_kind(&kind),
        occurred_at: row.try_get("", "occurred_at")?,
        operation_id: operation_id.map(OperationId),
        payload: row.try_get("", "payload")?,
    })
}

fn operation_from_row(row: QueryResult) -> Result<Operation, DbErr> {
    let kind: String = row.try_get("", "kind")?;
    let status: String = row.try_get("", "status")?;
    let thread_id: Option<uuid::Uuid> = row.try_get("", "thread_id")?;
    let workflow_id: Option<uuid::Uuid> = row.try_get("", "workflow_id")?;
    let task_id: Option<uuid::Uuid> = row.try_get("", "task_id")?;
    let outcome: Option<serde_json::Value> = row.try_get("", "outcome")?;
    Ok(Operation {
        id: OperationId(row.try_get("", "id")?),
        kind: parse_operation_kind(&kind),
        status: parse_operation_status(&status),
        thread_id: thread_id.map(ThreadId),
        workflow_id: workflow_id.map(WorkflowId),
        task_id: task_id.map(TaskId),
        runtime_instance_id: RuntimeInstanceId(row.try_get("", "runtime_instance_id")?),
        created_at: row.try_get("", "created_at")?,
        durable_seq: row.try_get::<i64>("", "durable_seq")? as u64,
        outcome: outcome
            .map(serde_json::from_value::<OperationOutcome>)
            .transpose()
            .map_err(|error| DbErr::Type(error.to_string()))?,
    })
}

struct V1;

impl MigrationName for V1 {
    fn name(&self) -> &str {
        "m20260920_000001_delta_v1"
    }
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for V1 {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(V1_SQL).await?;
        Ok(())
    }
}

struct V2;

impl MigrationName for V2 {
    fn name(&self) -> &str {
        "m20260920_000002_delta_payload"
    }
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for V2 {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(V2_SQL).await?;
        Ok(())
    }
}

struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(V1), Box::new(V2)]
    }
}
