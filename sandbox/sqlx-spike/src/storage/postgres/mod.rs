//! Postgres adapter. Mirrors sqlite structure.

use async_trait::async_trait;
use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, Operation, OperationId, OperationStatus, Principal,
    Project, ProjectId, ResearchArtifact, ResearchId, SearchHit, SearchQuery,
};
use sqlx::{PgPool, Row};

use crate::ports::{
    AtomicCommandTx, Backend, DomainError, EventStore, MultiEntityTx, MultiOp, OperationStore,
    ProjectStore, ResearchStore, SearchIndex,
};

pub mod entities;
pub mod migrations;
pub mod queries;

pub use entities as mapper;

pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub async fn connect(url: &str) -> Result<Self, DomainError> {
        let pool = sqlx::PgPool::connect(url)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let s = Self { pool };
        migrations::run_migrations(&s.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(s)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn next_seq_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<u64, DomainError> {
        let row = sqlx::query("UPDATE durable_seq_counter SET n = n + 1 RETURNING n")
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let n: i64 = row
            .try_get("n")
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(n as u64)
    }
}

impl Backend for PostgresBackend {
    fn dialect(&self) -> &'static str {
        "postgres"
    }
}

#[async_trait]
impl AtomicCommandTx for PostgresBackend {
    async fn run<F, Fut>(
        &self,
        command_id: CommandId,
        principal: Principal,
        scope: String,
        build: F,
    ) -> Result<CommandRecord, DomainError>
    where
        F: FnOnce(OperationId) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(Operation, DurableEvent), DomainError>> + Send,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        let existing: Option<mapper::CommandRow> = sqlx::query_as(
            "SELECT command_id, principal, scope, operation_id, recorded_at \
             FROM command_record WHERE command_id = $1",
        )
        .bind(command_id.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        if let Some(row) = existing {
            tx.commit()
                .await
                .map_err(|e| DomainError::Storage(e.to_string()))?;
            return Ok(mapper::command_from_row(row));
        }

        let op_id = OperationId::new();
        let seq = Self::next_seq_tx(&mut tx).await?;
        let (mut op, mut evt) = build(op_id).await?;
        op.durable_seq = seq;
        evt.durable_seq = seq;

        let op_row = mapper::operation_to_row(&op);
        sqlx::query(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(op_row.id)
        .bind(&op_row.kind)
        .bind(&op_row.status)
        .bind(op_row.thread_id)
        .bind(op_row.workflow_id)
        .bind(op_row.task_id)
        .bind(op_row.runtime_instance_id)
        .bind(op_row.created_at)
        .bind(op_row.durable_seq)
        .bind(&op_row.outcome)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;

        let evt_row = mapper::event_to_row(&evt);
        sqlx::query(
            "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(evt_row.id)
        .bind(evt_row.durable_seq)
        .bind(&evt_row.kind)
        .bind(evt_row.occurred_at)
        .bind(evt_row.operation_id)
        .bind(&evt_row.payload)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;

        let cmd = CommandRecord {
            command_id,
            principal,
            scope: scope.clone(),
            operation_id: op.id,
            recorded_at: shadows_domain::now_utc(),
        };
        let cr = mapper::command_to_row(&cmd);
        sqlx::query(
            "INSERT INTO command_record (command_id, principal, scope, operation_id, recorded_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(cr.command_id)
        .bind(cr.principal)
        .bind(&cr.scope)
        .bind(cr.operation_id)
        .bind(cr.recorded_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        Ok(cmd)
    }
}

#[async_trait]
impl EventStore for PostgresBackend {
    async fn append(&self, evt: DurableEvent) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let n: i64 = sqlx::query("UPDATE durable_seq_counter SET n = n + 1 RETURNING n")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?
            .try_get("n")
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let mut evt = evt;
        evt.durable_seq = n as u64;
        let r = mapper::event_to_row(&evt);
        sqlx::query(
            "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(r.id)
        .bind(r.durable_seq)
        .bind(&r.kind)
        .bind(r.occurred_at)
        .bind(r.operation_id)
        .bind(&r.payload)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn read_after(&self, seq: u64, limit: u32) -> Result<Vec<DurableEvent>, DomainError> {
        let rows: Vec<mapper::EventRow> = sqlx::query_as(
            "SELECT id, durable_seq, kind, occurred_at, operation_id, payload \
             FROM durable_event WHERE durable_seq > $1 \
             ORDER BY durable_seq ASC LIMIT $2",
        )
        .bind(seq as i64)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(rows.into_iter().map(mapper::event_from_row).collect())
    }
}

#[async_trait]
impl OperationStore for PostgresBackend {
    async fn insert(&self, op: Operation) -> Result<(), DomainError> {
        let r = mapper::operation_to_row(&op);
        sqlx::query(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(r.id)
        .bind(&r.kind)
        .bind(&r.status)
        .bind(r.thread_id)
        .bind(r.workflow_id)
        .bind(r.task_id)
        .bind(r.runtime_instance_id)
        .bind(r.created_at)
        .bind(r.durable_seq)
        .bind(&r.outcome)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: OperationId) -> Result<Option<Operation>, DomainError> {
        let row: Option<mapper::OperationRow> = sqlx::query_as(
            "SELECT id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
             created_at, durable_seq, outcome FROM operation WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(row.map(mapper::operation_from_row))
    }
    async fn update_status(
        &self,
        id: OperationId,
        status: OperationStatus,
    ) -> Result<(), DomainError> {
        let s = match status {
            OperationStatus::Pending => "Pending",
            OperationStatus::Running => "Running",
            OperationStatus::Completed => "Completed",
            OperationStatus::Failed => "Failed",
            OperationStatus::Cancelled => "Cancelled",
        };
        sqlx::query("UPDATE operation SET status = $1 WHERE id = $2")
            .bind(s)
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ProjectStore for PostgresBackend {
    async fn insert(&self, p: Project) -> Result<(), DomainError> {
        let (id, name, created_at) = mapper::project_to_row(&p);
        sqlx::query("INSERT INTO project (id, name, created_at) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ProjectId) -> Result<Option<Project>, DomainError> {
        let row: Option<mapper::ProjectRow> =
            sqlx::query_as("SELECT id, name, created_at FROM project WHERE id = $1")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(row.map(mapper::project_from_row))
    }
}

#[async_trait]
impl ResearchStore for PostgresBackend {
    async fn insert(&self, r: ResearchArtifact) -> Result<(), DomainError> {
        let row = mapper::research_to_row(&r);
        sqlx::query(
            "INSERT INTO research_artifact (id, project_id, title, source, summary, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(row.id)
        .bind(row.project_id)
        .bind(&row.title)
        .bind(&row.source)
        .bind(&row.summary)
        .bind(row.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ResearchId) -> Result<Option<ResearchArtifact>, DomainError> {
        let row: Option<mapper::ResearchRow> = sqlx::query_as(
            "SELECT id, project_id, title, source, summary, created_at, tsv FROM research_artifact WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(row.map(mapper::research_from_row))
    }
}

#[async_trait]
impl SearchIndex for PostgresBackend {
    async fn search(&self, q: SearchQuery) -> Result<Vec<SearchHit>, DomainError> {
        // Postgres tsvector / tsquery.
        let tsquery = queries::encode_tsquery(&q.text);
        let rows = sqlx::query(
            "SELECT id, ts_headline('english', summary, to_tsquery('english', $1), \
             'StartSel=[],StopSel=[],MaxFragments=1,MaxWords=10,MinWords=5') AS snip \
             FROM research_artifact \
             WHERE tsv @@ to_tsquery('english', $1) \
             LIMIT $2",
        )
        .bind(&tsquery)
        .bind(q.limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        let mut hits = Vec::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.try_get("id").unwrap_or_default();
            let snip: String = row.try_get("snip").unwrap_or_default();
            hits.push(SearchHit {
                ref_id: id.to_string(),
                scope: q.scope.clone(),
                snippet: snip,
            });
        }
        Ok(hits)
    }
}

#[async_trait]
impl MultiEntityTx for PostgresBackend {
    async fn run(&self, ops: Vec<MultiOp>) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        for op in ops {
            match op {
                MultiOp::InsertOperation(o) => {
                    let r = mapper::operation_to_row(&o);
                    sqlx::query(
                        "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
                         runtime_instance_id, created_at, durable_seq, outcome) \
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                    )
                    .bind(r.id)
                    .bind(&r.kind)
                    .bind(&r.status)
                    .bind(r.thread_id)
                    .bind(r.workflow_id)
                    .bind(r.task_id)
                    .bind(r.runtime_instance_id)
                    .bind(r.created_at)
                    .bind(r.durable_seq)
                    .bind(&r.outcome)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::AppendEvent(e) => {
                    let r = mapper::event_to_row(&e);
                    sqlx::query(
                        "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
                         VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(r.id)
                    .bind(r.durable_seq)
                    .bind(&r.kind)
                    .bind(r.occurred_at)
                    .bind(r.operation_id)
                    .bind(&r.payload)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::InsertCommand(c) => {
                    let r = mapper::command_to_row(&c);
                    sqlx::query(
                        "INSERT INTO command_record (command_id, principal, scope, operation_id, recorded_at) \
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(r.command_id)
                    .bind(r.principal)
                    .bind(&r.scope)
                    .bind(r.operation_id)
                    .bind(r.recorded_at)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::UpdateWorkflow(wf, status) => {
                    let now_sql = queries::now_sql();
                    let sql = format!(
                        "INSERT INTO workflow_status(workflow_id, status, set_at) \
                         VALUES ($1, $2, {now_sql}) \
                         ON CONFLICT (workflow_id) DO UPDATE SET status = EXCLUDED.status, set_at = EXCLUDED.set_at"
                    );
                    sqlx::query(&sql)
                        .bind(wf.0)
                        .bind(&status)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
            }
        }
        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
}
