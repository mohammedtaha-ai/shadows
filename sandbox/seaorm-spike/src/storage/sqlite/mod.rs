//! SQLite storage adapter for the SeaORM prototype.
//!
//! Strategy: skip SeaORM's entity macros. Use `DatabaseConnection` as a
//! connection abstraction + sea-query's `Query` builder for portable SQL,
//! plus raw `Statement::execute` for SQLite-specific features (FTS5).

use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait,
};
use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, Operation, OperationId, OperationStatus, Principal,
    Project, ProjectId, ResearchArtifact, ResearchId, SearchHit, SearchQuery,
};

use crate::ports::{
    AtomicCommandTx, Backend, DomainError, EventStore, MultiEntityTx, MultiOp, OperationStore,
    ProjectStore, ResearchStore, SearchIndex,
};

pub mod entities;
pub mod migrations;
pub mod queries;

pub use entities as mapper;

pub struct SqliteBackend {
    db: DatabaseConnection,
}

impl SqliteBackend {
    pub async fn connect_in_memory() -> Self {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite connect");
        Self { db }
    }

    pub async fn connect_file(path: &str) -> Result<Self, DomainError> {
        let db = Database::connect(format!("sqlite://{path}?mode=rwc"))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(Self { db })
    }

    pub async fn run_migrations(&self) -> Result<(), DomainError> {
        use sea_orm_migration::MigratorTrait;
        crate::migration::Migrator::up(&self.db, None)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }

    async fn next_durable_seq(&self) -> Result<u64, DomainError> {
        let conn = &self.db;
        let q = Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE durable_seq_counter SET n = n + 1 RETURNING n".to_string(),
        );
        let row = conn
            .query_one(q)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?
            .ok_or_else(|| DomainError::Storage("no row from RETURNING".into()))?;
        let n: i64 = row
            .try_get_by("n")
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(n as u64)
    }

    fn stmt(sql: String) -> Statement {
        Statement {
            sql,
            values: None,
            db_backend: DatabaseBackend::Sqlite,
        }
    }

    fn stmt_with(sql: String, v: Vec<sea_orm::Value>) -> Statement {
        Statement {
            sql,
            values: Some(sea_orm::Values(v)),
            db_backend: DatabaseBackend::Sqlite,
        }
    }
}

impl Backend for SqliteBackend {
    fn dialect(&self) -> &'static str {
        "sqlite"
    }
}

