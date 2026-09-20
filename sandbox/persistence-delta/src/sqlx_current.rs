use shadows_domain::{
    DurableEvent, EventId, Operation, OperationId, OperationOutcome, RuntimeInstanceId, TaskId,
    ThreadId, WorkflowId,
};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions};

use crate::{
    AtomicFixture, AtomicResult, event_kind, operation_kind, operation_status, parse_event_kind,
    parse_operation_kind, parse_operation_status,
};

static V1_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/sqlx-v1");
static LATEST_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/sqlx");

const RESET_SQL: &str = r#"
DROP TABLE IF EXISTS command_record, durable_event, operation, durable_cursor,
    seaql_migrations, _sqlx_migrations CASCADE
"#;

pub struct SqlxStore {
    pool: PgPool,
}

impl SqlxStore {
    async fn pool(url: &str) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new().max_connections(8).connect(url).await
    }

    pub async fn reset(url: &str) -> Result<(), sqlx::Error> {
        let pool = Self::pool(url).await?;
        sqlx::raw_sql(RESET_SQL).execute(&pool).await?;
        pool.close().await;
        Ok(())
    }

    pub async fn reset_and_migrate(url: &str) -> Result<Self, sqlx::Error> {
        Self::reset(url).await?;
        Self::migrate_latest(url).await?;
        Ok(Self {
            pool: Self::pool(url).await?,
        })
    }

    pub async fn migrate_v1(url: &str) -> Result<(), sqlx::Error> {
        let pool = Self::pool(url).await?;
        V1_MIGRATOR.run(&pool).await?;
        pool.close().await;
        Ok(())
    }

    pub async fn migrate_latest(url: &str) -> Result<(), sqlx::Error> {
        let pool = Self::pool(url).await?;
        LATEST_MIGRATOR.run(&pool).await?;
        pool.close().await;
        Ok(())
    }

    pub async fn seed_v1_fixture(url: &str) -> Result<(), sqlx::Error> {
        let pool = Self::pool(url).await?;
        sqlx::query(
            "INSERT INTO command_record \
             (principal, scope, command_id, operation_id, recorded_at) \
             VALUES ($1, 'legacy', $2, $3, now())",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await?;
        pool.close().await;
        Ok(())
    }

    pub async fn v1_fixture_survived(url: &str) -> Result<bool, sqlx::Error> {
        let pool = Self::pool(url).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM command_record WHERE scope = 'legacy' \
             AND request_payload = '{}'::jsonb",
        )
        .fetch_one(&pool)
        .await?;
        pool.close().await;
        Ok(count == 1)
    }

    pub async fn migration_count(url: &str) -> Result<i64, sqlx::Error> {
        let pool = Self::pool(url).await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await?;
        pool.close().await;
        Ok(count)
    }

    pub async fn atomic_command(
        &self,
        fixture: AtomicFixture,
    ) -> Result<AtomicResult, sqlx::Error> {
        self.atomic_command_inner(fixture, false).await
    }

    pub async fn atomic_command_then_fail(
        &self,
        fixture: AtomicFixture,
    ) -> Result<AtomicResult, sqlx::Error> {
        self.atomic_command_inner(fixture, true).await
    }

    async fn atomic_command_inner(
        &self,
        fixture: AtomicFixture,
        fail_before_commit: bool,
    ) -> Result<AtomicResult, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        if let Some((operation_id, durable_seq)) = sqlx::query_as::<_, (uuid::Uuid, i64)>(
            "SELECT c.operation_id, o.durable_seq \
             FROM command_record c JOIN operation o ON o.id = c.operation_id \
             WHERE c.principal = $1 AND c.scope = $2 AND c.command_id = $3",
        )
        .bind(fixture.principal.0)
        .bind(&fixture.scope)
        .bind(fixture.command_id.0)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(AtomicResult {
                operation_id: OperationId(operation_id),
                durable_seq: durable_seq as u64,
                inserted: false,
            });
        }

        let durable_seq: i64 = sqlx::query_scalar(
            "INSERT INTO durable_cursor(scope, next_seq) VALUES ($1, 1) \
             ON CONFLICT(scope) DO UPDATE SET next_seq = durable_cursor.next_seq + 1 \
             RETURNING next_seq",
        )
        .bind(&fixture.scope)
        .fetch_one(&mut *tx)
        .await?;
        insert_operation(&mut tx, &fixture.operation, durable_seq).await?;
        insert_event(&mut tx, &fixture, durable_seq).await?;
        sqlx::query(
            "INSERT INTO command_record \
             (principal, scope, command_id, operation_id, recorded_at, request_payload) \
             VALUES ($1,$2,$3,$4,now(),$5)",
        )
        .bind(fixture.principal.0)
        .bind(&fixture.scope)
        .bind(fixture.command_id.0)
        .bind(fixture.operation.id.0)
        .bind(&fixture.request_payload)
        .execute(&mut *tx)
        .await?;

        if fail_before_commit {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("intentional rollback probe".into()));
        }
        tx.commit().await?;
        Ok(AtomicResult {
            operation_id: fixture.operation.id,
            durable_seq: durable_seq as u64,
            inserted: true,
        })
    }

    pub async fn events_after(
        &self,
        scope: &str,
        seq: u64,
    ) -> Result<Vec<DurableEvent>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, durable_seq, kind, occurred_at, operation_id, payload \
             FROM durable_event WHERE scope = $1 AND durable_seq > $2 \
             ORDER BY durable_seq ASC",
        )
        .bind(scope)
        .bind(seq as i64)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(event_from_row).collect()
    }

    pub async fn operation(&self, id: OperationId) -> Result<Option<Operation>, sqlx::Error> {
        sqlx::query(
            "SELECT id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
             created_at, durable_seq, outcome FROM operation WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(operation_from_row)
        .transpose()
    }
}

