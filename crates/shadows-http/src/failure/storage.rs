//! One job: how a storage failure becomes a status code.

use axum::http::StatusCode;

use shadows_core::ErrorCode;
use shadows_core::StorageError;

use super::{Detail, Failure};

impl From<StorageError> for Failure {
    fn from(e: StorageError) -> Self {
        shadows_core::CoreError::Storage(e).into()
    }
}

impl Failure {
    pub(super) fn storage(e: StorageError, code: ErrorCode) -> Self {
        // These two texts are written in `StorageError` itself and name
        // nothing but the command, or the kind of thing that is missing.
        let own = |status| Failure {
            status,
            code,
            message: e.to_string(),
            cause: None,
            detail: Detail::default(),
        };
        let (status, message) = match &e {
            StorageError::CommandConflict => {
                return own(StatusCode::CONFLICT);
            }
            StorageError::NotFound(_) => {
                return own(StatusCode::NOT_FOUND);
            }
            StorageError::RevisionConflict { current, summary } => {
                return Failure {
                    status: StatusCode::CONFLICT,
                    code,
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
            StorageError::TransitionConflict { .. } => (
                StatusCode::CONFLICT,
                "the operation had already moved on; read it again",
            ),
            StorageError::Constraint(_) => (
                StatusCode::CONFLICT,
                "the request conflicts with what is already stored, such as a slug \
                 already in use",
            ),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "storage is unavailable"),
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
