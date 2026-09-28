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

use shadows_core::CoreError;
use shadows_core::error::ErrorCode;
use shadows_core::planner::StartError;
use shadows_core::project::DirectoryError;
use shadows_core::storage::StorageError;

/// A failed request: its status and the body it answers with.
pub struct Failure {
    status: StatusCode,
    code: ErrorCode,
    message: String,
    /// The internal cause, logged with a 5xx and never sent (spec §3.2).
    cause: Option<String>,
    detail: Detail,
}

/// What some refusals carry besides their message, for a client to act on
/// without reading it (spec §13.10).
#[derive(Default)]
struct Detail {
    current_revision: Option<i64>,
    problems: Option<Vec<String>>,
}

/// The body of every error this API answers: the stable code a client
/// matches on (spec §3.4), and a human message nobody should match on. A
/// request axum rejects before a handler runs (malformed JSON, a missing query
/// parameter) gets one too, through
/// [`rejections_as_error_bodies`].
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    /// `REVISION_CONFLICT` only: the plan's revision now, to read again at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<i64>,
    /// `WORKFLOW_VALIDATION_FAILED` only: each problem, the same sentences
    /// `message` joins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problems: Option<Vec<String>>,
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
            StorageError::WorkflowFrozen => {
                return own(StatusCode::CONFLICT, ErrorCode::WorkflowFrozenImmutable);
            }
            StorageError::RevisionConflict { current, summary } => {
                return Failure {
                    status: StatusCode::CONFLICT,
                    code: ErrorCode::RevisionConflict,
                    message: format!("the plan changed; current revision is {current}: {summary}"),
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

impl From<StartError> for Failure {
    fn from(e: StartError) -> Self {
        match e {
            StartError::Storage(e) => e.into(),
            StartError::RuntimeStopping => Failure::runtime_stopping(),
        }
    }
}

impl Failure {
    /// The thread's project has no usable directory. A 4xx whose message is
    /// the reason, because the user can act on it: the text is ours, about a
    /// directory the user chose (§3.2).
    pub(super) fn project_directory_unusable(reason: String) -> Self {
        Failure {
            status: StatusCode::CONFLICT,
            code: ErrorCode::PathNotFound,
            message: reason,
            cause: None,
            detail: Detail::default(),
        }
    }
    pub(super) fn harness_start_failed(reason: String) -> Self {
        Failure {
            status: StatusCode::BAD_GATEWAY,
            code: ErrorCode::HarnessStartFailed,
            message: "the harness could not start".into(),
            cause: Some(reason),
            detail: Detail::default(),
        }
    }
    /// Spec §12.4: the thread's harness is listed but cannot run here yet.
    pub(super) fn harness_unavailable(harness: &str) -> Self {
        Failure {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: ErrorCode::HarnessUnavailable,
            message: format!("the {harness} harness is not available yet"),
            cause: None,
            detail: Detail::default(),
        }
    }

    /// Spec §12.7: `what` (model, effort, mode or harness) is not among the
    /// choices on offer. `detail`, when given, is the harness's own refusal,
    /// which names a setting the user chose (§12.4: reported in its words).
    pub(super) fn setting_not_offered(what: &str, id: &str, detail: Option<&str>) -> Self {
        let message = match detail {
            Some(detail) => format!("{what} {id} was refused by the harness: {detail}"),
            None => format!("{what} {id} is not offered"),
        };
        Failure {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: ErrorCode::SettingNotOffered,
            message,
            cause: None,
            detail: Detail::default(),
        }
    }

    /// Spec §12.5: the project does not allow `mode`.
    pub(super) fn mode_not_allowed(mode: &str) -> Self {
        Failure {
            status: StatusCode::FORBIDDEN,
            code: ErrorCode::ModeNotAllowed,
            message: format!("this project does not allow the {mode} mode"),
            cause: None,
            detail: Detail::default(),
        }
    }

    /// Spec §8.5: a stopping daemon takes no new work. 503, because the
    /// refusal is about this daemon's state, not the request — the same
    /// request succeeds against the next one.
    pub(super) fn runtime_stopping() -> Self {
        Failure {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: ErrorCode::RuntimeStopping,
            message: StartError::RuntimeStopping.to_string(),
            cause: None,
            detail: Detail::default(),
        }
    }

    /// A refusal whose code and text a service wrote. 422: the request was
    /// understood and refused. No HTTP route produces one in Milestone 2.5.
    pub(super) fn refused(code: ErrorCode, message: String) -> Self {
        Failure {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code,
            message,
            cause: None,
            detail: Detail::default(),
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
            detail: Detail::default(),
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
            detail: Detail::default(),
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
                current_revision: self.detail.current_revision,
                problems: self.detail.problems,
            }),
        )
            .into_response()
    }
}

/// Axum answers a request its extractors refuse — a body that is not the JSON
/// a route takes, a query or path that does not parse — before any handler
/// runs, in plain text. Every such answer is a 4xx `text/plain`, and no
/// handler here answers one, so this rewrites exactly those into an
/// [`ErrorBody`] with `INVALID_COMMAND`: a client reads one error shape from
/// every route. Axum's text names only the field and the reason, which is safe
/// to pass on.
pub(super) async fn rejections_as_error_bodies(
    response: axum::response::Response,
) -> axum::response::Response {
    use axum::http::header::CONTENT_TYPE;
    let plain = response
        .headers()
        .get(CONTENT_TYPE)
        .is_some_and(|v| v.as_bytes().starts_with(b"text/plain"));
    if !response.status().is_client_error() || !plain {
        return response;
    }
    let status = response.status();
    let text = match axum::body::to_bytes(response.into_body(), 64 * 1024).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    };
    let message = if text.is_empty() {
        status
            .canonical_reason()
            .unwrap_or("the request was refused")
            .to_string()
    } else {
        text
    };
    axum::response::IntoResponse::into_response((
        status,
        Json(ErrorBody {
            code: ErrorCode::InvalidCommand,
            message,
            current_revision: None,
            problems: None,
        }),
    ))
}
