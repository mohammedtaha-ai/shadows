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
use shadows_core::{Focus, Operation, OperationId, PlanId, SendTurn, ThreadEntry, ThreadId};

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
    Ok(Json(s.core.threads().entries(&thread_id).await?))
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
    Ok(Json(s.core.threads().operations(&thread_id).await?))
}

/// Starts a turn as one command (spec §12.7).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct StartTurn {
    /// The idempotency key (spec §3.2). A retry sends the same one.
    command_id: String,
    prompt: String,
    model: String,
    mode: String,
    /// `null` exactly when the chosen model offers no effort (§12.4).
    #[schema(required)]
    effort: Option<String>,
    /// The task the person points at (§13.9). It must be a task of that plan
    /// version, which must be this thread's; it is kept with the message and
    /// told to the Planner. Part of the command: the same `command_id` with
    /// another focus is `COMMAND_CONFLICT`.
    #[serde(default)]
    focus: Option<Focus>,
    /// The plan the person opened this conversation to continue (§16.4).
    #[serde(default)]
    plan: Option<PlanId>,
    /// The sending tab's id, made once per page load (§13.9). Kept in memory
    /// for the turn only, so a `plan-show` frame can name it; not part of the
    /// command, never stored.
    #[serde(default)]
    client_tab: Option<String>,
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
/// `/api/subscribe`. Spec §12.7: the prompt, the operation, its invocation
/// and the command record commit together, after every check; a check that
/// fails writes nothing. A retry with the same `command_id` and body answers
/// the first operation and starts nothing, even while the daemon stops.
///
/// A daemon that has begun to stop refuses a new turn with 503 (spec §8.5).
#[utoipa::path(
    post,
    path = "/api/threads/{id}/turns",
    tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = StartTurn,
    responses(
        (status = 202, body = TurnStarted),
        (
            status = 403,
            description = "MODE_NOT_ALLOWED: the project does not allow this mode",
            body = ErrorBody,
        ),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (
            status = 409,
            description = "THREAD_BUSY: a turn is running; COMMAND_CONFLICT: this command_id was \
                           used with another request; PATH_NOT_FOUND: the project's directory is \
                           gone or was never set; nothing was written",
            body = ErrorBody,
        ),
        (
            status = 422,
            description = "SETTING_NOT_OFFERED: a model, mode or effort the session does not \
                           offer; HARNESS_UNAVAILABLE; INVALID_COMMAND: the focus names a task \
                           not in that plan, or a plan not this thread's",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
        (
            status = 502,
            description = "HARNESS_START_FAILED: the thread's session could not be opened; \
                           nothing was written",
            body = ErrorBody,
        ),
        (
            status = 503,
            description = "RUNTIME_STOPPING: the daemon is shutting down",
            body = ErrorBody,
        ),
    )
)]
pub(super) async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(StatusCode, Json<TurnStarted>), Failure> {
    let StartTurn {
        command_id,
        prompt,
        model,
        mode,
        effort,
        focus,
        plan,
        client_tab,
    } = body;
    let turn = SendTurn {
        command_id,
        prompt,
        model,
        mode,
        effort,
        focus,
        plan,
        client_tab,
    };
    let core = s.core.clone();
    let operation_id = detached(async move {
        core.turns()
            .send(thread_id, turn)
            .await
            .map_err(Failure::from)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(TurnStarted { operation_id })))
}

/// Spec §8.4 case 7: a client disconnecting cancels nothing. Hyper drops a
/// handler's future when its connection closes, and a turn route dropped
/// between its writes strands work: a `Pending` operation nothing will ever
/// start, or a tree `stop` killed without writing `Cancelled`. So the work runs
/// in its own task and the request only awaits it; a dropped request leaves
/// the task running to its end. A panic in it is re-raised here, exactly as
/// if the handler itself had panicked. `harness.rs` runs a model change the
/// same way: a dropped request must not drop the session it holds.
pub(super) async fn detached<T: Send + 'static>(
    work: impl Future<Output = Result<T, Failure>> + Send + 'static,
) -> Result<T, Failure> {
    match tokio::spawn(work.in_current_span()).await {
        Ok(answer) => answer,
        Err(error) => std::panic::resume_unwind(error.into_panic()),
    }
}

/// Stops a turn: asks the harness to cancel it, and if the harness does not
/// confirm in time, terminates the adapter's process tree, confirms it is
/// gone, and only then records it `Cancelled` (spec §2.3, §12.3). Answers
/// with the operation as it now stands — which may still be `Running` for a
/// moment when the turn had already ended on its own and its ending is being
/// recorded.
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
        (
            status = 409,
            description = "STORAGE_CONSTRAINT_VIOLATION: it had already ended another way",
            body = ErrorBody,
        ),
        (
            status = 500,
            description = "PROCESS_TERMINATION_FAILED: the tree is still running, or \
                           STORAGE_UNAVAILABLE",
            body = ErrorBody,
        ),
    )
)]
pub(super) async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<OperationId>,
) -> Result<Json<Operation>, Failure> {
    let core = s.core.clone();
    let operation =
        detached(async move { core.turns().stop(&op_id).await.map_err(Failure::from) }).await?;
    Ok(Json(operation))
}
