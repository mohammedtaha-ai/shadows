//! Failures owned by code indexing and project reach.
#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    #[error("{0}")]
    InvalidCommand(String),
    #[error("{0}")]
    GrantScope(String),
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
