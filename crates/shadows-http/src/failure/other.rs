//! One job: how a start, core or directory failure becomes a status code.

use axum::http::StatusCode;

use shadows_core::CoreError;
use shadows_core::DirectoryError;
use shadows_core::ErrorCode;
use shadows_core::StartError;

use super::{Detail, Failure};

impl From<StartError> for Failure {
    fn from(e: StartError) -> Self {
        match e {
            StartError::Storage(e) => e.into(),
            StartError::RuntimeStopping => Failure::runtime_stopping(),
        }
    }
}

impl From<CoreError> for Failure {
    /// Each variant through the mapping it had before services returned
    /// `CoreError`, so no status, code or text moves.
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::Storage(e) => e.into(),
            CoreError::Start(e) => e.into(),
            CoreError::Directory(e) => e.into(),
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
            CoreError::Refused { code, message } => Failure::refused(code, message),
        }
    }
}

impl From<DirectoryError> for Failure {
    fn from(e: DirectoryError) -> Self {
        let (status, code) = match &e {
            DirectoryError::NotAbsolute(_)
            | DirectoryError::NotUtf8
            | DirectoryError::InvalidName { .. } => {
                (StatusCode::BAD_REQUEST, ErrorCode::PathInvalid)
            }
            DirectoryError::AlreadyExists(_) => {
                (StatusCode::CONFLICT, ErrorCode::PathAlreadyExists)
            }
            DirectoryError::NotFound(_) => (StatusCode::NOT_FOUND, ErrorCode::PathNotFound),
            DirectoryError::NotADirectory(_) => {
                (StatusCode::BAD_REQUEST, ErrorCode::PathNotADirectory)
            }
            DirectoryError::AccessDenied(_) => (StatusCode::FORBIDDEN, ErrorCode::PathAccessDenied),
            DirectoryError::Unavailable { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::PathUnavailable,
            ),
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
