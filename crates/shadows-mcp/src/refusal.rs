//! One job: what a tool call answers (spec §13.6, the second error layer).
//!
//! Once a tool has started, it answers a result, never a JSON-RPC error: its
//! JSON as text (the code tools' own lines, as they are), or `isError` with
//! text that begins with the stable code a client matches on
//! (`REVISION_CONFLICT: …`) and goes on to say what to do.
//! A JSON-RPC error would reach the model as an opaque failure.

use rmcp::model::{CallToolResult, ContentBlock};

use shadows_core::CoreError;
use shadows_core::DirectoryError;
use shadows_core::ErrorCode;
use shadows_core::StartError;
use shadows_core::StorageError;

pub(super) struct Refusal {
    code: ErrorCode,
    message: String,
}

impl Refusal {
    pub(super) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// A plan, thread or ref outside what the grant reaches (§13.6).
    pub(super) fn scope(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::GrantScope, message)
    }
}

impl From<StorageError> for Refusal {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::RevisionConflict { current, summary } => Self::new(
                ErrorCode::RevisionConflict,
                format!(
                    "the plan changed; current revision is {current}: {summary}. \
                     Read it with workflow_get, then rebuild your edit."
                ),
            ),
            StorageError::WorkflowFrozen => Self::new(
                ErrorCode::WorkflowFrozenImmutable,
                "this plan version is approved and never changes again. \
                 Start a new version with draft_start.",
            ),
            StorageError::PlanInvalid(problems) => Self::new(
                ErrorCode::WorkflowValidationFailed,
                problems
                    .into_iter()
                    .map(|p| p.message)
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
            StorageError::GrantInvalid => Self::new(
                ErrorCode::GrantInvalid,
                "this grant was revoked while the call ran; nothing was changed",
            ),
            StorageError::GrantScope => Self::scope(
                "outside what this grant allows: plan writes stay in its own project; \
                 workflow_list and workflow_get can read an explicitly linked project; \
                 draft_ref starts a plan only within the hour after draft_prepare",
            ),
            StorageError::CommandConflict => Self::new(
                ErrorCode::CommandConflict,
                "this command id, or this draft_ref, already did something different; \
                 for a new plan call draft_prepare again",
            ),
            StorageError::TaskNotInPlan(message) => Self::new(ErrorCode::InvalidCommand, message),
            // A rule the request broke, not a broken database: say which, so an
            // agent fixes the request instead of asking for a restart.
            StorageError::Constraint(message) => Self::new(ErrorCode::InvalidCommand, message),
            StorageError::NotFound(what) => {
                Self::new(ErrorCode::InvalidCommand, format!("no such {what}"))
            }
            other => {
                tracing::error!(error = %other, "mcp.storage_failed");
                Self::new(
                    ErrorCode::StorageUnavailable,
                    "the daemon could not read or write its database",
                )
            }
        }
    }
}

impl From<CoreError> for Refusal {
    /// A storage failure keeps the text written above; a service's own refusal
    /// keeps its code and text; anything else, which no tool meets in this
    /// milestone, answers its code and its own words.
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::Storage(e) | CoreError::Start(StartError::Storage(e)) => e.into(),
            CoreError::Refused { code, message } => Self::new(code, message),
            other => Self::new(code_of(&other), other.to_string()),
        }
    }
}

/// The code HTTP answers the same failure with.
fn code_of(e: &CoreError) -> ErrorCode {
    match e {
        CoreError::Storage(_) | CoreError::Start(StartError::Storage(_)) => {
            ErrorCode::StorageUnavailable
        }
        CoreError::Refused { code, .. } => *code,
        CoreError::Start(StartError::RuntimeStopping) | CoreError::RuntimeStopping => {
            ErrorCode::RuntimeStopping
        }
        CoreError::Directory(d) => directory_code(d),
        CoreError::ProjectDirectoryUnusable(_) => ErrorCode::PathNotFound,
        CoreError::HarnessStartFailed(_) => ErrorCode::HarnessStartFailed,
        CoreError::HarnessUnavailable(_) => ErrorCode::HarnessUnavailable,
        CoreError::SettingNotOffered { .. } => ErrorCode::SettingNotOffered,
        CoreError::ModeNotAllowed(_) => ErrorCode::ModeNotAllowed,
        CoreError::TerminationFailed => ErrorCode::ProcessTerminationFailed,
    }
}

/// The code HTTP answers the same directory failure with.
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

/// The tool result for `outcome`: its JSON, or the refusal's text.
pub(super) fn answer<T: serde::Serialize>(outcome: Result<T, Refusal>) -> CallToolResult {
    let refusal = match outcome.map(|value| serde_json::to_string(&value)) {
        Ok(Ok(json)) => return CallToolResult::success(vec![ContentBlock::text(json)]),
        Ok(Err(e)) => Refusal::from(StorageError::Json(e)),
        Err(refusal) => refusal,
    };
    refused(refusal)
}

/// The tool result for `outcome`: its text as it is, or the refusal's text.
pub(super) fn text(outcome: Result<String, Refusal>) -> CallToolResult {
    match outcome {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(refusal) => refused(refusal),
    }
}

/// `isError`, with the code a client matches on, then what to do.
fn refused(refusal: Refusal) -> CallToolResult {
    let code = serde_json::to_value(refusal.code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{code}: {}",
        refusal.message
    ))])
}