async fn insert_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation: &Operation,
    durable_seq: i64,
) -> Result<(), sqlx::Error> {
    let outcome = operation
        .outcome
        .map(|value| serde_json::to_value(value).expect("serializable outcome"));
    sqlx::query(
        "INSERT INTO operation \
         (id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
          created_at, durable_seq, outcome) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(operation.id.0)
    .bind(operation_kind(operation.kind))
    .bind(operation_status(operation.status))
    .bind(operation.thread_id.map(|v| v.0))
    .bind(operation.workflow_id.map(|v| v.0))
    .bind(operation.task_id.map(|v| v.0))
    .bind(operation.runtime_instance_id.0)
    .bind(operation.created_at)
    .bind(durable_seq)
    .bind(outcome)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    fixture: &AtomicFixture,
    durable_seq: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO durable_event \
         (id, scope, durable_seq, kind, occurred_at, operation_id, payload) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(fixture.event.id.0)
    .bind(&fixture.scope)
    .bind(durable_seq)
    .bind(event_kind(fixture.event.kind))
    .bind(fixture.event.occurred_at)
    .bind(fixture.event.operation_id.map(|v| v.0))
    .bind(&fixture.event.payload)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn event_from_row(row: &sqlx::postgres::PgRow) -> Result<DurableEvent, sqlx::Error> {
    let kind: String = row.try_get("kind")?;
    let operation_id: Option<uuid::Uuid> = row.try_get("operation_id")?;
    Ok(DurableEvent {
        id: EventId(row.try_get("id")?),
        durable_seq: row.try_get::<i64, _>("durable_seq")? as u64,
        kind: parse_event_kind(&kind),
        occurred_at: row.try_get("occurred_at")?,
        operation_id: operation_id.map(OperationId),
        payload: row.try_get("payload")?,
    })
}

fn operation_from_row(row: &sqlx::postgres::PgRow) -> Result<Operation, sqlx::Error> {
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    let thread_id: Option<uuid::Uuid> = row.try_get("thread_id")?;
    let workflow_id: Option<uuid::Uuid> = row.try_get("workflow_id")?;
    let task_id: Option<uuid::Uuid> = row.try_get("task_id")?;
    let outcome: Option<serde_json::Value> = row.try_get("outcome")?;
    Ok(Operation {
        id: OperationId(row.try_get("id")?),
        kind: parse_operation_kind(&kind),
        status: parse_operation_status(&status),
        thread_id: thread_id.map(ThreadId),
        workflow_id: workflow_id.map(WorkflowId),
        task_id: task_id.map(TaskId),
        runtime_instance_id: RuntimeInstanceId(row.try_get("runtime_instance_id")?),
        created_at: row.try_get("created_at")?,
        durable_seq: row.try_get::<i64, _>("durable_seq")? as u64,
        outcome: outcome
            .map(serde_json::from_value::<OperationOutcome>)
            .transpose()
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?,
    })
}
