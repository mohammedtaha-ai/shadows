//! One job: the routes under `/api/projects` — projects, and the threads each
//! one holds.
//!
//! Every handler here is `pub(super)`: the route table in the parent is the
//! only thing that names them, and a route handler reachable from outside
//! `protocol/` would be a second, undeclared entry point into the product.

use axum::Json;
use axum::extract::{Path, State};

use std::collections::BTreeMap;

use super::failure::ErrorBody;
use super::thread::known_harness;
use super::{AppState, Failure};
use shadows_agent::policy;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::project::{Project, ProjectDirectory, ProjectId};
use shadows_core::thread::PlanningThread;

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
    Ok(Json(s.core.storage().list_projects().await?))
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
        s.core
            .storage()
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
    Ok(Json(
        s.core
            .storage()
            .list_threads_for_project(&project_id)
            .await?,
    ))
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
    let harness = body.harness.as_deref().unwrap_or(policy::CLAUDE_CODE);
    known_harness(harness)?;
    let params = serde_json::json!({
        "project": project_id, "title": body.title, "harness": harness,
    });
    let c = ctx(body.command_id, "thread.create", params);
    Ok(Json(
        s.core
            .storage()
            .create_planning_thread(&c, &project_id, &body.title, harness)
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
    let mut modes = BTreeMap::new();
    for (harness, list) in body.allowed_modes {
        known_harness(&harness)?;
        let policy = policy::allowed_modes(&harness);
        let mut set: Vec<String> = Vec::new();
        for mode in list {
            if !policy.contains(&mode.as_str()) {
                return Err(Failure::setting_not_offered("mode", &mode, None));
            }
            if !set.contains(&mode) {
                set.push(mode);
            }
        }
        // A set: the order it was sent in is not part of the request.
        set.sort();
        modes.insert(harness, set);
    }
    let params = serde_json::json!({ "project": project_id, "allowed_modes": modes });
    let c = ctx(body.command_id, "project.modes", params);
    Ok(Json(
        s.core
            .storage()
            .set_project_modes(&c, &project_id, &modes)
            .await?,
    ))
}
