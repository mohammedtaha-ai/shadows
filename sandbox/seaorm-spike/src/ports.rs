//! Port (capability) traits. Pure async Rust, no persistence types.

use async_trait::async_trait;
use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, Operation, OperationId, OperationStatus, Principal,
    Project, ProjectId, ResearchArtifact, ResearchId, SearchHit, SearchQuery, WorkflowId,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found")]
    NotFound,
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),
}

#[async_trait]
pub trait AtomicCommandTx: Send + Sync {
    async fn run<F, Fut>(
        &self,
        command_id: CommandId,
        principal: Principal,
        scope: String,
        build: F,
    ) -> Result<CommandRecord, DomainError>
    where
        F: FnOnce(OperationId) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(Operation, DurableEvent), DomainError>> + Send;
}

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, evt: DurableEvent) -> Result<(), DomainError>;
    async fn read_after(&self, seq: u64, limit: u32) -> Result<Vec<DurableEvent>, DomainError>;
}

#[async_trait]
pub trait OperationStore: Send + Sync {
    async fn insert(&self, op: Operation) -> Result<(), DomainError>;
    async fn get(&self, id: OperationId) -> Result<Option<Operation>, DomainError>;
    async fn update_status(
        &self,
        id: OperationId,
        status: OperationStatus,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ProjectStore: Send + Sync {
    async fn insert(&self, p: Project) -> Result<(), DomainError>;
    async fn get(&self, id: ProjectId) -> Result<Option<Project>, DomainError>;
}

#[async_trait]
pub trait ResearchStore: Send + Sync {
    async fn insert(&self, r: ResearchArtifact) -> Result<(), DomainError>;
    async fn get(&self, id: ResearchId) -> Result<Option<ResearchArtifact>, DomainError>;
}

#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn search(&self, q: SearchQuery) -> Result<Vec<SearchHit>, DomainError>;
}

#[async_trait]
pub trait MultiEntityTx: Send + Sync {
    /// Apply a batch of operations atomically.
    /// The application builds a Vec<MultiOp> outside the transaction,
    /// then hands it to the backend for atomic execution.
    async fn run(&self, ops: Vec<MultiOp>) -> Result<(), DomainError>;
}

/// Operations a multi-entity transaction can perform.
/// This is a value type — no trait object, no lifetime issues.
#[derive(Debug, Clone)]
pub enum MultiOp {
    InsertOperation(Operation),
    AppendEvent(DurableEvent),
    InsertCommand(CommandRecord),
    UpdateWorkflow(WorkflowId, String),
}

pub trait Backend:
    AtomicCommandTx
    + EventStore
    + OperationStore
    + ProjectStore
    + ResearchStore
    + SearchIndex
    + MultiEntityTx
    + Send
    + Sync
{
    fn dialect(&self) -> &'static str;
}
