//! One job: the routes over a task's attempts — the task's run state,
//! Start, Stop, an attempt's diff, a file at its commit, and the person's
//! done mark.
//!
//! Each handler reads the request, calls one `Runs` method and shapes the
//! answer. A refused Start is 409 with Runs' own `code` and `reason`.
//! Every handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::AppState;
use super::failure::{ErrorBody, RunFailure, RunRefusal};
use super::transport::Transport;
use shadows_core::{Attempt, AttemptFile, AttemptId, FileDiff, TaskRun, WorkflowId};

/// A task of a plan version: `/api/workflows/{workflow}/versions/{version}/tasks/{task}`.
type TaskPath = Path<(WorkflowId, i64, u32)>;

/// What Start takes.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct StartAttempt {
    /// The idempotency key: the same key and body answer the first attempt.
    command_id: String,
    /// The executor's model, one the harness offers.
    model: String,
}

/// What a done mark takes.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct MarkDone {
    /// The idempotency key, scoped to done marks.
    command_id: String,
    done: bool,
}

/// The task as its page reads it: whether it was marked done, its latest
/// attempt, the tasks it needs, whether Start may run now and why not, and
/// the model to offer first.
#[utoipa::path(
    get,
    path = "/api/workflows/{workflow}/versions/{version}/tasks/{task}",
    tag = "runs",
    params(
        ("workflow" = WorkflowId, Path, description = "The plan version"),
        ("version" = i64, Path, description = "Its version number"),
        ("task" = u32, Path, description = "The task's number"),
    ),
    responses(
        (status = 200, body = TaskRun),
        (status = 404, description = "no such plan version or task", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn task(
    State(s): State<AppState>,
    Path((workflow, version, task)): TaskPath,
) -> Result<Json<TaskRun>, RunFailure> {
    Ok(Json(s.core.runs().task(&workflow, version, task).await?))
}

/// The task's attempts, newest first.
#[utoipa::path(
    get,
    path = "/api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts",
    tag = "runs",
    params(
        ("workflow" = WorkflowId, Path, description = "The plan version"),
        ("version" = i64, Path, description = "Its version number"),
        ("task" = u32, Path, description = "The task's number"),
    ),
    responses(
        (status = 200, body = Vec<Attempt>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn attempts(
    State(s): State<AppState>,
    Path((workflow, version, task)): TaskPath,
) -> Result<Json<Vec<Attempt>>, RunFailure> {
    Ok(Json(
        s.core.runs().attempts(&workflow, version, task).await?,
    ))
}

/// Starts an attempt: an executor session on this task, on `model`. An
/// attempt whose session or first turn could not start is answered `failed`
/// with its reason. A replay of the same `command_id` and body answers the
/// first attempt.
#[utoipa::path(
    post,
    path = "/api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts",
    tag = "runs",
    params(
        ("workflow" = WorkflowId, Path, description = "The plan version"),
        ("version" = i64, Path, description = "Its version number"),
        ("task" = u32, Path, description = "The task's number"),
    ),
    request_body = StartAttempt,
    responses(
        (status = 201, body = Attempt),
        (status = 404, description = "no such plan version or task", body = ErrorBody),
        (
            status = 409,
            description = "Start refused, with the code a task's `start` answers: waiting, \
                           task_busy, overlap, not_git, dirty_scope, missing_input, \
                           input_unavailable, not_frozen, superseded, archived, bad_scope; \
                           or command_conflict",
            body = RunRefusal,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn start(
    State(s): State<AppState>,
    Path((workflow, version, task)): TaskPath,
    Json(body): Json<StartAttempt>,
) -> Result<(StatusCode, Json<Attempt>), RunFailure> {
    let core = s.core.clone();
    let attempt = Transport::detached_as(async move {
        core.runs()
            .start(body.command_id, &workflow, version, task, body.model)
            .await
            .map_err(RunFailure::from)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(attempt)))
}

/// Marks the task done or not done; the tasks that need it wait for this.
#[utoipa::path(
    put,
    path = "/api/workflows/{workflow}/versions/{version}/tasks/{task}/done",
    tag = "runs",
    params(
        ("workflow" = WorkflowId, Path, description = "The plan version"),
        ("version" = i64, Path, description = "Its version number"),
        ("task" = u32, Path, description = "The task's number"),
    ),
    request_body = MarkDone,
    responses(
        (status = 200, body = TaskRun),
        (status = 404, description = "no such plan version or task", body = ErrorBody),
        (status = 409, description = "command_conflict", body = RunRefusal),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn mark_done(
    State(s): State<AppState>,
    Path((workflow, version, task)): TaskPath,
    Json(body): Json<MarkDone>,
) -> Result<Json<TaskRun>, RunFailure> {
    Ok(Json(
        s.core
            .runs()
            .mark_done(body.command_id, &workflow, version, task, body.done)
            .await?,
    ))
}

/// Stops the attempt and answers once its turn's end is confirmed. An
/// attempt that already ended keeps its state.
#[utoipa::path(
    post,
    path = "/api/attempts/{id}/stop",
    tag = "runs",
    params(("id" = AttemptId, Path, description = "The attempt")),
    responses(
        (status = 200, body = Attempt),
        (status = 404, description = "no such attempt", body = ErrorBody),
        (status = 500, description = "the session could not be terminated", body = ErrorBody),
    )
)]
pub(super) async fn stop(
    State(s): State<AppState>,
    Path(attempt): Path<AttemptId>,
) -> Result<Json<Attempt>, RunFailure> {
    let core = s.core.clone();
    let attempt = Transport::detached_as(async move {
        core.runs().stop(&attempt).await.map_err(RunFailure::from)
    })
    .await?;
    Ok(Json(attempt))
}

/// Each file of the attempt's commit with its counts and patch; none before
/// it commits. A patch longer than 2,000 lines is cut with a marker.
#[utoipa::path(
    get,
    path = "/api/attempts/{id}/diff",
    tag = "runs",
    params(("id" = AttemptId, Path, description = "The attempt")),
    responses(
        (status = 200, body = Vec<FileDiff>),
        (status = 404, description = "no such attempt", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn diff(
    State(s): State<AppState>,
    Path(attempt): Path<AttemptId>,
) -> Result<Json<Vec<FileDiff>>, RunFailure> {
    Ok(Json(s.core.runs().diff(&attempt).await?))
}

/// Which file of an attempt's commit to read.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct FilePath {
    /// A path inside the task's writes, as the diff names it.
    path: String,
}

/// One file of the task as the attempt's commit holds it, changed by it or
/// not; text is null for a deleted or binary file, and cut is true when a
/// long file was shortened.
#[utoipa::path(
    get,
    path = "/api/attempts/{id}/files",
    tag = "runs",
    params(("id" = AttemptId, Path, description = "The attempt"), FilePath),
    responses(
        (status = 200, body = AttemptFile),
        (
            status = 404,
            description = "no such attempt, no commit yet, or no such file at its commit",
            body = ErrorBody,
        ),
        (status = 409, description = "outside_writes", body = RunRefusal),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn file(
    State(s): State<AppState>,
    Path(attempt): Path<AttemptId>,
    Query(query): Query<FilePath>,
) -> Result<Json<AttemptFile>, RunFailure> {
    Ok(Json(s.core.runs().file(&attempt, &query.path).await?))
}
