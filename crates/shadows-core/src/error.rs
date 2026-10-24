use crate::harness::OpenError;
use crate::turns::StartError;
use core_db::StorageError;
use core_projects::DirectoryError;

/// Stable codes clients pattern-match on. Never match on human text.
/// Spec §3.4. `Blocked`/`Rejected` are domain outcomes and never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ExtensionNotFound,
    ExtensionNameConflict,
    /// Desktop identity was not configured; HTTP readiness only.
    RuntimeNotConfigured,
    AylulProfileNotFound,
    AylulProviderNotFound,
    ProviderInUse,
    ProfileInvalid,
    SecretNameReserved,
    SecretNameInvalid,
    SecretValueInvalid,
    SecretUnreadable,
    SecretsUnavailable,
    AylulKeyMissing,
    AylulNotConfigured,
    StageBlocked,
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
    /// The waiting message was already sent or removed (§20.5); 404.
    QueuedMessageGone,
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
    #[error("{message}")]
    ValidationFailed {
        message: String,
        problems: Vec<String>,
    },
    #[error("a new version needs its reason")]
    ReasonMissing,
    #[error("{0}")]
    Unavailable(String),
    #[error("extension not found")]
    ExtensionNotFound,
    #[error("extension name is already in use")]
    ExtensionNameConflict,
    #[error("AYLUL agent not found: {0}")]
    AylulProfileNotFound(String),
    #[error("AYLUL provider not found: {0}")]
    AylulProviderNotFound(String),
    #[error("provider is used by: {0}")]
    ProviderInUse(String),
    #[error("invalid profile: {0}")]
    ProfileInvalid(String),
    #[error("reserved environment name: {0}")]
    SecretNameReserved(String),
    #[error("invalid environment variable name: {0}")]
    SecretNameInvalid(String),
    #[error("invalid environment variable value for {0}")]
    SecretValueInvalid(String),
    #[error("cannot decrypt {0}; replace it in Settings")]
    SecretUnreadable(String),
    #[error("encrypted environment storage is unavailable")]
    SecretsUnavailable,
    #[error("AYLUL key is missing: {0}")]
    AylulKeyMissing(String),
    #[error("AYLUL is not configured")]
    AylulNotConfigured,
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
        Self::Storage(error)
    }
}

impl From<OpenError> for CoreError {
    fn from(e: OpenError) -> Self {
        match e {
            OpenError::Env(e) => e,
            OpenError::Secrets(error) => error.into(),
            OpenError::Provider(error) => error.into(),
            OpenError::Person(error) => error.into(),
            OpenError::Storage(e) => CoreError::Storage(e),
            OpenError::Start(reason) => CoreError::HarnessStartFailed(reason),
            OpenError::Workspace(reason) => CoreError::ProjectDirectoryUnusable(reason),
        }
    }
}

impl From<sqlx::Error> for CoreError {
    fn from(error: sqlx::Error) -> Self {
        StorageError::from(error).into()
    }
}
impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::from(error).into()
    }
}

impl CoreError {
    pub(crate) fn invalid_constraint(error: impl Into<Self>) -> Self {
        match error.into() {
            Self::Storage(StorageError::Constraint(message)) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            other => other,
        }
    }
}

