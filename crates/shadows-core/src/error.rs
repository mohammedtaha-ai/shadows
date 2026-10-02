use crate::db::StorageError;
use crate::harness::OpenError;
use crate::projects::DirectoryError;
use crate::turns::StartError;

/// Stable codes clients pattern-match on. Never match on human text.
/// Spec §3.4. `Blocked`/`Rejected` are domain outcomes and never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    HarnessStartFailed,
    ProcessSpawnFailed,
    ProcessTerminated,
    ProcessTerminationFailed,
    RuntimeStopping,
    StorageUnavailable,
    StorageMigrationFailed,
    StorageConstraintViolation,
    CommandConflict,
    IdempotencyKeyRequired,
    InvalidCommand,
    InvalidCursor,
    AgentAuthFailed,
    AgentUnsupportedProfile,
    PathInvalid,
    PathNotFound,
    PathNotADirectory,
    PathAccessDenied,
    PathAlreadyExists,
    PathUnavailable,
    OriginRefused,
    /// Spec §12.10: the thread's harness is listed but not runnable.
    HarnessUnavailable,
    /// A model, mode or effort the session does not offer, or a mode outside
    /// Shadows' policy.
    SettingNotOffered,
    /// The project does not allow this mode (§12.5).
    ModeNotAllowed,
    /// The thread already ran a turn on its harness (§12.6).
    HarnessLocked,
    /// The thread has a turn running.
    ThreadBusy,
    /// Fork from anything but the last completed entry (§12.9).
    ForkPointNotSupported,
    /// A change to a frozen plan version (§13.2); 409.
    WorkflowFrozenImmutable,
    /// A check of §13.4 failed; the answer lists each problem; 422.
    WorkflowValidationFailed,
    /// `expected_revision` is stale (§13.5); the answer carries the current
    /// revision; 409.
    RevisionConflict,
    /// A project that holds a planning thread is not removed (§4.2); 409.
    ProjectHasThreads,
    /// An MCP call outside its grant's thread or project (§13.6). MCP tool
    /// results only: no HTTP route answers it.
    GrantScope,
    /// An MCP grant unknown, or revoked while its call was in flight (§13.7).
    /// MCP tool results only: no HTTP route answers it.
    GrantInvalid,
}

/// The one failure a service returns (spec §14.4), in words no adapter owns.
/// Each adapter maps it to its own shape — `Failure` for HTTP, `Refusal` for
/// MCP — and the codes and texts clients see stay exactly what they were.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Storage(StorageError),
    #[error(transparent)]
    Start(#[from] StartError),
    #[error(transparent)]
    Directory(#[from] DirectoryError),
    /// The project's directory cannot be run in; the text is the reason.
    #[error("{0}")]
    ProjectDirectoryUnusable(String),
    #[error("the harness could not start: {0}")]
    HarnessStartFailed(String),
    #[error("the {0} harness is not available yet")]
    HarnessUnavailable(String),
    /// `detail`, when given, is the harness's own refusal (§12.4).
    #[error("{what} {id} is not offered")]
    SettingNotOffered {
        what: String,
        id: String,
        detail: Option<String>,
    },
    #[error("this project does not allow the {0} mode")]
    ModeNotAllowed(String),
    #[error("the daemon is stopping")]
    RuntimeStopping,
    #[error("the turn's process tree could not be terminated")]
    TerminationFailed,
    /// A refusal whose code and exact text a service writes, as the MCP tools do today.
    #[error("{message}")]
    Refused { code: ErrorCode, message: String },
}

impl From<StorageError> for CoreError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::PlanArchived(_) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message: error.to_string(),
            },
            other => Self::Storage(other),
        }
    }
}

impl From<OpenError> for CoreError {
    fn from(e: OpenError) -> Self {
        match e {
            OpenError::Storage(e) => CoreError::Storage(e),
            OpenError::Start(reason) => CoreError::HarnessStartFailed(reason),
            OpenError::Workspace(reason) => CoreError::ProjectDirectoryUnusable(reason),
        }
    }
}
