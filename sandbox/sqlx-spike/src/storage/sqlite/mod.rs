//! SQLx SQLite adapter. Mirrors seaorm-spike structure.

use async_trait::async_trait;
use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, Operation, OperationId, OperationStatus, Principal,
    Project, ProjectId, ResearchArtifact, ResearchId, SearchHit, SearchQuery,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use std::str::FromStr;

use crate::ports::{
    AtomicCommandTx, Backend, DomainError, EventStore, MultiEntityTx, MultiOp, OperationStore,
    ProjectStore, ResearchStore, SearchIndex,
};

pub mod entities;
pub mod migrations;
pub mod queries;

pub use entities as mapper;

pub struct SqliteBackend {
    pool: SqlitePool,
}

impl SqliteBackend {
    pub async fn connect_in_memory() -> Result<Self, DomainError> {
        // For the spike we use a single connection via shared cache.
        // Multi-connection pools hit SQLite write-lock deadlocks that
        // would need retry logic — recorded as a finding, not solved here.
        let opts: sqlx::sqlite::SqliteConnectOptions =
            sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:?cache=shared")
                .map_err(|e| DomainError::Storage(e.to_string()))?
                .busy_timeout(std::time::Duration::from_secs(5));
        let pool = sqlx::Pool::connect_with(opts)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let s = Self { pool };
        s.run_migrations().await?;
        Ok(s)
    }

    pub async fn connect_file(path: &str) -> Result<Self, DomainError> {
        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{path}?mode=rwc"))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let s = Self { pool };
        s.run_migrations().await?;
        Ok(s)
    }

    pub async fn run_migrations(&self) -> Result<(), DomainError> {
        migrations::run_migrations(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn next_durable_seq_tx(conn: &mut Transaction<'_, Sqlite>) -> Result<u64, DomainError> {
        let q = sqlx::query("UPDATE durable_seq_counter SET n = n + 1 RETURNING n");
        let row = q
            .fetch_one(&mut **conn)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let n: i64 = row
            .try_get("n")
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(n as u64)
    }
}

impl Backend for SqliteBackend {
    fn dialect(&self) -> &'static str {
        "sqlite"
    }
}

#[async_trait]
impl AtomicCommandTx for SqliteBackend {
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

        let existing: Option<mapper::CommandRow> =
            sqlx::query_as::<_, mapper::CommandRow>("SELECT command_id, principal, scope, operation_id, recorded_at FROM command_record WHERE command_id = ?1")
                .bind(command_id.0.to_string())
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
        let seq = Self::next_durable_seq_tx(&mut tx).await?;
        let (mut op, mut evt) = build(op_id).await?;
        op.durable_seq = seq;
        evt.durable_seq = seq;

        let op_row = mapper::operation_to_row(&op);
        sqlx::query(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&op_row.id)
        .bind(&op_row.kind)
        .bind(&op_row.status)
        .bind(&op_row.thread_id)
        .bind(&op_row.workflow_id)
        .bind(&op_row.task_id)
        .bind(&op_row.runtime_instance_id)
        .bind(&op_row.created_at)
        .bind(op_row.durable_seq)
        .bind(&op_row.outcome)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;

