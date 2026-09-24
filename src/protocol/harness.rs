//! One job: the harness and session-choice routes (spec §12.4, §12.10).
//!
//! `GET /api/harnesses` lists the CLIs a conversation can run on; `POST
//! /api/threads/{id}/session` opens a thread's harness session and answers
//! what it offers, after Shadows' policy and the project's allowed modes.

use axum::Json;
use axum::extract::{Path, State};

use super::failure::ErrorBody;
use super::{AppState, Failure};
use crate::agent::breakdown::Category;
use crate::agent::choices::{Offered, SessionChoices, for_client};
use crate::agent::events::AccountLimits;
use crate::agent::policy;
use crate::planner::OpenError;
use crate::storage::Storage;
use crate::thread::ThreadId;

/// The model and effort last chosen for this harness (spec §12.4).
#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct RememberedSettings {
    model: String,
    #[schema(required)]
    effort: Option<String>,
}

/// A CLI a conversation can run on (spec §12.1). `kind` is `claude-code` or
/// `codex`.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct HarnessInfo {
    kind: String,
    label: String,
    available: bool,
    /// Why it cannot run, when `available` is false.
    #[schema(required)]
    reason: Option<String>,
    #[schema(required)]
    remembered: Option<RememberedSettings>,
    #[schema(required)]
    limits: Option<AccountLimits>,
}

fn label(kind: &str) -> &'static str {
    match kind {
        policy::CLAUDE_CODE => "Claude Code",
        policy::CODEX => "Codex",
        _ => "Unknown",
    }
}

/// Every harness Shadows knows, runnable or not, with the model and effort
/// last used on it and the account limits it last reported.
#[utoipa::path(
    get,
    path = "/api/harnesses",
    tag = "harnesses",
    responses(
        (status = 200, body = Vec<HarnessInfo>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_harnesses(
    State(s): State<AppState>,
) -> Result<Json<Vec<HarnessInfo>>, Failure> {
    let mut list = Vec::new();
    for kind in policy::KNOWN {
        let available = policy::is_available(kind);
        let remembered = s
            .storage
            .remembered_settings(kind)
            .await?
            .map(|(model, effort)| RememberedSettings { model, effort });
        list.push(HarnessInfo {
            kind: kind.to_string(),
            label: label(kind).to_string(),
            available,
            reason: (!available).then(|| "Coming later".to_string()),
            remembered,
            limits: s.storage.latest_limits(kind).await?,
        });
    }
    Ok(Json(list))
}

/// A session opening failure as the client sees it (spec §12.7's order).
pub(super) fn open_failure(e: OpenError) -> Failure {
    match e {
        OpenError::Storage(e) => Failure::from(e),
        OpenError::Start(reason) => Failure::harness_start_failed(reason),
        OpenError::Workspace(reason) => Failure::project_directory_unusable(reason),
    }
}

/// What a client is offered on `thread`: the session's choices after the
/// policy of the thread's harness and its project's allowed modes.
pub(super) async fn choices_for(
    storage: &Storage,
    thread: &ThreadId,
    offered: &Offered,
) -> Result<SessionChoices, Failure> {
    let context = storage.turn_context(thread).await?;
    let project = storage.get_project(&context.project_id).await?;
    let allowed = project
        .allowed_modes
        .get(&context.harness)
        .cloned()
        .unwrap_or_default();
    Ok(for_client(offered, &context.harness, &allowed))
}

/// Opens the thread's harness session (spec §12.2) and answers what it
/// offers. Idempotent: an open session answers what it holds. A client calls
/// it when it shows the conversation, so the menus are ready before the
/// first message.
#[utoipa::path(
    post,
    path = "/api/threads/{id}/session",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = SessionChoices),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "PATH_NOT_FOUND: the project's directory is gone or was never set", body = ErrorBody),
        (status = 422, description = "HARNESS_UNAVAILABLE: the thread's harness is listed but not runnable", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the adapter did not start", body = ErrorBody),
    )
)]
pub(super) async fn open_session(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<SessionChoices>, Failure> {
    let context = s.storage.turn_context(&thread).await?;
    if !policy::is_available(&context.harness) {
        return Err(Failure::harness_unavailable(&context.harness));
    }
    s.sessions.open(&thread).await.map_err(open_failure)?;
    let offered =
        s.sessions.offered(&thread).await.ok_or_else(|| {
            Failure::harness_start_failed("the session closed as it opened".into())
        })?;
    Ok(Json(choices_for(&s.storage, &thread, &offered).await?))
}

/// The context breakdown read on demand (spec §12.8): the categories, or none
/// with the reason.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct ContextBreakdown {
    #[schema(required)]
    categories: Option<Vec<Category>>,
    #[schema(required)]
    reason: Option<String>,
}

/// The context breakdown of the thread's session, read on demand (spec
/// §12.8). It is fetched only on an open, idle session that has answered a
/// turn, within five seconds; otherwise it says why it has none. Nothing is
/// written: no operation, no entry.
#[utoipa::path(
    get,
    path = "/api/threads/{id}/context",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = ContextBreakdown),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn thread_context(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<ContextBreakdown>, Failure> {
    s.storage.turn_context(&thread).await?;
    Ok(Json(match s.sessions.context(&thread).await {
        Ok(categories) => ContextBreakdown {
            categories: Some(categories),
            reason: None,
        },
        Err(why) => ContextBreakdown {
            categories: None,
            reason: Some(why.reason().to_string()),
        },
    }))
}
