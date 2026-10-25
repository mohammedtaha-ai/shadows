//! Thread refusals and their stable application error mapping.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ThreadsError {
    #[error("the thread's harness is fixed: it already ran a turn, or it is a fork")]
    HarnessLocked,
    #[error("only the thread's last entry, written by a completed turn, can be forked from")]
    ForkPointNotSupported,
    #[error("{0}")]
    InvalidCommand(String),
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Agent(#[from] core_agents::AgentsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for ThreadsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<serde_json::Error> for ThreadsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}
