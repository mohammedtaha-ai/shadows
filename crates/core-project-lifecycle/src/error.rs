//! Cross-owner project lifecycle failures.
#[derive(Debug, thiserror::Error)]
pub enum ProjectLifecycleError {
    #[error("the project has planning threads; it cannot be removed")]
    ProjectHasThreads,
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
impl From<sqlx::Error> for ProjectLifecycleError {
    fn from(error: sqlx::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}
impl From<serde_json::Error> for ProjectLifecycleError {
    fn from(error: serde_json::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}
