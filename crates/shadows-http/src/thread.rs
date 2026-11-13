//! One job: the routes that change a planning thread itself — its harness, a fork of it,
//! or the person's answer to a question asked in it.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{PlanningThread, QuestionId, Queued, ThreadEntryId, ThreadId};

/// The idempotency key for conversation deletion.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveThreadQuery {
    command_id: String,
}

#[utoipa::path(
    get,
    path = "/api/threads/{id}",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses((status = 200, body = PlanningThread), (status = 404, body = ErrorBody))
)]
pub(super) async fn get_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<PlanningThread>, Failure> {
    Ok(Json(s.core.threads().get(&thread).await?))
}

#[utoipa::path(
    delete,
    path = "/api/threads/{id}",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread"), RemoveThreadQuery),
    responses(
        (status = 200, body = PlanningThread),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
    )
)]
pub(super) async fn remove_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Query(query): Query<RemoveThreadQuery>,
) -> Result<Json<PlanningThread>, Failure> {
    Ok(Json(
        s.core
            .turns()
            .remove_thread(query.command_id, &thread)
            .await?,
    ))
}

/// Changes the thread's CLI. Refused once the thread has run a turn, and on
/// a fork from birth (`HARNESS_LOCKED`).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct UpdateThread {
    /// The idempotency key, scoped to the thread.
    command_id: String,
    /// `claude-code` or `codex`.
    harness: String,
}

/// Changes the thread's CLI before its first turn. A replay
/// answers the thread as it now stands and changes nothing, even once the
/// harness is locked; a new command after the first turn, or on a fork (its
/// session is a fork of its source's), is `HARNESS_LOCKED`. A change
/// closes the thread's adapter.
#[utoipa::path(
    patch,
    path = "/api/threads/{id}",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = UpdateThread,
    responses(
        (status = 200, body = PlanningThread),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (
            status = 409,
            description = "HARNESS_LOCKED: the thread already ran a turn on its harness, or is a \
                           fork; or COMMAND_CONFLICT",
            body = ErrorBody,
        ),
        (
            status = 422,
            description = "SETTING_NOT_OFFERED: a harness Shadows does not know",
            body = ErrorBody,
        ),
    )
)]
pub(super) async fn update_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<UpdateThread>,
) -> Result<Json<PlanningThread>, Failure> {
    Ok(Json(
        s.core
            .harness()
            .set_thread_harness(body.command_id, &thread, &body.harness)
            .await?,
    ))
}

/// Forks the thread from its last completed entry.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ForkThread {
    /// The idempotency key, scoped to the source thread.
    command_id: String,
    at_entry_id: ThreadEntryId,
}

/// Forks the thread from its last completed entry into a new thread on the
/// same project. The new thread holds copies of the entries up to
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
        (
            status = 409,
            description = "THREAD_BUSY: a turn is running, or COMMAND_CONFLICT",
            body = ErrorBody,
        ),
        (
            status = 422,
            description = "FORK_POINT_NOT_SUPPORTED: fork from anything but the last completed \
                           entry",
            body = ErrorBody,
        ),
    )
)]
pub(super) async fn fork_thread(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<ForkThread>,
) -> Result<(StatusCode, Json<PlanningThread>), Failure> {
    let fork = s
        .core
        .harness()
        .fork_thread(body.command_id, &thread, &body.at_entry_id)
        .await?;
    Ok((StatusCode::CREATED, Json(fork)))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct AnswerBody {
    /// The idempotency key.
    command_id: String,
    /// The chosen option, by its index in the question's card.
    option: usize,
}

/// The person's click on a question's option: their message to the Planner,
/// sent at once on an idle thread, waiting while a turn runs.
#[utoipa::path(
    post, path = "/api/threads/{id}/questions/{question}/answer", tag = "turns",
    params(
        ("id" = ThreadId, Path, description = "The thread"),
        ("question" = QuestionId, Path, description = "The question"),
    ),
    request_body = AnswerBody,
    responses(
        (status = 202, body = Queued),
        (status = 404, description = "no such thread or question", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT: already answered", body = ErrorBody),
        (status = 422, description = "INVALID_COMMAND: no such option, or the question was \
                                      asked in another conversation", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED", body = ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING", body = ErrorBody),
    )
)]
pub(super) async fn answer_question(
    State(s): State<AppState>,
    Path((thread, question)): Path<(ThreadId, QuestionId)>,
    Json(body): Json<AnswerBody>,
) -> Result<(StatusCode, Json<Queued>), Failure> {
    let core = s.core.clone();
    let answer = super::transport::Transport::detached(async move {
        core.turns()
            .answer(body.command_id, &thread, &question, body.option)
            .await
            .map_err(Failure::from)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
}
