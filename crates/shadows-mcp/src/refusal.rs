//! One job: what a tool call answers (spec §13.6, the second error layer).
//!
//! Once a tool has started, it answers a result, never a JSON-RPC error: its
//! JSON as text (the code tools' own lines, as they are), or `isError` with
//! text that begins with the stable code a client matches on
//! (`REVISION_CONFLICT: …`) and goes on to say what to do.
//! A JSON-RPC error would reach the model as an opaque failure.

use rmcp::model::{CallToolResult, ContentBlock};

use shadows_core::CoreError;
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
    fn from(error: StorageError) -> Self {
        CoreError::Storage(error).into()
    }
}

impl From<shadows_core::CodeError> for Refusal {
    fn from(error: shadows_core::CodeError) -> Self {
        CoreError::from(error).into()
    }
}

impl Refusal {
    fn storage(e: StorageError, code: ErrorCode) -> Self {
        match e {
            StorageError::RevisionConflict { current, summary } => Self::new(
                code,
                format!(
                    "the plan changed; current revision is {current}: {summary}. \
                     Read it with workflow_get, then rebuild your edit."
                ),
            ),
            StorageError::CommandConflict => Self::new(
                code,
                "this command id, or this draft_ref, already did something different; \
                 for a new plan call draft_prepare again",
            ),
            // A rule the request broke, not a broken database: say which, so an
            // agent fixes the request instead of asking for a restart.
            StorageError::Constraint(message) => Self::new(ErrorCode::InvalidCommand, message),
            StorageError::NotFound(what) => Self::new(code, format!("no such {what}")),
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
        let code = e.code();
        match e {
            CoreError::Refused {
                code: ErrorCode::WorkflowFrozenImmutable,
                ..
            } => Self::new(
                ErrorCode::WorkflowFrozenImmutable,
                "this plan version is approved and never changes again. \
                 Start a new version with draft_start.",
            ),
            CoreError::ValidationFailed { problems, .. } => {
                Self::new(ErrorCode::WorkflowValidationFailed, problems.join("; "))
            }
            other @ (CoreError::ReasonMissing | CoreError::Unavailable(_)) => {
                tracing::error!(error = %other, "mcp.storage_failed");
                Self::new(
                    ErrorCode::StorageUnavailable,
                    "the daemon could not read or write its database",
                )
            }
            CoreError::Storage(e) | CoreError::Start(StartError::Storage(e)) => {
                Self::storage(e, code)
            }
            CoreError::Refused {
                code: ErrorCode::GrantInvalid,
                message,
            } if message == "the grant is unknown or revoked" => Self::new(
                ErrorCode::GrantInvalid,
                "this grant was revoked while the call ran; nothing was changed",
            ),
            CoreError::Refused {
                code: ErrorCode::GrantScope,
                message,
            } if message == "outside what the grant allows" => Self::scope(
                "outside what this grant allows: plan writes stay in its own project; \
                    workflow_list and workflow_get can read an explicitly linked project; \
                    draft_ref starts a plan only within the hour after draft_prepare",
            ),
            other @ CoreError::Refused {
                code: ErrorCode::ThreadBusy | ErrorCode::QueuedMessageGone,
                ..
            } => {
                tracing::error!(error = %other, "mcp.storage_failed");
                Self::new(
                    ErrorCode::StorageUnavailable,
                    "the daemon could not read or write its database",
                )
            }
            CoreError::Refused { code, message } => Self::new(code, message),
            other => Self::new(code, other.to_string()),
        }
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

impl From<shadows_core::ThreadsError> for Refusal {
    fn from(error: shadows_core::ThreadsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::GrantsError> for Refusal {
    fn from(error: shadows_core::GrantsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::OperationsError> for Refusal {
    fn from(error: shadows_core::OperationsError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::HarnessError> for Refusal {
    fn from(error: shadows_core::HarnessError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::DesignError> for Refusal {
    fn from(error: shadows_core::DesignError) -> Self {
        CoreError::from(error).into()
    }
}

impl From<shadows_core::PlansError> for Refusal {
    fn from(error: shadows_core::PlansError) -> Self {
        CoreError::from(error).into()
    }
}

#[cfg(test)]
mod mapping_tests {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../shadows-http/src/failure/cases.rs"
    ));

    #[test]
    fn every_core_error_keeps_its_mcp_mapping() {
        assert_eq!(Case::all().len(), 59);
        for case in Case::all() {
            let name = format!("{:?}", case.error);
            assert_eq!(case.error.code(), case.code, "{name}");
            let refusal = super::Refusal::from(case.error);
            assert_eq!(refusal.code, case.mcp_code, "{name}");
            assert_eq!(refusal.message, case.mcp, "{name}");
            let code = serde_json::to_value(case.mcp_code).unwrap();
            let result = serde_json::to_value(super::refused(refusal)).unwrap();
            assert_eq!(result["isError"], true, "{name}");
            assert_eq!(
                result["content"][0]["text"],
                format!("{}: {}", code.as_str().unwrap(), case.mcp),
                "{name}"
            );
            let _ = (case.code, case.status, case.http_code, case.http);
        }
    }
}
