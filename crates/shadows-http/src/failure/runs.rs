//! One job: how a run's failure becomes a status code.
//!
//! A run Core refused is 409 with Runs' own opaque code and the reason to
//! show, the same pair a task's `start` answer carries; a request naming
//! nothing is 404, and a failure to read or write is a 5xx `ErrorBody`.

use axum::Json;
use axum::http::StatusCode;

use shadows_core::{ErrorCode, RunsError, StorageError};

use super::{Detail, Failure};

/// A run Core refused. `code` is Core's opaque id (`waiting`, `task_busy`,
/// `overlap`, `not_git`, `dirty_scope`, `not_frozen`, `superseded`,
/// `archived`, `bad_scope`, `command_conflict`); `reason` is the text to show.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RunRefusal {
    pub code: String,
    pub reason: String,
}

/// What a run route answers when it does not answer its value.
pub enum RunFailure {
    Refused(RunRefusal),
    Other(Failure),
}

impl From<RunsError> for RunFailure {
    fn from(error: RunsError) -> Self {
        match error {
            RunsError::NotFound => Self::Other(StorageError::NotFound("task or attempt").into()),
            RunsError::Storage(error) => Self::Other(error.into()),
            RunsError::Grants(error) => Self::Other(error.into()),
            RunsError::Code(error) => Self::Other(error.into()),
            RunsError::Unavailable(cause) => Self::Other(Failure {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: ErrorCode::StorageUnavailable,
                message: "the run could not be read or written".into(),
                cause: Some(cause),
                detail: Detail::default(),
            }),
            other => Self::Refused(RunRefusal {
                code: other.code().to_string(),
                reason: other.to_string(),
            }),
        }
    }
}

impl axum::response::IntoResponse for RunFailure {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Refused(refusal) => (StatusCode::CONFLICT, Json(refusal)).into_response(),
            Self::Other(failure) => failure.into_response(),
        }
    }
}