impl CoreError {
    /// Classifies a failure independently of its transport envelope.
    pub fn code(&self) -> ErrorCode {
        let e = self;
        match e {
            CoreError::ValidationFailed { .. } => ErrorCode::WorkflowValidationFailed,
            CoreError::ReasonMissing | CoreError::Unavailable(_) => ErrorCode::StorageUnavailable,
            CoreError::ExtensionNotFound => ErrorCode::ExtensionNotFound,
            CoreError::ExtensionNameConflict => ErrorCode::ExtensionNameConflict,
            CoreError::AylulProfileNotFound(_) => ErrorCode::AylulProfileNotFound,
            CoreError::AylulProviderNotFound(_) => ErrorCode::AylulProviderNotFound,
            CoreError::ProviderInUse(_) => ErrorCode::ProviderInUse,
            CoreError::ProfileInvalid(_) => ErrorCode::ProfileInvalid,
            CoreError::SecretNameReserved(_) => ErrorCode::SecretNameReserved,
            CoreError::SecretNameInvalid(_) => ErrorCode::SecretNameInvalid,
            CoreError::SecretValueInvalid(_) => ErrorCode::SecretValueInvalid,
            CoreError::SecretUnreadable(_) => ErrorCode::SecretUnreadable,
            CoreError::SecretsUnavailable => ErrorCode::SecretsUnavailable,
            CoreError::AylulKeyMissing(_) => ErrorCode::AylulKeyMissing,
            CoreError::AylulNotConfigured => ErrorCode::AylulNotConfigured,
            CoreError::Storage(error) | CoreError::Start(StartError::Storage(error)) => match error
            {
                StorageError::CommandConflict => ErrorCode::CommandConflict,
                StorageError::NotFound(_) => ErrorCode::InvalidCommand,
                StorageError::RevisionConflict { .. } => ErrorCode::RevisionConflict,
                StorageError::Constraint(_) | StorageError::TransitionConflict { .. } => {
                    ErrorCode::StorageConstraintViolation
                }
                StorageError::Unavailable(_)
                | StorageError::Migration(_)
                | StorageError::Json(_)
                | StorageError::Database(_) => ErrorCode::StorageUnavailable,
            },
            CoreError::Refused { code, .. } => *code,
            CoreError::Start(StartError::RuntimeStopping) | CoreError::RuntimeStopping => {
                ErrorCode::RuntimeStopping
            }
            CoreError::Directory(d) => Self::directory_code(d),
            CoreError::ProjectDirectoryUnusable(_) => ErrorCode::PathNotFound,
            CoreError::HarnessStartFailed(_) => ErrorCode::HarnessStartFailed,
            CoreError::HarnessUnavailable(_) => ErrorCode::HarnessUnavailable,
            CoreError::SettingNotOffered { .. } => ErrorCode::SettingNotOffered,
            CoreError::ModeNotAllowed(_) => ErrorCode::ModeNotAllowed,
            CoreError::TerminationFailed => ErrorCode::ProcessTerminationFailed,
        }
    }

    fn directory_code(e: &DirectoryError) -> ErrorCode {
        match e {
            DirectoryError::NotAbsolute(_)
            | DirectoryError::NotUtf8
            | DirectoryError::InvalidName { .. } => ErrorCode::PathInvalid,
            DirectoryError::AlreadyExists(_) => ErrorCode::PathAlreadyExists,
            DirectoryError::NotFound(_) => ErrorCode::PathNotFound,
            DirectoryError::NotADirectory(_) => ErrorCode::PathNotADirectory,
            DirectoryError::AccessDenied(_) => ErrorCode::PathAccessDenied,
            DirectoryError::Unavailable { .. } => ErrorCode::PathUnavailable,
        }
    }
}

impl From<core_secrets::SecretsError> for CoreError {
    fn from(error: core_secrets::SecretsError) -> Self {
        use core_secrets::SecretsError;
        match error {
            SecretsError::NameReserved(name) => Self::SecretNameReserved(name),
            SecretsError::NameInvalid(name) => Self::SecretNameInvalid(name),
            SecretsError::ValueInvalid(name) => Self::SecretValueInvalid(name),
            SecretsError::Unreadable(name) => Self::SecretUnreadable(name),
            SecretsError::Unavailable => Self::SecretsUnavailable,
            SecretsError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_providers::ProvidersError> for CoreError {
    fn from(error: core_providers::ProvidersError) -> Self {
        use core_providers::ProvidersError;
        match error {
            ProvidersError::NotFound(slug) => Self::AylulProviderNotFound(slug),
            ProvidersError::InvalidProfile(message) => Self::ProfileInvalid(message),
            ProvidersError::KeyMissing(slug) => Self::AylulKeyMissing(slug),
            ProvidersError::Secrets(error) => error.into(),
            ProvidersError::Storage(error) => error.into(),
        }
    }
}

impl From<core_projects::ProjectsError> for CoreError {
    fn from(error: core_projects::ProjectsError) -> Self {
        use core_projects::ProjectsError;
        match error {
            ProjectsError::Directory(error) => Self::Directory(error),
            ProjectsError::SettingNotOffered { what, id, detail } => {
                Self::SettingNotOffered { what, id, detail }
            }
            ProjectsError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_usage::UsageError> for CoreError {
    fn from(error: core_usage::UsageError) -> Self {
        match error {
            core_usage::UsageError::InvalidPeriod => {
                Self::ProfileInvalid("invalid usage period".into())
            }
            core_usage::UsageError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_person::PersonError> for CoreError {
    fn from(error: core_person::PersonError) -> Self {
        use core_person::PersonError;
        match error {
            PersonError::InvalidName => Self::ProfileInvalid("invalid person name".into()),
            PersonError::NameTooLong => Self::ProfileInvalid("person name is too long".into()),
            PersonError::InvalidLanguage => Self::ProfileInvalid("invalid language tag".into()),
            PersonError::Storage(error) => Self::Storage(error),
        }
    }
}
