use core_db::StorageError;
use core_harness::OpenError;
use core_projects::DirectoryError;
use core_turns::StartError;

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
            OpenError::Env(e) => e.into(),
            OpenError::Secrets(error) => error.into(),
            OpenError::Provider(error) => error.into(),
            OpenError::Person(error) => error.into(),
            OpenError::Agent(error) => error.into(),
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

impl From<core_instructions::InstructionsError> for CoreError {
    fn from(error: core_instructions::InstructionsError) -> Self {
        match error {
            core_instructions::InstructionsError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_standards::StandardsError> for CoreError {
    fn from(error: core_standards::StandardsError) -> Self {
        match error {
            core_standards::StandardsError::InvalidAdditions(message) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            core_standards::StandardsError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_code::CodeError> for CoreError {
    fn from(error: core_code::CodeError) -> Self {
        use core_code::CodeError;
        match error {
            CodeError::InvalidCommand(message) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            CodeError::GrantScope(message) => Self::Refused {
                code: ErrorCode::GrantScope,
                message,
            },
            CodeError::Projects(error) => error.into(),
            CodeError::Storage(error) => Self::Storage(error),
        }
    }
}

impl From<core_agents::AgentsError> for CoreError {
    fn from(error: core_agents::AgentsError) -> Self {
        use core_agents::AgentsError;
        match error {
            AgentsError::NotFound(slug) => Self::AylulProfileNotFound(slug),
            AgentsError::ProviderInUse(names) => Self::ProviderInUse(names),
            AgentsError::Invalid(reason) => Self::ProfileInvalid(reason),
            AgentsError::NotConfigured => Self::AylulNotConfigured,
            AgentsError::Provider(error) => error.into(),
            AgentsError::Secrets(error) => error.into(),
            AgentsError::Projects(error) => error.into(),
            AgentsError::Storage(error) => error.into(),
        }
    }
}

impl From<core_extensions::ExtensionsError> for CoreError {
    fn from(error: core_extensions::ExtensionsError) -> Self {
        use core_extensions::ExtensionsError;
        match error {
            ExtensionsError::NotFound => Self::ExtensionNotFound,
            ExtensionsError::NameConflict => Self::ExtensionNameConflict,
            ExtensionsError::Invalid(reason) => Self::ProfileInvalid(reason),
            ExtensionsError::NotConfigured => Self::AylulNotConfigured,
            ExtensionsError::Agent(error) => error.into(),
            ExtensionsError::Storage(error) => error.into(),
        }
    }
}

impl From<core_threads::ThreadsError> for CoreError {
    fn from(error: core_threads::ThreadsError) -> Self {
        use core_threads::ThreadsError;
        let code = match error {
            ThreadsError::HarnessLocked => ErrorCode::HarnessLocked,
            ThreadsError::ForkPointNotSupported => ErrorCode::ForkPointNotSupported,
            ThreadsError::InvalidCommand(message) => {
                return Self::Refused {
                    code: ErrorCode::InvalidCommand,
                    message,
                };
            }
            ThreadsError::InvalidProfile(message) => return Self::ProfileInvalid(message),
            ThreadsError::Projects(error) => return error.into(),
            ThreadsError::Agent(error) => return error.into(),
            ThreadsError::Storage(error) => return error.into(),
        };
        Self::Refused {
            code,
            message: error.to_string(),
        }
    }
}

impl From<core_grants::GrantsError> for CoreError {
    fn from(error: core_grants::GrantsError) -> Self {
        use core_grants::GrantsError;
        let code = match error {
            GrantsError::GrantInvalid => ErrorCode::GrantInvalid,
            GrantsError::GrantScope => ErrorCode::GrantScope,
            GrantsError::GrantScopeMessage(message) => {
                return Self::Refused {
                    code: ErrorCode::GrantScope,
                    message,
                };
            }
            GrantsError::Storage(error) => return error.into(),
        };
        Self::Refused {
            code,
            message: error.to_string(),
        }
    }
}

impl From<core_operations::OperationsError> for CoreError {
    fn from(error: core_operations::OperationsError) -> Self {
        use core_operations::OperationsError;
        match error {
            OperationsError::ThreadBusy => Self::Refused {
                code: ErrorCode::ThreadBusy,
                message: "the thread has a turn running".into(),
            },
            OperationsError::Threads(error) => error.into(),
            OperationsError::Storage(error) => error.into(),
        }
    }
}

impl From<core_harness::HarnessError> for CoreError {
    fn from(error: core_harness::HarnessError) -> Self {
        use core_harness::HarnessError;
        match error {
            HarnessError::AylulProfileNotFound(value) => Self::AylulProfileNotFound(value),
            HarnessError::AylulKeyMissing(value) => Self::AylulKeyMissing(value),
            HarnessError::ProfileInvalid(value) => Self::ProfileInvalid(value),
            HarnessError::SecretNameReserved(value) => Self::SecretNameReserved(value),
            HarnessError::HarnessStartFailed(value) => Self::HarnessStartFailed(value),
            HarnessError::ProjectDirectoryUnusable(value) => Self::ProjectDirectoryUnusable(value),
            HarnessError::HarnessUnavailable(value) => Self::HarnessUnavailable(value),
            HarnessError::ModeNotAllowed(value) => Self::ModeNotAllowed(value),
            HarnessError::AylulNotConfigured => Self::AylulNotConfigured,
            HarnessError::SecretsUnavailable => Self::SecretsUnavailable,
            HarnessError::SettingNotOffered { what, id, detail } => {
                Self::SettingNotOffered { what, id, detail }
            }
            HarnessError::ThreadBusy => Self::Refused {
                code: ErrorCode::ThreadBusy,
                message: "the thread has a turn running".into(),
            },
            HarnessError::ImagesUnsupported(message) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            HarnessError::Agents(error) => error.into(),
            HarnessError::Extensions(error) => error.into(),
            HarnessError::Grants(error) => error.into(),
            HarnessError::Instructions(error) => error.into(),
            HarnessError::Operations(error) => error.into(),
            HarnessError::Person(error) => error.into(),
            HarnessError::Projects(error) => error.into(),
            HarnessError::Providers(error) => error.into(),
            HarnessError::Secrets(error) => error.into(),
            HarnessError::Threads(error) => error.into(),
            HarnessError::Storage(error) => error.into(),
        }
    }
}

impl From<core_design::DesignError> for CoreError {
    fn from(error: core_design::DesignError) -> Self {
        use core_design::DesignError;
        match error {
            DesignError::StageBlocked(message) => Self::Refused {
                code: ErrorCode::StageBlocked,
                message,
            },
            DesignError::InvalidCommand(message) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            DesignError::GrantScope(message) => Self::Refused {
                code: ErrorCode::GrantScope,
                message,
            },
            DesignError::ReasonMissing => Self::ReasonMissing,
            DesignError::Grants(error) => error.into(),
            DesignError::Storage(error) => error.into(),
        }
    }
}

impl From<core_plans::PlansError> for CoreError {
    fn from(error: core_plans::PlansError) -> Self {
        use core_plans::PlansError;
        let message = error.to_string();
        match error {
            PlansError::PlanInvalid(problems) => Self::ValidationFailed {
                message,
                problems: problems.into_iter().map(|p| p.message).collect(),
            },
            PlansError::ReasonMissing => Self::ReasonMissing,
            PlansError::NotLatestVersion(_) => Self::Unavailable(message),
            PlansError::WorkflowFrozen => Self::Refused {
                code: ErrorCode::WorkflowFrozenImmutable,
                message,
            },
            PlansError::GrantScope(message) => Self::Refused {
                code: ErrorCode::GrantScope,
                message,
            },
            PlansError::GrantInvalid => Self::Refused {
                code: ErrorCode::GrantInvalid,
                message,
            },
            PlansError::Design(error) => error.into(),
            PlansError::Projects(error) => error.into(),
            PlansError::Storage(error) => error.into(),
            PlansError::PlanLinkInvalid(_)
            | PlansError::TaskNotInPlan(_)
            | PlansError::PlanArchived(_)
            | PlansError::InvalidCommand(_) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
        }
    }
}

impl From<core_turns::TurnsError> for CoreError {
    fn from(error: core_turns::TurnsError) -> Self {
        use core_turns::TurnsError;
        let message = error.to_string();
        match error {
            TurnsError::ThreadBusy => Self::Refused {
                code: ErrorCode::ThreadBusy,
                message,
            },
            TurnsError::QueuedMessageGone => Self::Refused {
                code: ErrorCode::QueuedMessageGone,
                message,
            },
            TurnsError::RuntimeStopping => Self::RuntimeStopping,
            TurnsError::TerminationFailed => Self::TerminationFailed,
            TurnsError::InvalidCommand(message) => Self::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            TurnsError::HarnessStartFailed(message) => Self::HarnessStartFailed(message),
            TurnsError::SettingNotOffered { what, id, detail } => {
                Self::SettingNotOffered { what, id, detail }
            }
            TurnsError::ModeNotAllowed(mode) => Self::ModeNotAllowed(mode),
            TurnsError::Start(error) => error.into(),
            TurnsError::Harness(error) => error.into(),
            TurnsError::Plans(error) => error.into(),
            TurnsError::Projects(error) => error.into(),
            TurnsError::Threads(error) => error.into(),
            TurnsError::Instructions(error) => error.into(),
            TurnsError::Standards(error) => error.into(),
            TurnsError::Storage(error) => error.into(),
        }
    }
}

impl From<core_project_lifecycle::ProjectLifecycleError> for CoreError {
    fn from(error: core_project_lifecycle::ProjectLifecycleError) -> Self {
        match error {
            core_project_lifecycle::ProjectLifecycleError::ProjectHasThreads => Self::Refused {
                code: ErrorCode::ProjectHasThreads,
                message: "the project has planning threads; it cannot be removed".into(),
            },
            core_project_lifecycle::ProjectLifecycleError::Projects(error) => error.into(),
            core_project_lifecycle::ProjectLifecycleError::Storage(error) => error.into(),
        }
    }
}
