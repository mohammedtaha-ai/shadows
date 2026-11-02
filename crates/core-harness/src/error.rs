//! Refusals and failures owned by thread harness sessions.

use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("permission request is unknown")]
    PermissionNotFound,
    #[error("permission request is settled")]
    PermissionSettled,
    #[error("permission option was not offered")]
    PermissionOptionNotOffered,
    #[error("AYLUL agent not found: {0}")]
    AylulProfileNotFound(String),
    #[error("AYLUL is not configured")]
    AylulNotConfigured,
    #[error("AYLUL key is missing: {0}")]
    AylulKeyMissing(String),
    #[error("encrypted environment storage is unavailable")]
    SecretsUnavailable,
    #[error("invalid profile: {0}")]
    ProfileInvalid(String),
    #[error("reserved environment name: {0}")]
    SecretNameReserved(String),
    #[error("the harness could not start: {0}")]
    HarnessStartFailed(String),
    #[error("{0}")]
    ProjectDirectoryUnusable(String),
    #[error("the {0} harness is not available yet")]
    HarnessUnavailable(String),
    #[error("{what} {id} is not offered")]
    SettingNotOffered {
        what: String,
        id: String,
        detail: Option<String>,
    },
    #[error("this project does not allow the {0} mode")]
    ModeNotAllowed(String),
    #[error("the thread has a turn running")]
    ThreadBusy,
    #[error("{0}")]
    ImagesUnsupported(String),
    #[error(transparent)]
    Agents(#[from] core_agents::AgentsError),
    #[error(transparent)]
    Extensions(#[from] core_extensions::ExtensionsError),
    #[error(transparent)]
    Grants(#[from] core_grants::GrantsError),
    #[error(transparent)]
    Instructions(#[from] core_instructions::InstructionsError),
    #[error(transparent)]
    Operations(#[from] core_operations::OperationsError),
    #[error(transparent)]
    Person(#[from] core_person::PersonError),
    #[error(transparent)]
    Projects(#[from] core_projects::ProjectsError),
    #[error(transparent)]
    Providers(#[from] core_providers::ProvidersError),
    #[error(transparent)]
    Secrets(#[from] core_secrets::SecretsError),
    #[error(transparent)]
    Threads(#[from] core_threads::ThreadsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<sqlx::Error> for HarnessError {
    fn from(error: sqlx::Error) -> Self {
        StorageError::from(error).into()
    }
}

impl From<serde_json::Error> for HarnessError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::from(error).into()
    }
}

impl From<crate::OpenError> for HarnessError {
    fn from(error: crate::OpenError) -> Self {
        match error {
            crate::OpenError::Env(error) => error,
            crate::OpenError::Secrets(error) => error.into(),
            crate::OpenError::Provider(error) => error.into(),
            crate::OpenError::Person(error) => error.into(),
            crate::OpenError::Agent(error) => error.into(),
            crate::OpenError::Storage(error) => error.into(),
            crate::OpenError::Start(reason) => Self::HarnessStartFailed(reason),
            crate::OpenError::Workspace(reason) => Self::ProjectDirectoryUnusable(reason),
        }
    }
}
