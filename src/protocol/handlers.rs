//! One job: what each route does.
//!
//! Every item here is `pub(super)`: `router()` in the parent is the only thing
//! that names them, and a route handler reachable from outside `protocol/`
//! would be a second, undeclared entry point into the product.

use axum::Json;
use axum::extract::{Path, State};

use super::{AppState, Failure};
use crate::command::{CommandContext, fingerprint};
use crate::events::Actor;
use crate::operation::OperationId;
use crate::planner::{PlannerTurn, PlannerTurnRequest};
use crate::project::{ProjectDirectory, ProjectId};
use crate::thread::{NewThreadEntry, ThreadId};

/// Every route that mutates carries the caller's command id (spec §3.2), and
/// the fingerprint is derived from the same parameters the capability is about
/// to act on — never supplied by the client, which would let a replay with a
/// different body claim to be the same command.
fn ctx(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

#[derive(serde::Deserialize)]
pub(super) struct CreateProject {
    command_id: String,
    slug: String,
    name: String,
    /// Absolute path to an existing directory. Stored canonical; see
    /// `ProjectDirectory`.
    directory: String,
}

pub(super) async fn list_projects(
    State(s): State<AppState>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(s.storage.list_projects().await?)))
}

pub(super) async fn create_project(
    State(s): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Result<Json<serde_json::Value>, Failure> {
    // Resolved before the fingerprint is taken, so two spellings of one
    // folder are one request, and a bad path is refused before any write.
    let directory = ProjectDirectory::resolve(std::path::Path::new(&body.directory))?;
    let params = serde_json::json!({
        "slug": body.slug, "name": body.name, "directory": directory.as_str(),
    });
    let c = ctx(body.command_id, "project.create", params);
    Ok(Json(serde_json::json!(
        s.storage
            .create_project(&c, &body.slug, &body.name, &directory)
            .await?
    )))
}

pub(super) async fn list_threads(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_threads_for_project(&project_id).await?
    )))
}

#[derive(serde::Deserialize)]
pub(super) struct CreateThread {
    command_id: String,
    title: String,
}

pub(super) async fn create_thread(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateThread>,
) -> Result<Json<serde_json::Value>, Failure> {
    let params = serde_json::json!({ "project": project_id, "title": body.title });
    let c = ctx(body.command_id, "thread.create", params);
    Ok(Json(serde_json::json!(
        s.storage
            .create_planning_thread(&c, &project_id, &body.title)
            .await?
    )))
}

pub(super) async fn list_entries(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_thread_entries(&thread_id).await?
    )))
}

#[derive(serde::Deserialize)]
pub(super) struct StartTurn {
    prompt: String,
}

/// Spec §3.3: a long-running command returns 202 and an operation id. The
/// operation reaches its terminal outcome later.
pub(super) async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), Failure> {
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

    let op = PlannerTurn::start(
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

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(serde_json::json!({ "operation_id": op })),
    ))
}

pub(super) async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<OperationId>,
) -> Result<Json<serde_json::Value>, Failure> {
    PlannerTurn::stop(s.runtime.clone(), s.handles.clone(), &op_id).await?;
    Ok(Json(serde_json::json!(
        s.storage.get_operation(&op_id).await?
    )))
}
