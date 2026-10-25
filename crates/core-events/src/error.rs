//! Event subscription failures.
#[derive(Debug, thiserror::Error)]
pub enum EventsError {
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
