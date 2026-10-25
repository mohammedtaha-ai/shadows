//! Turn lifecycle failures and refusals.
use core_db::StorageError;
#[derive(Debug, thiserror::Error)]
pub enum TurnsError {
    #[error("the thread has a turn running")]
    ThreadBusy,
    #[error("the waiting message was already sent or removed")]
    QueuedMessageGone,
    #[error("the daemon is stopping")]
    RuntimeStopping,
    #[error("the turn's process tree could not be terminated")]
    TerminationFailed,
    #[error("the harness could not start: {0}")]
    HarnessStartFailed(String),
    #[error("{0}")]
    InvalidCommand(String),
    #[error("{what} {id} is not offered")]
    SettingNotOffered {
        what: String,
        id: String,
        detail: Option<String>,
    },
    #[error("this project does not allow the {0} mode")]
    ModeNotAllowed(String),
    #[error(transparent)]
    Start(#[from] crate::StartError),
    #[error(transparent)]
    Harness(core_harness::HarnessError),
    #[error(transparent)]
    Plans(#[from] core_plans::PlansError),
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Threads(#[from] core_threads::ThreadsError),
    #[error(transparent)]
    Instructions(#[from] core_instructions::InstructionsError),
    #[error(transparent)]
    Standards(#[from] core_standards::StandardsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
impl From<sqlx::Error> for TurnsError {
    fn from(error: sqlx::Error) -> Self {
        StorageError::from(error).into()
    }
}
impl From<serde_json::Error> for TurnsError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::from(error).into()
    }
}
impl From<core_harness::HarnessError> for TurnsError {
    fn from(error: core_harness::HarnessError) -> Self {
        match error {
            core_harness::HarnessError::ThreadBusy => Self::ThreadBusy,
            core_harness::HarnessError::Storage(error) => error.into(),
            other => Self::Harness(other),
        }
    }
}