        let evt_row = mapper::event_to_row(&evt);
        sqlx::query(
            "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&evt_row.id)
        .bind(evt_row.durable_seq)
        .bind(&evt_row.kind)
        .bind(&evt_row.occurred_at)
        .bind(&evt_row.operation_id)
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
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&cr.command_id)
        .bind(&cr.principal)
        .bind(&cr.scope)
        .bind(&cr.operation_id)
        .bind(&cr.recorded_at)
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
impl EventStore for SqliteBackend {
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&r.id)
        .bind(r.durable_seq)
        .bind(&r.kind)
        .bind(&r.occurred_at)
        .bind(&r.operation_id)
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
             FROM durable_event WHERE durable_seq > ?1 \
             ORDER BY durable_seq ASC LIMIT ?2",
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
impl OperationStore for SqliteBackend {
    async fn insert(&self, op: Operation) -> Result<(), DomainError> {
        let r = mapper::operation_to_row(&op);
        sqlx::query(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&r.id)
        .bind(&r.kind)
        .bind(&r.status)
        .bind(&r.thread_id)
        .bind(&r.workflow_id)
        .bind(&r.task_id)
        .bind(&r.runtime_instance_id)
        .bind(&r.created_at)
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
             created_at, durable_seq, outcome FROM operation WHERE id = ?1",
        )
        .bind(id.0.to_string())
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
        sqlx::query("UPDATE operation SET status = ?1 WHERE id = ?2")
            .bind(s)
            .bind(id.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ProjectStore for SqliteBackend {
    async fn insert(&self, p: Project) -> Result<(), DomainError> {
        let (id, name, created_at) = mapper::project_to_row(&p);
        sqlx::query("INSERT INTO project (id, name, created_at) VALUES (?1, ?2, ?3)")
            .bind(id.to_string())
            .bind(name)
            .bind(created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ProjectId) -> Result<Option<Project>, DomainError> {
        let row: Option<mapper::ProjectRow> =
            sqlx::query_as("SELECT id, name, created_at FROM project WHERE id = ?1")
                .bind(id.0.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(row.map(mapper::project_from_row))
    }
}

#[async_trait]
impl ResearchStore for SqliteBackend {
    async fn insert(&self, r: ResearchArtifact) -> Result<(), DomainError> {
        let row = mapper::research_to_row(&r);
        sqlx::query(
            "INSERT INTO research_artifact (id, project_id, title, source, summary, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&row.id)
        .bind(&row.project_id)
        .bind(&row.title)
        .bind(&row.source)
        .bind(&row.summary)
        .bind(&row.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ResearchId) -> Result<Option<ResearchArtifact>, DomainError> {
        let row: Option<mapper::ResearchRow> = sqlx::query_as(
            "SELECT id, project_id, title, source, summary, created_at FROM research_artifact WHERE id = ?1",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(row.map(mapper::research_from_row))
    }
}

#[async_trait]
impl SearchIndex for SqliteBackend {
    async fn search(&self, q: SearchQuery) -> Result<Vec<SearchHit>, DomainError> {
        let term = queries::encode_fts5_literal_query(&q.text);
        let rows = sqlx::query(
            "SELECT rowid AS ref_rowid, snippet(research_fts, 1, '[', ']', '…', 10) AS snip \
             FROM research_fts WHERE research_fts MATCH ?1 LIMIT ?2",
        )
        .bind(term)
        .bind(q.limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(e.to_string()))?;
        let mut hits = Vec::with_capacity(rows.len());
        for row in rows {
            let rowid: i64 = row.try_get("ref_rowid").unwrap_or_default();
            let snip: String = row.try_get("snip").unwrap_or_default();
            hits.push(SearchHit {
                ref_id: format!("rowid:{rowid}"),
                scope: q.scope.clone(),
                snippet: snip,
            });
        }
        Ok(hits)
    }
}

struct SqliteMultiSession<'a> {
    tx: Transaction<'a, Sqlite>,
}

#[async_trait]
impl MultiEntityTx for SqliteBackend {
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
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    )
                    .bind(&r.id)
                    .bind(&r.kind)
                    .bind(&r.status)
                    .bind(&r.thread_id)
                    .bind(&r.workflow_id)
                    .bind(&r.task_id)
                    .bind(&r.runtime_instance_id)
                    .bind(&r.created_at)
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
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    )
                    .bind(&r.id)
                    .bind(r.durable_seq)
                    .bind(&r.kind)
                    .bind(&r.occurred_at)
                    .bind(&r.operation_id)
                    .bind(&r.payload)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::InsertCommand(c) => {
                    let r = mapper::command_to_row(&c);
                    sqlx::query(
                        "INSERT INTO command_record (command_id, principal, scope, operation_id, recorded_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                    )
                    .bind(&r.command_id)
                    .bind(&r.principal)
                    .bind(&r.scope)
                    .bind(&r.operation_id)
                    .bind(&r.recorded_at)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::UpdateWorkflow(wf, status) => {
                    let now_sql = queries::now_sql();
                    let sql = format!(
                        "INSERT INTO workflow_status(workflow_id, status, set_at) \
                         VALUES (?1, ?2, {}) \
                         ON CONFLICT(workflow_id) DO UPDATE SET status=excluded.status, set_at=excluded.set_at",
                        now_sql
                    );
                    sqlx::query(&sql)
                        .bind(wf.0.to_string())
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
