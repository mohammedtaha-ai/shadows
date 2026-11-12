//! Failures owned by code indexing and project reach.
#[derive(Debug, thiserror::Error)]
pub enum CodebaseError {
    #[error("{0}")]
    InvalidCommand(String),
    #[error("{0}")]
    GrantScope(String),
    #[error("the file list changed; read it again from the start")]
    InventoryStale,
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
