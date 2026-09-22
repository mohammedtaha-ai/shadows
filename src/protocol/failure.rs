//! One job: how a failure becomes a status code.
//!
//! Transport mapping lives here and nowhere else. Spec §3.3: `Blocked` and
//! `Rejected` are domain outcomes, not HTTP failures, and would be returned as
//! 200 with the outcome — they are not reachable in Milestone 0.

use axum::Json;
use axum::http::StatusCode;

use crate::error::ErrorCode;
use crate::project::DirectoryError;
use crate::storage::StorageError;

/// A failed request: its status, the stable code a client matches on (spec
/// §3.4), and a human message nobody should match on.
pub struct Failure {
    status: StatusCode,
    code: ErrorCode,
    message: String,
}

impl From<StorageError> for Failure {
    fn from(e: StorageError) -> Self {
        let (status, code) = match &e {
            StorageError::CommandConflict => (StatusCode::CONFLICT, ErrorCode::CommandConflict),
            StorageError::NotFound(_) => (StatusCode::NOT_FOUND, ErrorCode::InvalidCommand),
            StorageError::TransitionConflict { .. } => {
                (StatusCode::CONFLICT, ErrorCode::StorageConstraintViolation)
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::StorageUnavailable,
            ),
        };
        Failure {
            status,
            code,
            message: e.to_string(),
        }
    }
}

impl From<DirectoryError> for Failure {
    fn from(e: DirectoryError) -> Self {
        let (status, code) = match &e {
            DirectoryError::NotAbsolute(_) | DirectoryError::NotUtf8 => {
                (StatusCode::BAD_REQUEST, ErrorCode::PathInvalid)
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
        }
    }
}

impl axum::response::IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        if self.status.is_server_error() {
            // Inside the request's `http` span, so the line names the route.
            tracing::error!(error = %self.message, "http.failure");
        }
        (
            self.status,
            Json(serde_json::json!({ "code": self.code, "message": self.message })),
        )
            .into_response()
    }
}
