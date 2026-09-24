//! One job: the routes a conversation runs through — reading a thread's
//! entries and its turns, starting a turn on it, stopping one.
//!
//! Every handler here is `pub(super)`, for the reason `project.rs` gives.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use tracing::Instrument;

use super::failure::ErrorBody;
use super::harness::open_failure;
use super::project::ctx;
use super::{AppState, Failure};
use crate::agent::TurnSettings;
use crate::agent::acp::AcpError;
use crate::agent::choices::{Offered, refusal};
use crate::agent::policy;
use crate::events::Actor;
use crate::operation::{Operation, OperationId};
use crate::planner::{OpenSession, PlannerTurn, PlannerTurnRequest, StopOutcome};
use crate::storage::{NewTurn, StorageError};
use crate::thread::{ThreadEntry, ThreadId};

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

/// Starts a turn as one command (spec §12.7).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct StartTurn {
    /// The idempotency key (spec §3.2). A retry sends the same one.
    command_id: String,
    prompt: String,
    model: String,
    mode: String,
    /// `null` exactly when the chosen model offers no effort (§12.4).
    effort: Option<String>,
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
        (status = 403, description = "MODE_NOT_ALLOWED: the project does not allow this mode", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = ErrorBody),
        (status = 409, description = "THREAD_BUSY: a turn is running; COMMAND_CONFLICT: this command_id was used with another request; PATH_NOT_FOUND: the project's directory is gone or was never set; nothing was written", body = ErrorBody),
        (status = 422, description = "SETTING_NOT_OFFERED: a model, mode or effort the session does not offer; HARNESS_UNAVAILABLE", body = ErrorBody),
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
    let operation_id = detached(start(s, thread_id, body)).await?;
    Ok((StatusCode::ACCEPTED, Json(TurnStarted { operation_id })))
}

/// §12.7's order: a replay first, answered before anything else; then every
/// check, before any write; then the one transaction; then the run.
async fn start(s: AppState, thread_id: ThreadId, body: StartTurn) -> Result<OperationId, Failure> {
    let StartTurn {
        command_id,
        prompt,
        model,
        mode,
        effort,
    } = body;
    let params = serde_json::json!({
        "thread_id": thread_id, "prompt": prompt, "model": model, "mode": mode, "effort": effort,
    });
    let command = ctx(command_id, "turn.start", params);
    if let Some(replay) = s.storage.replayed_turn(&command, &thread_id).await? {
        return Ok(replay.operation_id);
    }
    if s.handles.is_closed().await {
        return Err(Failure::runtime_stopping());
    }
    let settings = TurnSettings {
        model,
        mode,
        effort,
    };
    let context = s.storage.turn_context(&thread_id).await?;
    if !policy::is_available(&context.harness) {
        return Err(Failure::harness_unavailable(&context.harness));
    }
    // Checked before the session is touched: setting a model below would
    // change the session a running turn is using.
    if s.storage.thread_is_busy(&thread_id).await? {
        return Err(StorageError::ThreadBusy.into());
    }
    let opened = s.sessions.open(&thread_id).await.map_err(open_failure)?;
    let offered = offer_for_model(&s, &thread_id, &opened, &settings.model).await?;
    if let Some((what, id)) = refusal(&offered, &context.harness, &settings) {
        return Err(Failure::setting_not_offered(what, &id, None));
    }
    let project = s.storage.get_project(&context.project_id).await?;
    let allowed = project.allowed_modes.get(&context.harness);
    if !allowed.is_some_and(|modes| modes.contains(&settings.mode)) {
        return Err(Failure::mode_not_allowed(&settings.mode));
    }

    let adapter = s.sessions.adapter();
    let (harness_path, agent_path) = (
        adapter.adapter.to_string_lossy().into_owned(),
        adapter.agent.to_string_lossy().into_owned(),
    );
    let started = s
        .storage
        .start_turn(
            &command,
            NewTurn {
                thread_id: &thread_id,
                runtime: &s.runtime.instance_id,
                prompt: &prompt,
                role: "Planner",
                harness_kind: &context.harness,
                harness_path: &harness_path,
                harness_version: &adapter.adapter_version,
                agent_path: &agent_path,
                agent_version: &adapter.agent_version,
                settings: &settings,
            },
        )
        .await;
    let started = match started {
        Ok(started) => started,
        Err(StorageError::TransitionConflict { .. }) if s.handles.is_closed().await => {
            return Err(Failure::runtime_stopping());
        }
        Err(e) => return Err(e.into()),
    };
    if started.replayed {
        return Ok(started.operation_id);
    }
    Ok(PlannerTurn::start(
        s.runtime.clone(),
        s.handles.clone(),
        s.sessions.clone(),
        opened,
        PlannerTurnRequest {
            thread_id,
            operation_id: started.operation_id,
            prompt,
            settings,
        },
        s.bus.clone(),
    )
    .await?)
}

/// The session's offer for `model`. Efforts belong to a model, so a turn
/// naming another model than the session holds sets it first (§12.7): the
/// only session change made before the transaction, since it records
/// nothing. The harness refusing it is `SETTING_NOT_OFFERED` in its words.
async fn offer_for_model(
    s: &AppState,
    thread: &ThreadId,
    opened: &OpenSession,
    model: &str,
) -> Result<Offered, Failure> {
    let closed = || Failure::harness_start_failed("the harness session closed".into());
    let offered = s.sessions.offered(thread).await.ok_or_else(closed)?;
    if offered.current.model == model || !offered.offers_model(model) {
        return Ok(offered);
    }
    let id = offered.ids.model.clone();
    match s.sessions.set_option(thread, opened, &id, model).await {
        Ok(next) => Ok(next),
        Err(AcpError::Rpc(message)) => {
            Err(Failure::setting_not_offered("model", model, Some(&message)))
        }
        Err(AcpError::Closed) => Err(closed()),
    }
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
