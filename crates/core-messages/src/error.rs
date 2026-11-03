use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum MessagesError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("group not found")]
    NotFound,
    #[error(transparent)]
    Agent(#[from] core_agents::AgentsError),
    #[error(transparent)]
    Thread(#[from] core_threads::ThreadsError),
    #[error(transparent)]
    Turn(#[from] core_turns::TurnsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for MessagesError {
    fn from(error: sqlx::Error) -> Self {
        StorageError::from(error).into()
    }
}

impl From<serde_json::Error> for MessagesError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::from(error).into()
    }
}
