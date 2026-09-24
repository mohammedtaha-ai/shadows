//! One job: the routes a conversation runs through — reading a thread's
//! entries and its turns, starting a turn on it, stopping one.
//!
//! Every handler here is `pub(super)`, for the reason `project.rs` gives.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::Instrument;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use crate::events::Actor;
use crate::operation::{Operation, OperationId};
use crate::planner::{PlannerTurn, PlannerTurnRequest, StopOutcome};
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

/// A thread's operations — its turns — newest first, each as it now stands.
/// A client opening a thread reads this to learn whether a turn is running and
/// which one (so it can offer Stop), then follows it on `/api/subscribe`,
/// whose durable frames name their `operation_id`.
#[utoipa::path(
    get,
    path = "/api/threads/{id}/operations",
    tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = Vec<Operation>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_operations(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
) -> Result<Json<Vec<Operation>>, Failure> {
    Ok(Json(
        s.storage.list_operations_for_thread(&thread_id).await?,
    ))
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
/// `/api/subscribe`. The thread's harness session is opened first (spec
/// §12.7); when it cannot be, nothing is written: 409 when the project's
/// directory is gone or was never set, 502 when the adapter does not start.
///
/// A daemon that has begun to stop refuses the turn with 503 (spec §8.5).
#[utoipa::path(
    post,
    path = "/api/threads/{id}/turns",
    tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = StartTurn,
    responses(
        (status = 202, body = TurnStarted),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "PATH_NOT_FOUND: the project's directory is gone or was never set; nothing was written", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the thread's session could not be opened; nothing was written", body = ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING: the daemon is shutting down", body = ErrorBody),
    )
)]
pub(super) async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(StatusCode, Json<TurnStarted>), Failure> {
    let operation_id = detached(start(s, thread_id, body.prompt)).await?;
    Ok((StatusCode::ACCEPTED, Json(TurnStarted { operation_id })))
}

async fn start(s: AppState, thread_id: ThreadId, prompt: String) -> Result<OperationId, Failure> {
    // Refused before the message is recorded, so a turn the daemon will not
    // run leaves no question behind it. `PlannerTurn::start` asks again; this
    // only keeps the common case clean.
    if s.handles.is_closed().await {
        return Err(Failure::runtime_stopping());
    }
    let opened = s.sessions.open(&thread_id).await.map_err(|e| match e {
        crate::planner::OpenError::Storage(e) => Failure::from(e),
        crate::planner::OpenError::Start(reason) => Failure::harness_start_failed(reason),
        crate::planner::OpenError::Workspace(reason) => Failure::project_directory_unusable(reason),
    })?;
    // Record the user's message as a durable entry before the turn starts, so
    // a restart mid-turn still shows what was asked.
    s.storage
        .append_thread_entry(
            &thread_id,
            NewThreadEntry {
                kind: "UserMessage",
                author: Actor::user("local"),
                body: &prompt,
                refs: &[],
            },
        )
        .await?;

    Ok(PlannerTurn::start(
        s.runtime.clone(),
        s.handles.clone(),
        s.sessions.clone(),
        opened,
        PlannerTurnRequest { thread_id, prompt },
        s.bus.clone(),
    )
    .await?)
}

/// Spec §8.4 case 7: a client disconnecting cancels nothing. Hyper drops a
/// handler's future when its connection closes, and a turn route dropped
/// between its writes strands work: a `Pending` operation nothing will ever
/// start, or a tree `stop` killed without writing `Cancelled`. So the work runs
/// in its own task and the request only awaits it; a dropped request leaves
/// the task running to its end. A panic in it is re-raised here, exactly as
/// if the handler itself had panicked.
async fn detached<T: Send + 'static>(
    work: impl Future<Output = Result<T, Failure>> + Send + 'static,
) -> Result<T, Failure> {
    match tokio::spawn(work.in_current_span()).await {
        Ok(answer) => answer,
        Err(error) => std::panic::resume_unwind(error.into_panic()),
    }
}

/// Stops a turn: asks the harness to cancel it, and if the harness does not
/// confirm in time, terminates the adapter's process tree, confirms it is
/// gone, and only then records it `Cancelled` (spec §2.3, §12.3). Answers with the operation as it
/// now stands — which may still be `Running` for a moment when the turn had
/// already ended on its own and its ending is being recorded.
///
/// If the tree cannot be terminated the answer is 500
/// `PROCESS_TERMINATION_FAILED`, and the operation is not `Cancelled`.
#[utoipa::path(
    post,
    path = "/api/operations/{id}/stop",
    tag = "turns",
    params(("id" = OperationId, Path, description = "The turn's operation")),
    responses(
        (status = 200, body = Operation),
        (status = 404, description = "INVALID_COMMAND: no such operation", body = ErrorBody),
        (status = 409, description = "STORAGE_CONSTRAINT_VIOLATION: it had already ended another way", body = ErrorBody),
        (status = 500, description = "PROCESS_TERMINATION_FAILED: the tree is still running, or STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<OperationId>,
) -> Result<Json<Operation>, Failure> {
    let operation = detached(async move {
        let outcome = PlannerTurn::stop(
            s.runtime.clone(),
            s.handles.clone(),
            s.sessions.clone(),
            &op_id,
            Actor::user("local"),
        )
        .await?;
        if outcome == StopOutcome::TerminationFailed {
            return Err(Failure::termination_failed());
        }
        Ok(s.storage.get_operation(&op_id).await?)
    })
    .await?;
    Ok(Json(operation))
}
