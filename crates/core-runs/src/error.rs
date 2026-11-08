//! Why a run command was refused.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum RunsError {
    #[error("only an approved plan version can be run")]
    NotFrozen,
    #[error("a newer version of this plan exists")]
    Superseded,
    #[error("the plan is archived")]
    Archived,
    #[error("the task waits for tasks {tasks:?}")]
    Waiting { tasks: Vec<u32> },
    #[error("the task already has an attempt that can still write")]
    TaskBusy,
    #[error("task {task} is running and may write the same files")]
    Overlap { task: u32 },
    #[error("the command id was used for another request")]
    CommandConflict,
    #[error("not found")]
    NotFound,
    #[error("the task's writes are not valid patterns: {0}")]
    BadScope(String),
    #[error("{0}")]
    Unavailable(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for RunsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<serde_json::Error> for RunsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}
