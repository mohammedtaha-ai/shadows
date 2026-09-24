//! One job: the routes under `/api/projects` — projects, and the threads each
//! one holds.
//!
//! Every handler here is `pub(super)`: the route table in the parent is the
//! only thing that names them, and a route handler reachable from outside
//! `protocol/` would be a second, undeclared entry point into the product.

use axum::Json;
use axum::extract::{Path, State};

use super::failure::ErrorBody;
use super::{AppState, Failure};
use crate::agent::policy;
use crate::command::{CommandContext, fingerprint};
use crate::project::{Project, ProjectDirectory, ProjectId};
use crate::thread::PlanningThread;

/// Every route that mutates carries the caller's command id (spec §3.2), and
/// the fingerprint is derived from the same parameters the capability is about
/// to act on — never supplied by the client, which would let a replay with a
/// different body claim to be the same command.
pub(super) fn ctx(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateProject {
    /// The idempotency key (spec §3.2). The same id with the same request
    /// replays the first answer; with a different one it is a conflict.
    command_id: String,
    slug: String,
    name: String,
    /// Absolute path to an existing directory, in any spelling; it is stored
    /// and returned canonical.
    directory: String,
}

/// Every project, oldest first.
#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "projects",
    responses(
        (status = 200, body = Vec<Project>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_projects(
    State(s): State<AppState>,
) -> Result<Json<Vec<Project>>, Failure> {
    Ok(Json(s.storage.list_projects().await?))
}

/// Creates a project owning an existing directory.
#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "projects",
    request_body = CreateProject,
    responses(
        (status = 200, description = "Created, or the replay of the same command", body = Project),
        (status = 400, description = "PATH_INVALID, PATH_NOT_A_DIRECTORY", body = ErrorBody),
        (status = 403, description = "PATH_ACCESS_DENIED", body = ErrorBody),
        (status = 404, description = "PATH_NOT_FOUND", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT, or STORAGE_CONSTRAINT_VIOLATION: the slug is in use", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE, PATH_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn create_project(
    State(s): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Result<Json<Project>, Failure> {
    // Resolved before the fingerprint is taken, so two spellings of one
    // folder are one request, and a bad path is refused before any write.
    let directory = ProjectDirectory::resolve(std::path::Path::new(&body.directory))?;
    let params = serde_json::json!({
        "slug": body.slug, "name": body.name, "directory": directory.as_str(),
    });
    let c = ctx(body.command_id, "project.create", params);
    Ok(Json(
        s.storage
            .create_project(
                &c,
                &body.slug,
                &body.name,
                &directory,
                &policy::default_modes(),
            )
            .await?,
    ))
}

/// A project's planning threads, oldest first. An unknown project has none.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/threads",
    tag = "threads",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Vec<PlanningThread>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_threads(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<Vec<PlanningThread>>, Failure> {
    Ok(Json(s.storage.list_threads_for_project(&project_id).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateThread {
    /// The idempotency key (spec §3.2), scoped to the project.
    command_id: String,
    title: String,
}

/// Creates a planning thread in a project.
#[utoipa::path(
    post,
    path = "/api/projects/{id}/threads",
    tag = "threads",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = CreateThread,
    responses(
        (status = 200, description = "Created, or the replay of the same command", body = PlanningThread),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn create_thread(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateThread>,
) -> Result<Json<PlanningThread>, Failure> {
    let params = serde_json::json!({ "project": project_id, "title": body.title });
    let c = ctx(body.command_id, "thread.create", params);
    Ok(Json(
        s.storage
            .create_planning_thread(&c, &project_id, &body.title, policy::CLAUDE_CODE)
            .await?,
    ))
}
