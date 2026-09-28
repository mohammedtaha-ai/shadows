//! One job: the routes that change a planning thread itself — its harness
//! (spec §12.6), or a fork of it (§12.9).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::project::ctx;
use super::{AppState, Failure};
use crate::thread::{PlanningThread, ThreadEntryId, ThreadId};
use shadows_agent::policy;

/// Changes the thread's CLI. Refused once the thread has run a turn, and on
/// a fork from birth (`HARNESS_LOCKED`).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct UpdateThread {
    /// The idempotency key (spec §3.2), scoped to the thread.
    command_id: String,
    /// `claude-code` or `codex`.
    harness: String,
}

/// A harness Shadows knows, or `SETTING_NOT_OFFERED` naming it.
pub(super) fn known_harness(harness: &str) -> Result<(), Failure> {
    if policy::is_known(harness) {
        Ok(())
    } else {
        Err(Failure::setting_not_offered("harness", harness, None))
    }
}

/// Changes the thread's CLI before its first turn (spec §12.6). A replay
/// answers the thread as it now stands and changes nothing, even once the
/// harness is locked; a new command after the first turn, or on a fork (its
/// session is a fork of its source's, §12.9), is `HARNESS_LOCKED`. A change
/// closes the thread's adapter (§12.2).
#[utoipa::path(
    patch,
    path = "/api/threads/{id}",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = UpdateThread,
    responses(
        (status = 200, body = PlanningThread),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "HARNESS_LOCKED: the thread already ran a turn on its harness, or is a fork; or COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "SETTING_NOT_OFFERED: a harness Shadows does not know", body = ErrorBody),
    )
)]
pub(super) async fn update_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<UpdateThread>,
) -> Result<Json<PlanningThread>, Failure> {
    known_harness(&body.harness)?;
    let before = s.storage.turn_context(&thread).await?.harness;
    let params = serde_json::json!({ "thread_id": thread, "harness": body.harness });
    let c = ctx(body.command_id, "thread.harness", params);
    let updated = s
        .storage
        .set_thread_harness(&c, &thread, &body.harness)
        .await?;
    if updated.harness != before
        && let Err(error) = s.sessions.terminate(&thread).await
    {
        tracing::error!(%error, thread_id = %thread, "thread.harness_change_close_failed");
    }
    Ok(Json(updated))
}

/// Forks the thread from its last completed entry (spec §12.9).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ForkThread {
    /// The idempotency key (spec §3.2), scoped to the source thread.
    command_id: String,
    at_entry_id: ThreadEntryId,
}

/// Forks the thread from its last completed entry into a new thread on the
/// same project (spec §12.9). The new thread holds copies of the entries up to
/// and including `at_entry_id`; its next turn continues a fork of the
/// source's harness session. The source is unchanged. A replay answers the
/// fork already made.
#[utoipa::path(
    post,
    path = "/api/threads/{id}/fork",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The source thread")),
    request_body = ForkThread,
    responses(
        (status = 201, body = PlanningThread),
        (status = 404, description = "INVALID_COMMAND: no such thread or entry", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY: a turn is running, or COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "FORK_POINT_NOT_SUPPORTED: fork from anything but the last completed entry", body = ErrorBody),
    )
)]
pub(super) async fn fork_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<ForkThread>,
) -> Result<(StatusCode, Json<PlanningThread>), Failure> {
    let params = serde_json::json!({ "thread_id": thread, "at_entry_id": body.at_entry_id });
    let c = ctx(body.command_id, "thread.fork", params);
    let fork = s
        .storage
        .fork_thread(&c, &thread, &body.at_entry_id)
        .await?;
    Ok((StatusCode::CREATED, Json(fork)))
}
