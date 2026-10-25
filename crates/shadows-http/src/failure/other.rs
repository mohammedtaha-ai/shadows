//! One job: how a start, core or directory failure becomes a status code.

use axum::http::StatusCode;

use shadows_core::CoreError;
use shadows_core::DirectoryError;
use shadows_core::ErrorCode;
use shadows_core::StartError;

use super::{Detail, Failure};

impl From<StartError> for Failure {
    fn from(e: StartError) -> Self {
        CoreError::Start(e).into()
    }
}

impl From<CoreError> for Failure {
    fn from(e: CoreError) -> Self {
        let code = e.code();
        let mut failure = match e {
            CoreError::ValidationFailed { problems, .. } => Failure {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: ErrorCode::WorkflowValidationFailed,
                message: problems.join("; "),
                cause: None,
                detail: Detail {
                    current_revision: None,
                    problems: Some(problems),
                },
            },
            CoreError::ReasonMissing | CoreError::Unavailable(_) => Failure {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: ErrorCode::StorageUnavailable,
                message: "storage is unavailable".into(),
                cause: Some(e.to_string()),
                detail: Detail::default(),
            },
            CoreError::ExtensionNotFound
            | CoreError::ExtensionNameConflict
            | CoreError::AylulProfileNotFound(_)
            | CoreError::AylulProviderNotFound(_)
            | CoreError::ProviderInUse(_)
            | CoreError::ProfileInvalid(_)
            | CoreError::SecretNameReserved(_)
            | CoreError::SecretNameInvalid(_)
            | CoreError::SecretValueInvalid(_)
            | CoreError::SecretUnreadable(_)
            | CoreError::SecretsUnavailable
            | CoreError::AylulKeyMissing(_)
            | CoreError::AylulNotConfigured => {
                let status = match code {
                    ErrorCode::ExtensionNotFound
                    | ErrorCode::AylulProfileNotFound
                    | ErrorCode::AylulProviderNotFound
                    | ErrorCode::AylulNotConfigured => StatusCode::NOT_FOUND,
                    ErrorCode::ExtensionNameConflict
                    | ErrorCode::ProviderInUse
                    | ErrorCode::SecretNameReserved
                    | ErrorCode::SecretUnreadable
                    | ErrorCode::AylulKeyMissing => StatusCode::CONFLICT,
                    ErrorCode::ProfileInvalid
                    | ErrorCode::SecretNameInvalid
                    | ErrorCode::SecretValueInvalid => StatusCode::BAD_REQUEST,
                    ErrorCode::SecretsUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                    _ => unreachable!("only configuration errors enter this branch"),
                };
                Failure {
                    status,
                    code,
                    message: e.to_string(),
                    cause: None,
                    detail: Detail::default(),
                }
            }
            CoreError::Storage(e) | CoreError::Start(StartError::Storage(e)) => {
                Failure::storage(e, code)
            }
            CoreError::Start(StartError::RuntimeStopping) => Failure::runtime_stopping(),
            CoreError::Directory(e) => Failure::directory(e, code),
            CoreError::ProjectDirectoryUnusable(reason) => {
                Failure::project_directory_unusable(reason)
            }
            CoreError::HarnessStartFailed(reason) => Failure::harness_start_failed(reason),
            CoreError::HarnessUnavailable(harness) => Failure::harness_unavailable(&harness),
            CoreError::SettingNotOffered { what, id, detail } => {
                Failure::setting_not_offered(&what, &id, detail.as_deref())
            }
            CoreError::ModeNotAllowed(mode) => Failure::mode_not_allowed(&mode),
            CoreError::RuntimeStopping => Failure::runtime_stopping(),
            CoreError::TerminationFailed => Failure::termination_failed(),
            CoreError::Refused { code, message } => return Failure::refused(code, message),
        };
        failure.code = code;
        failure
    }
}

impl From<DirectoryError> for Failure {
    fn from(e: DirectoryError) -> Self {
        CoreError::Directory(e).into()
    }
}

impl Failure {
    fn directory(e: DirectoryError, code: ErrorCode) -> Self {
        let status = match code {
            ErrorCode::PathInvalid | ErrorCode::PathNotADirectory => StatusCode::BAD_REQUEST,
            ErrorCode::PathAlreadyExists => StatusCode::CONFLICT,
            ErrorCode::PathNotFound => StatusCode::NOT_FOUND,
            ErrorCode::PathAccessDenied => StatusCode::FORBIDDEN,
            ErrorCode::PathUnavailable => StatusCode::INTERNAL_SERVER_ERROR,
            _ => unreachable!("only directory errors enter this branch"),
        };
        Failure {
            status,
            code,
            message: e.to_string(),
            cause: None,
            detail: Detail::default(),
        }
    }
}

impl From<shadows_core::ProvidersError> for Failure {
    fn from(error: shadows_core::ProvidersError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::ProjectsError> for Failure {
    fn from(error: shadows_core::ProjectsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::UsageError> for Failure {
    fn from(error: shadows_core::UsageError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::PersonError> for Failure {
    fn from(error: shadows_core::PersonError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::InstructionsError> for Failure {
    fn from(error: shadows_core::InstructionsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::StandardsError> for Failure {
    fn from(error: shadows_core::StandardsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::CodeError> for Failure {
    fn from(error: shadows_core::CodeError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::AgentsError> for Failure {
    fn from(error: shadows_core::AgentsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::ExtensionsError> for Failure {
    fn from(error: shadows_core::ExtensionsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::ThreadsError> for Failure {
    fn from(error: shadows_core::ThreadsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::GrantsError> for Failure {
    fn from(error: shadows_core::GrantsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::OperationsError> for Failure {
    fn from(error: shadows_core::OperationsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::HarnessError> for Failure {
    fn from(error: shadows_core::HarnessError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::DesignError> for Failure {
    fn from(error: shadows_core::DesignError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::PlansError> for Failure {
    fn from(error: shadows_core::PlansError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::TurnsError> for Failure {
    fn from(error: shadows_core::TurnsError) -> Self {
        CoreError::from(error).into()
    }
}
