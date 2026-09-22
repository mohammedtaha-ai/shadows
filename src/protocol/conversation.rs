//! One job: the routes a conversation runs through — reading a thread's
//! entries, starting a turn on it, stopping one.
//!
//! Every handler here is `pub(super)`, for the reason `project.rs` gives.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use crate::events::Actor;
use crate::operation::{Operation, OperationId};
use crate::planner::{PlannerTurn, PlannerTurnRequest};
use crate::thread::{NewThreadEntry, ThreadEntry, ThreadId};

/// A thread's entries in ordinal order.
#[utoipa::path(
    get,
    path = "/api/threads/{id}/entries",
    tag = "threads",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = Vec<ThreadEntry>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_entries(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
) -> Result<Json<Vec<ThreadEntry>>, Failure> {
    Ok(Json(s.storage.list_thread_entries(&thread_id).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct StartTurn {
    prompt: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct TurnStarted {
    operation_id: OperationId,
}

/// Starts a Planner turn. The turn runs in the thread's project directory and
/// continues the thread's harness session; neither is the client's to name.
///
/// Spec §3.3: a long-running command answers 202 with an operation id, and the
/// operation reaches its terminal outcome later — watch it on
/// `/api/subscribe`. A turn that cannot run (no project directory, no harness)
/// is still 202: the operation fails at `Prepare`, durably, with its reason.
#[utoipa::path(
    post,
    path = "/api/threads/{id}/turns",
    tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = StartTurn,
    responses(
        (status = 202, body = TurnStarted),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(StatusCode, Json<TurnStarted>), Failure> {
    // Record the user's message as a durable entry before the turn starts, so
    // a restart mid-turn still shows what was asked.
    s.storage
        .append_thread_entry(
            &thread_id,
            NewThreadEntry {
                kind: "UserMessage",
                author: Actor::user("local"),
                body: &body.prompt,
                refs: &[],
            },
        )
        .await?;

    let operation_id = PlannerTurn::start(
        s.runtime.clone(),
        s.handles.clone(),
        s.harness.clone(),
        PlannerTurnRequest {
            thread_id,
            prompt: body.prompt,
        },
        s.bus.clone(),
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(TurnStarted { operation_id })))
}

/// Stops a turn: terminates its process tree, confirms it is gone, and only
/// then records it `Cancelled` (spec §2.3). Answers with the operation as it
/// now stands.
#[utoipa::path(
    post,
    path = "/api/operations/{id}/stop",
    tag = "turns",
    params(("id" = OperationId, Path, description = "The turn's operation")),
    responses(
        (status = 200, body = Operation),
        (status = 404, description = "INVALID_COMMAND: no such operation", body = ErrorBody),
        (status = 409, description = "STORAGE_CONSTRAINT_VIOLATION: it had already ended another way", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<OperationId>,
) -> Result<Json<Operation>, Failure> {
    PlannerTurn::stop(s.runtime.clone(), s.handles.clone(), &op_id).await?;
    Ok(Json(s.storage.get_operation(&op_id).await?))
}