// =================================================================
// AtomicCommandTx
// =================================================================
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
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        // Idempotency check: existing command_id returns the prior CommandRecord.
        let check_sql = format!(
            "SELECT command_id, principal, scope, operation_id, recorded_at \
             FROM command_record WHERE command_id = '{}'",
            command_id.0
        );
        let existing = txn
            .query_one(Self::stmt(check_sql))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        if let Some(row) = existing {
            txn.commit()
                .await
                .map_err(|e| DomainError::Storage(e.to_string()))?;
            let cr = mapper::CommandRow {
                command_id: row.try_get_by("command_id").unwrap_or_default(),
                principal: row.try_get_by("principal").unwrap_or_default(),
                scope: row.try_get_by("scope").unwrap_or_default(),
                operation_id: row.try_get_by("operation_id").unwrap_or_default(),
                recorded_at: row.try_get_by("recorded_at").unwrap_or_default(),
            };
            return Ok(mapper::command_from_row(cr));
        }

        let op_id = OperationId::new();
        let seq = {
            let q = Statement::from_string(
                DatabaseBackend::Sqlite,
                "UPDATE durable_seq_counter SET n = n + 1 RETURNING n".to_string(),
            );
            let row = txn
                .query_one(q)
                .await
                .map_err(|e| DomainError::Storage(e.to_string()))?
                .ok_or_else(|| DomainError::Storage("seq RETURNING empty".into()))?;
            let n: i64 = row
                .try_get_by("n")
                .map_err(|e| DomainError::Storage(e.to_string()))?;
            n as u64
        };

        let (mut op, mut evt) = build(op_id).await?;

        // Assign durable_seq so caller doesn't have to.
        op.durable_seq = seq;
        evt.durable_seq = seq;

        let op_row = mapper::operation_to_row(&op);
        let insert_op_sql = format!(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES ('{}','{}','{}',{},{},{},'{}','{}',{},{})",
            op_row.id,
            op_row.kind,
            op_row.status,
            op_row
                .thread_id
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or("NULL".to_string()),
            op_row
                .workflow_id
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or("NULL".to_string()),
            op_row
                .task_id
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or("NULL".to_string()),
            op_row.runtime_instance_id,
            op_row.created_at,
            op_row.durable_seq,
            op_row
                .outcome
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or("NULL".to_string()),
        );
        txn.execute_unprepared(&insert_op_sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        let evt_row = mapper::event_to_row(&evt);
        let insert_evt_sql = format!(
            "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES ('{}', {}, '{}', '{}', {}, '{}')",
            evt_row.id,
            evt_row.durable_seq,
            evt_row.kind,
            evt_row.occurred_at,
            evt_row
                .operation_id
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or("NULL".to_string()),
            evt_row.payload.replace('\'', "''"),
        );
        txn.execute_unprepared(&insert_evt_sql)
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
        let insert_cmd_sql = format!(
            "INSERT INTO command_record (command_id, principal, scope, operation_id, recorded_at) \
             VALUES ('{}','{}','{}','{}','{}')",
            cr.command_id, cr.principal, cr.scope, cr.operation_id, cr.recorded_at
        );
        txn.execute_unprepared(&insert_cmd_sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;

        Ok(cmd)
    }
}

// =================================================================
// EventStore
// =================================================================
#[async_trait]
impl EventStore for SqliteBackend {
    async fn append(&self, evt: DurableEvent) -> Result<(), DomainError> {
        let seq_sql = "UPDATE durable_seq_counter SET n = n + 1 RETURNING n";
        let q = Statement::from_string(DatabaseBackend::Sqlite, seq_sql.to_string());
        let row = self
            .db
            .query_one(q)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?
            .ok_or_else(|| DomainError::Storage("seq empty".into()))?;
        let n: i64 = row
            .try_get_by("n")
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let mut evt = evt;
        evt.durable_seq = n as u64;

        let r = mapper::event_to_row(&evt);
        let sql = format!(
            "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
             VALUES ('{}', {}, '{}', '{}', {}, '{}')",
            r.id,
            r.durable_seq,
            r.kind,
            r.occurred_at,
            r.operation_id
                .as_ref()
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "NULL".to_string()),
            r.payload.replace('\'', "''"),
        );
        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn read_after(&self, seq: u64, limit: u32) -> Result<Vec<DurableEvent>, DomainError> {
        // Order strictly by durable_seq — never by rowid.
        let sql = format!(
            "SELECT id, durable_seq, kind, occurred_at, operation_id, payload \
             FROM durable_event WHERE durable_seq > {seq} \
             ORDER BY durable_seq ASC LIMIT {limit}"
        );
        let rows: Vec<mapper::EventRow> = self
            .db
            .query_all(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?
            .into_iter()
            .map(|row| mapper::EventRow {
                id: row.try_get_by("id").unwrap_or_default(),
                durable_seq: row.try_get_by("durable_seq").unwrap_or_default(),
                kind: row.try_get_by("kind").unwrap_or_default(),
                occurred_at: row.try_get_by("occurred_at").unwrap_or_default(),
                operation_id: row.try_get_by("operation_id").ok(),
                payload: row.try_get_by("payload").unwrap_or_default(),
            })
            .collect();
        Ok(rows.into_iter().map(mapper::event_from_row).collect())
    }
}

// =================================================================
// OperationStore
// =================================================================
#[async_trait]
impl OperationStore for SqliteBackend {
    async fn insert(&self, op: Operation) -> Result<(), DomainError> {
        let r = mapper::operation_to_row(&op);
        let sql = format!(
            "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
             runtime_instance_id, created_at, durable_seq, outcome) \
             VALUES ('{}','{}','{}',{},{},{},'{}','{}',{},{})",
            r.id,
            r.kind,
            r.status,
            r.thread_id
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "NULL".to_string()),
            r.workflow_id
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "NULL".to_string()),
            r.task_id
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "NULL".to_string()),
            r.runtime_instance_id,
            r.created_at,
            r.durable_seq,
            r.outcome
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "NULL".to_string()),
        );
        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: OperationId) -> Result<Option<Operation>, DomainError> {
        let sql = format!(
            "SELECT id, kind, status, thread_id, workflow_id, task_id, runtime_instance_id, \
             created_at, durable_seq, outcome FROM operation WHERE id = '{}'",
            id.0
        );
        let opt = self
            .db
            .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(opt.map(|row| {
            mapper::operation_from_row(mapper::OperationRow {
                id: row.try_get_by("id").unwrap_or_default(),
                kind: row.try_get_by("kind").unwrap_or_default(),
                status: row.try_get_by("status").unwrap_or_default(),
                thread_id: row.try_get_by("thread_id").ok(),
                workflow_id: row.try_get_by("workflow_id").ok(),
                task_id: row.try_get_by("task_id").ok(),
                runtime_instance_id: row.try_get_by("runtime_instance_id").unwrap_or_default(),
                created_at: row.try_get_by("created_at").unwrap_or_default(),
                durable_seq: row.try_get_by("durable_seq").unwrap_or_default(),
                outcome: row.try_get_by("outcome").ok(),
            })
        }))
    }
    async fn update_status(
        &self,
        id: OperationId,
        status: OperationStatus,
    ) -> Result<(), DomainError> {
        let sql = format!(
            "UPDATE operation SET status = '{}' WHERE id = '{}'",
            match status {
                OperationStatus::Pending => "Pending",
                OperationStatus::Running => "Running",
                OperationStatus::Completed => "Completed",
                OperationStatus::Failed => "Failed",
                OperationStatus::Cancelled => "Cancelled",
            },
            id.0
        );
        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
}

