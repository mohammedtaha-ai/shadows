//! One job: the routes of a conversation's waiting messages (spec §20.5).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use shadows_core::{Queued, QueuedMessage, QueuedMessageId, SendTurn, SentNow, ThreadId};

use super::conversation::{StartTurn, detached};
use super::failure::ErrorBody;
use super::{AppState, Failure};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CommandParam {
    /// The idempotency key (spec §3.2).
    command_id: String,
}

/// Queues a message while a turn runs; on an idle thread starts it as a turn.
#[utoipa::path(
    post, path = "/api/threads/{id}/queue", tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = StartTurn,
    responses(
        (status = 202, body = Queued),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY; COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "as POST /api/threads/{id}/turns", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED", body = ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING", body = ErrorBody),
    )
)]
pub(super) async fn queue(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(StatusCode, Json<Queued>), Failure> {
    let turn: SendTurn = body.into();
    let core = s.core.clone();
    let answer = detached(async move {
        core.turns()
            .queue(thread, turn)
            .await
            .map_err(Failure::from)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
}

/// The thread's waiting messages, oldest first.
#[utoipa::path(
    get, path = "/api/threads/{id}/queue", tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = Vec<QueuedMessage>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn queued(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<Vec<QueuedMessage>>, Failure> {
    Ok(Json(s.core.turns().queued(&thread).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SendNowBody {
    /// The idempotency key (spec §3.2).
    command_id: String,
}

/// Send now: into the running turn, or as a turn when none runs.
#[utoipa::path(
    post, path = "/api/threads/{id}/queue/{qid}/send-now", tag = "turns",
    params(
        ("id" = ThreadId, Path, description = "The thread"),
        ("qid" = QueuedMessageId, Path, description = "The waiting message"),
    ),
    request_body = SendNowBody,
    responses(
        (status = 202, body = SentNow),
        (status = 404, description = "QUEUED_MESSAGE_GONE", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY: a Stop is pending; COMMAND_CONFLICT",
         body = ErrorBody),
        (status = 422, description = "as POST /api/threads/{id}/turns", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the steer failed; the message \
                                      keeps the reason", body = ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING", body = ErrorBody),
    )
)]
pub(super) async fn send_now(
    State(s): State<AppState>,
    Path((thread, qid)): Path<(ThreadId, QueuedMessageId)>,
    Json(body): Json<SendNowBody>,
) -> Result<(StatusCode, Json<SentNow>), Failure> {
    let core = s.core.clone();
    let answer = detached(async move {
        core.turns()
            .send_now(&thread, &qid, body.command_id)
            .await
            .map_err(Failure::from)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
}

/// Removes a waiting message.
#[utoipa::path(
    delete, path = "/api/threads/{id}/queue/{qid}", tag = "turns",
    params(
        ("id" = ThreadId, Path, description = "The thread"),
        ("qid" = QueuedMessageId, Path, description = "The waiting message"),
        CommandParam,
    ),
    responses(
        (status = 204),
        (status = 404, description = "QUEUED_MESSAGE_GONE", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn unqueue(
    State(s): State<AppState>,
    Path((thread, qid)): Path<(ThreadId, QueuedMessageId)>,
    Query(p): Query<CommandParam>,
) -> Result<StatusCode, Failure> {
    s.core.turns().unqueue(&thread, &qid, p.command_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
