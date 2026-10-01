//! One job: the harness and session-choice routes (spec §12.4, §12.10).
//!
//! `GET /api/harnesses` lists the CLIs a conversation can run on; `POST
//! /api/threads/{id}/session` opens a thread's harness session and answers
//! what it offers, after Shadows' policy and the project's allowed modes;
//! `PUT /api/threads/{id}/session/model` and `.../session/effort` change that
//! session's model or effort. Each ends in `core.harness()`.

use axum::Json;
use axum::extract::{Path, State};

use super::conversation::detached;
use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::ThreadId;
use shadows_core::{ContextBreakdown, HarnessInfo, SessionChoices};

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
    Ok(Json(s.core.harness().list().await?))
}

/// Opens the thread's harness session (spec §12.2) and answers what it
/// offers. Idempotent: an open session answers what it holds. A client calls
/// it when it shows the conversation, so the menus are ready before the
/// first message. It carries no `command_id` and records no command, entry
/// or operation. An opening that starts an adapter issues the thread's MCP
/// grant (spec §13.7): an `mcp_grant` row and a durable `McpGrantIssued`
/// event, after revoking with `McpGrantRevoked` the grant of any closed
/// adapter it replaces; an opening that fails revokes the grant it issued.
/// A session already open writes nothing.
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
    Ok(Json(s.core.harness().open_session(&thread).await?))
}

/// The model a person picked (spec §12.7).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ChangeModel {
    /// One of the session's `models`.
    model: String,
}

/// Sets the thread's session to `model` as soon as a person picks it (spec
/// §12.7), opening the session first if it is not open, and answers its
/// choices exactly as `POST .../session` does — the efforts are now the new
/// model's. The change itself writes nothing durable and no `command_id` is
/// carried: setting the same model twice is the same state, and a turn
/// records its model in its own invocation. An opening it causes writes what
/// `POST .../session` writes (the thread's MCP grant and its events). The
/// remembered settings do not move (§12.4); when the session moves to the
/// model, it is set to that model's remembered effort if still offered.
/// A running turn's session is not changed (`THREAD_BUSY`).
#[utoipa::path(
    put,
    path = "/api/threads/{id}/session/model",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = ChangeModel,
    responses(
        (status = 200, body = SessionChoices),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY: a turn is running; PATH_NOT_FOUND: the project's directory is gone or was never set", body = ErrorBody),
        (status = 422, description = "SETTING_NOT_OFFERED: a model the session does not offer, or one the harness refused (its words in the message); HARNESS_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the adapter did not start, or its session closed", body = ErrorBody),
    )
)]
pub(super) async fn change_model(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<ChangeModel>,
) -> Result<Json<SessionChoices>, Failure> {
    let choices =
        detached(async move { Ok(s.core.harness().change_model(&thread, &body.model).await?) })
            .await?;
    Ok(Json(choices))
}

/// The effort a person picked (spec §12.7).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ChangeEffort {
    /// One of the session's `efforts`, for the model it holds.
    effort: String,
}

/// Sets the thread's session to `effort` as soon as a person picks it (spec
/// §12.7), as `PUT .../session/model` sets a model: it opens the session
/// first if it is not open, answers its choices exactly as `POST
/// .../session` does, writes nothing durable and carries no `command_id`.
/// The remembered settings do not move (§12.4).
/// A running turn's session is not changed (`THREAD_BUSY`).
#[utoipa::path(
    put,
    path = "/api/threads/{id}/session/effort",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = ChangeEffort,
    responses(
        (status = 200, body = SessionChoices),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY: a turn is running; PATH_NOT_FOUND: the project's directory is gone or was never set", body = ErrorBody),
        (status = 422, description = "SETTING_NOT_OFFERED: an effort the session's model does not offer, or one the harness refused (its words in the message); HARNESS_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the adapter did not start, or its session closed", body = ErrorBody),
    )
)]
pub(super) async fn change_effort(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<ChangeEffort>,
) -> Result<Json<SessionChoices>, Failure> {
    let choices = detached(async move {
        Ok(s.core
            .harness()
            .change_effort(&thread, &body.effort)
            .await?)
    })
    .await?;
    Ok(Json(choices))
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
    Ok(Json(s.core.harness().context(&thread).await?))
}
