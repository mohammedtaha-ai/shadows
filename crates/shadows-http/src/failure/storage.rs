//! One job: how a storage failure becomes a status code.

use axum::http::StatusCode;

use shadows_core::ErrorCode;
use shadows_core::StorageError;

use super::{Detail, Failure};

impl From<StorageError> for Failure {
    fn from(e: StorageError) -> Self {
        // These two texts are written in `StorageError` itself and name
        // nothing but the command, or the kind of thing that is missing.
        let own = |status, code| Failure {
            status,
            code,
            message: e.to_string(),
            cause: None,
            detail: Detail::default(),
        };
        let (status, code, message) = match &e {
            StorageError::CommandConflict => {
                return own(StatusCode::CONFLICT, ErrorCode::CommandConflict);
            }
            StorageError::NotFound(_) => {
                return own(StatusCode::NOT_FOUND, ErrorCode::InvalidCommand);
            }
            StorageError::HarnessLocked => {
                return own(StatusCode::CONFLICT, ErrorCode::HarnessLocked);
            }
            StorageError::ThreadBusy => return own(StatusCode::CONFLICT, ErrorCode::ThreadBusy),
            StorageError::QueuedMessageGone => {
                return own(StatusCode::NOT_FOUND, ErrorCode::QueuedMessageGone);
            }
            StorageError::ProjectHasThreads => {
                return own(StatusCode::CONFLICT, ErrorCode::ProjectHasThreads);
            }
            StorageError::WorkflowFrozen => {
                return own(StatusCode::CONFLICT, ErrorCode::WorkflowFrozenImmutable);
            }
            StorageError::RevisionConflict { current, summary } => {
                return Failure {
                    status: StatusCode::CONFLICT,
                    code: ErrorCode::RevisionConflict,
                    message: format!(
                        "the resource changed; current revision is {current}: {summary}"
                    ),
                    cause: None,
                    detail: Detail {
                        current_revision: Some(*current),
                        problems: None,
                    },
                };
            }
            StorageError::PlanInvalid(problems) => {
                let problems: Vec<String> = problems.iter().map(|p| p.message.clone()).collect();
                return Failure {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    code: ErrorCode::WorkflowValidationFailed,
                    message: problems.join("; "),
                    cause: None,
                    detail: Detail {
                        current_revision: None,
                        problems: Some(problems),
                    },
                };
            }
            // A grant refusal answers an MCP tool call (§13.10); a route has
            // no grant, so one reaching here is the daemon's own defect.
            StorageError::GrantInvalid | StorageError::GrantScope => {
                tracing::error!(error = %e, "http.grant_refusal_on_route");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::StorageUnavailable,
                    "the daemon refused its own request",
                )
            }
            StorageError::TaskNotInPlan(_) => {
                return own(StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::InvalidCommand);
            }
            StorageError::ForkPointNotSupported => {
                return own(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    ErrorCode::ForkPointNotSupported,
                );
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
            detail: Detail::default(),
        }
    }
}