// =================================================================
// ProjectStore
// =================================================================
#[async_trait]
impl ProjectStore for SqliteBackend {
    async fn insert(&self, p: Project) -> Result<(), DomainError> {
        let (id, name, created_at) = mapper::project_to_row(&p);
        let sql = format!(
            "INSERT INTO project (id, name, created_at) VALUES ('{id}', '{name}', '{created_at}')"
        );
        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ProjectId) -> Result<Option<Project>, DomainError> {
        let sql = format!(
            "SELECT id, name, created_at FROM project WHERE id = '{}'",
            id.0
        );
        let opt = self
            .db
            .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(opt.map(|row| {
            mapper::project_from_row(mapper::ProjectRow {
                id: row.try_get_by("id").unwrap_or_default(),
                name: row.try_get_by("name").unwrap_or_default(),
                created_at: row.try_get_by("created_at").unwrap_or_default(),
            })
        }))
    }
}

// =================================================================
// ResearchStore
// =================================================================
#[async_trait]
impl ResearchStore for SqliteBackend {
    async fn insert(&self, r: ResearchArtifact) -> Result<(), DomainError> {
        let row = mapper::research_to_row(&r);
        let sql =
            format!(
            "INSERT INTO research_artifact (id, project_id, title, source, summary, created_at) \
             VALUES ('{}', '{}', '{}', {}, '{}', '{}')",
            row.id,
            row.project_id,
            row.title,
            row.source.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
            row.summary,
            row.created_at,
        );
        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
    async fn get(&self, id: ResearchId) -> Result<Option<ResearchArtifact>, DomainError> {
        let sql = format!(
            "SELECT id, project_id, title, source, summary, created_at FROM research_artifact WHERE id = '{}'",
            id.0
        );
        let opt = self
            .db
            .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(opt.map(|row| {
            mapper::research_from_row(mapper::ResearchRow {
                id: row.try_get_by("id").unwrap_or_default(),
                project_id: row.try_get_by("project_id").unwrap_or_default(),
                title: row.try_get_by("title").unwrap_or_default(),
                source: row.try_get_by("source").ok(),
                summary: row.try_get_by("summary").unwrap_or_default(),
                created_at: row.try_get_by("created_at").unwrap_or_default(),
            })
        }))
    }
}

// =================================================================
// SearchIndex
// =================================================================
#[async_trait]
impl SearchIndex for SqliteBackend {
    async fn search(&self, q: SearchQuery) -> Result<Vec<SearchHit>, DomainError> {
        // Backend-specific: FTS5 MATCH with literal query encoding.
        let term = queries::encode_fts5_literal_query(&q.text);
        let sql = format!(
            "SELECT rowid AS ref_rowid, snippet(research_fts, 1, '[', ']', '…', 10) AS snip \
             FROM research_fts WHERE research_fts MATCH ?1 LIMIT {lim}",
            lim = q.limit
        );
        let stmt = Self::stmt_with(sql, vec![sea_orm::Value::from(term)]);
        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let mut hits = Vec::with_capacity(rows.len());
        for row in rows {
            let rowid: i64 = row.try_get_by("ref_rowid").unwrap_or_default();
            let snip: String = row.try_get_by("snip").unwrap_or_default();
            hits.push(SearchHit {
                ref_id: format!("rowid:{rowid}"),
                scope: q.scope.clone(),
                snippet: snip,
            });
        }
        Ok(hits)
    }
}

// =================================================================
// MultiEntityTx
// =================================================================
#[async_trait]
impl MultiEntityTx for SqliteBackend {
    async fn run(&self, ops: Vec<MultiOp>) -> Result<(), DomainError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        for op in ops {
            match op {
                MultiOp::InsertOperation(o) => {
                    let r = mapper::operation_to_row(&o);
                    let sql = format!(
                        "INSERT INTO operation (id, kind, status, thread_id, workflow_id, task_id, \
                         runtime_instance_id, created_at, durable_seq, outcome) \
                         VALUES ('{}','{}','{}',{},{},{},'{}','{}',{},{})",
                        r.id, r.kind, r.status,
                        r.thread_id.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
                        r.workflow_id.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
                        r.task_id.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
                        r.runtime_instance_id, r.created_at, r.durable_seq,
                        r.outcome.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
                    );
                    txn.execute_unprepared(&sql)
                        .await
                        .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::AppendEvent(e) => {
                    let r = mapper::event_to_row(&e);
                    let sql = format!(
                        "INSERT INTO durable_event (id, durable_seq, kind, occurred_at, operation_id, payload) \
                         VALUES ('{}', {}, '{}', '{}', {}, '{}')",
                        r.id, r.durable_seq, r.kind, r.occurred_at,
                        r.operation_id.map(|s| format!("'{s}'")).unwrap_or_else(|| "NULL".to_string()),
                        r.payload.replace('\'', "''"),
                    );
                    txn.execute_unprepared(&sql)
                        .await
                        .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::InsertCommand(c) => {
                    let r = mapper::command_to_row(&c);
                    let sql = format!(
                        "INSERT INTO command_record (command_id, principal, scope, operation_id, recorded_at) \
                         VALUES ('{}','{}','{}','{}','{}')",
                        r.command_id, r.principal, r.scope, r.operation_id, r.recorded_at
                    );
                    txn.execute_unprepared(&sql)
                        .await
                        .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
                MultiOp::UpdateWorkflow(wf, status) => {
                    let sql = format!(
                        "INSERT INTO workflow_status(workflow_id, status, set_at) VALUES ('{}', '{}', {}) \
                         ON CONFLICT(workflow_id) DO UPDATE SET status=excluded.status, set_at=excluded.set_at",
                        wf.0, status, queries::now_sql()
                    );
                    txn.execute_unprepared(&sql)
                        .await
                        .map_err(|e| DomainError::Storage(e.to_string()))?;
                }
            }
        }
        txn.commit()
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(())
    }
}
