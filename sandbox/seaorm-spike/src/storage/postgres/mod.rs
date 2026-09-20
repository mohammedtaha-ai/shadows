//! SeaORM Postgres adapter — minimal stub showing same shape as sqlite.
//! This spike focuses on SQLite for SeaORM; the Postgres path uses the
//! same `DatabaseConnection` abstraction, so porting the storage code
//! is mechanical (the same connector swap, the same Statement::from_string
//! with DatabaseBackend::Postgres). Domain isolation is identical.

use async_trait::async_trait;
use sea_orm::{Database, DatabaseConnection, DbBackend, Statement};
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

pub struct PostgresBackend {
    db: DatabaseConnection,
}

impl PostgresBackend {
    pub async fn connect(url: &str) -> Result<Self, DomainError> {
        use sea_orm_migration::MigratorTrait;
        let db = Database::connect(url)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        let s = Self { db };
        crate::migration_pg::Migrator::up(&s.db, None)
            .await
            .map_err(|e| DomainError::Storage(e.to_string()))?;
        Ok(s)
    }
}

impl Backend for PostgresBackend {
    fn dialect(&self) -> &'static str {
        "postgres"
    }
}

// Storage code follows the same patterns as the SQLite adapter —
// for the spike the SQLite side is the focus. Stub implementations
// satisfy the trait so the type compiles, but they panic if called.
#[async_trait]
impl AtomicCommandTx for PostgresBackend {
    async fn run<F, Fut>(
        &self,
        _command_id: CommandId,
        _principal: Principal,
        _scope: String,
        _build: F,
    ) -> Result<CommandRecord, DomainError>
    where
        F: FnOnce(OperationId) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(Operation, DurableEvent), DomainError>> + Send,
    {
        Err(DomainError::Storage(
            "Postgres not implemented for SeaORM spike".into(),
        ))
    }
}
#[async_trait]
impl EventStore for PostgresBackend {
    async fn append(&self, _evt: DurableEvent) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
    async fn read_after(&self, _seq: u64, _limit: u32) -> Result<Vec<DurableEvent>, DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
#[async_trait]
impl OperationStore for PostgresBackend {
    async fn insert(&self, _op: Operation) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
    async fn get(&self, _id: OperationId) -> Result<Option<Operation>, DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
    async fn update_status(
        &self,
        _id: OperationId,
        _s: OperationStatus,
    ) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
#[async_trait]
impl ProjectStore for PostgresBackend {
    async fn insert(&self, _p: Project) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
    async fn get(&self, _id: ProjectId) -> Result<Option<Project>, DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
#[async_trait]
impl ResearchStore for PostgresBackend {
    async fn insert(&self, _r: ResearchArtifact) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
    async fn get(&self, _id: ResearchId) -> Result<Option<ResearchArtifact>, DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
#[async_trait]
impl SearchIndex for PostgresBackend {
    async fn search(&self, _q: SearchQuery) -> Result<Vec<SearchHit>, DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
#[async_trait]
impl MultiEntityTx for PostgresBackend {
    async fn run(&self, _ops: Vec<MultiOp>) -> Result<(), DomainError> {
        Err(DomainError::Storage("not implemented".into()))
    }
}
