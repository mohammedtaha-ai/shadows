//! One job: the routes under `/api/projects` — projects, and the threads each
//! one holds.
//!
//! Every handler here is `pub(super)`: the route table in the parent is the
//! only thing that names them, and a route handler reachable from outside
//! this crate would be a second, undeclared entry point into the product.

use axum::Json;
use axum::extract::{Path, Query, State};

use std::collections::BTreeMap;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{PlanningThread, Project, ProjectId};

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
    Ok(Json(s.core.projects().list().await?))
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
    let CreateProject {
        command_id,
        slug,
        name,
        directory,
    } = body;
    Ok(Json(
        s.core
            .projects()
            .create(command_id, &slug, &name, &directory)
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
    Ok(Json(s.core.threads().list(&project_id).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateThread {
    /// The idempotency key (spec §3.2), scoped to the project.
    command_id: String,
    title: String,
    /// The CLI for this conversation; `claude-code` when omitted.
    harness: Option<String>,
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
        (status = 422, description = "SETTING_NOT_OFFERED: a harness Shadows does not know", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn create_thread(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<CreateThread>,
) -> Result<Json<PlanningThread>, Failure> {
    Ok(Json(
        s.core
            .threads()
            .create(
                body.command_id,
                &project_id,
                &body.title,
                body.harness.as_deref(),
            )
            .await?,
    ))
}

/// Sets the modes this project allows, per harness (spec §12.5).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct UpdateProject {
    /// The idempotency key (spec §3.2), scoped to the project.
    command_id: String,
    /// Per harness kind, the modes this project allows; a harness not named
    /// keeps its modes. Only modes in Shadows' policy for that harness.
    allowed_modes: BTreeMap<String, Vec<String>>,
}

/// Sets the modes this project allows, per harness (spec §12.5). A turn's
/// mode is checked against them when it starts. A replay answers the project
/// as it now stands.
#[utoipa::path(
    patch,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = UpdateProject,
    responses(
        (status = 200, body = Project),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "SETTING_NOT_OFFERED: a mode outside Shadows' policy", body = ErrorBody),
    )
)]
pub(super) async fn update_project(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Json(body): Json<UpdateProject>,
) -> Result<Json<Project>, Failure> {
    Ok(Json(
        s.core
            .projects()
            .set_modes(body.command_id, &project_id, body.allowed_modes)
            .await?,
    ))
}

/// A `DELETE` has no body, so its idempotency key rides in the query.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveProjectQuery {
    /// The idempotency key (spec §3.2), scoped to the project.
    command_id: String,
}

/// Removes a project that holds no planning thread (spec §4.2): it is listed
/// nowhere again, its code index, its links and its MCP grants go, and its
/// slug stays taken. A replay answers the removed project.
#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = ProjectId, Path, description = "The project"), RemoveProjectQuery),
    responses(
        (status = 200, description = "Removed, or the replay of the same command", body = Project),
        (status = 400, description = "INVALID_COMMAND: no `command_id`", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "PROJECT_HAS_THREADS, or COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn remove_project(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Query(q): Query<RemoveProjectQuery>,
) -> Result<Json<Project>, Failure> {
    Ok(Json(
        s.core.projects().remove(q.command_id, &project_id).await?,
    ))
}
