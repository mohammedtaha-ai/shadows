//! One job: how a failure becomes a status code.
//!
//! Transport mapping lives here and nowhere else. Spec §3.3: `Blocked` and
//! `Rejected` are domain outcomes, not HTTP failures, and would be returned as
//! 200 with the outcome — they are not reachable in Milestone 0.
//!
//! Spec §3.2: what reaches a client is public-safe. A storage failure's own
//! text names tables, columns, and driver internals, so a client is told what
//! happened in words written here, and the cause goes to the log only.

use axum::Json;
use axum::http::StatusCode;

use crate::error::ErrorCode;
use crate::planner::StartError;
use crate::project::DirectoryError;
use crate::storage::StorageError;

/// A failed request: its status and the body it answers with.
pub struct Failure {
    status: StatusCode,
    code: ErrorCode,
    message: String,
    /// The internal cause, logged with a 5xx and never sent (spec §3.2).
    cause: Option<String>,
}

/// The body of every error this API answers with itself: the stable code a
/// client matches on (spec §3.4), and a human message nobody should match on.
/// A request axum rejects before a handler runs (malformed JSON, a missing
/// query parameter) is answered by axum in plain text, not with this.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

impl From<StorageError> for Failure {
    fn from(e: StorageError) -> Self {
        // These two texts are written in `StorageError` itself and name
        // nothing but the command, or the kind of thing that is missing.
        let own = |status, code| Failure {
            status,
            code,
            message: e.to_string(),
            cause: None,
        };
        let (status, code, message) = match &e {
            StorageError::CommandConflict => {
                return own(StatusCode::CONFLICT, ErrorCode::CommandConflict);
            }
            StorageError::NotFound(_) => {
                return own(StatusCode::NOT_FOUND, ErrorCode::InvalidCommand);
            }
            StorageError::TransitionConflict { .. } => (
                StatusCode::CONFLICT,
                ErrorCode::StorageConstraintViolation,
                "the operation had already moved on; read it again",
            ),
            StorageError::Constraint(_) => (
                StatusCode::CONFLICT,
                ErrorCode::StorageConstraintViolation,
                "the request conflicts with what is already stored, such as a slug \
                 already in use",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::StorageUnavailable,
                "storage is unavailable",
            ),
        };
        Failure {
            status,
            code,
            message: message.into(),
            cause: Some(e.to_string()),
        }
    }
}

impl From<StartError> for Failure {
    fn from(e: StartError) -> Self {
        match e {
            StartError::Storage(e) => e.into(),
            StartError::RuntimeStopping => Failure::runtime_stopping(),
        }
    }
}

impl Failure {
    /// Spec §8.5: a stopping daemon takes no new work. 503, because the
    /// refusal is about this daemon's state, not the request — the same
    /// request succeeds against the next one.
    pub(super) fn runtime_stopping() -> Self {
        Failure {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: ErrorCode::RuntimeStopping,
            message: StartError::RuntimeStopping.to_string(),
            cause: None,
        }
    }

    /// Spec §1: a browser sent this on behalf of a page that is not one of
    /// this daemon's clients (`guard.rs`). 403: the request is refused for
    /// who sent it, whatever it asks.
    pub(super) fn origin_refused(why: &'static str) -> Self {
        Failure {
            status: StatusCode::FORBIDDEN,
            code: ErrorCode::OriginRefused,
            message: why.into(),
            cause: None,
        }
    }

    /// Spec §8.4 case 6: the tree could not be terminated. The daemon failed
    /// to do what Stop asks, so this is a 5xx, and the operation was not
    /// recorded `Cancelled` — it is still running as far as anyone can tell.
    pub(super) fn termination_failed() -> Self {
        Failure {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: ErrorCode::ProcessTerminationFailed,
            message: "the turn's process tree could not be terminated; it was not \
                      recorded as cancelled"
                .into(),
            cause: None,
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
        }
    }
}

impl axum::response::IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        if self.status.is_server_error() {
            // Inside the request's `http` span, so the line names the route.
            tracing::error!(
                error = %self.message,
                cause = self.cause.as_deref().unwrap_or_default(),
                "http.failure"
            );
        }
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}
