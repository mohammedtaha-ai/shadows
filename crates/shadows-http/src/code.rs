//! One job: the routes over the code index (spec §15.7) — a project's three
//! questions, its index status, its links, and the active limit.
//!
//! A person asks over HTTP, so a question is `Asker::Person`: a project
//! outside the scope is refused INVALID_COMMAND, never GRANT_SCOPE. Every
//! handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{Answer, Asker, CodeSettings, ProjectId, ProjectLink, ProjectStatus};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct NameQuery {
    /// The exact name, case-sensitively.
    name: String,
    /// A linked project's slug, to ask only it; none asks the project and
    /// every project it links to.
    project: Option<String>,
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct PathQuery {
    /// A file or folder relative to the project's folder; "" for all of it.
    path: String,
    /// As for `definitions`.
    project: Option<String>,
}

/// Where a name is defined: per hit, its project, path, line, kind and
/// signature (§15.5).
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/definitions",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project asked from"), NameQuery),
    responses(
        (status = 200, body = Answer),
        (status = 400, description = "INVALID_COMMAND: no `name`", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (
            status = 422,
            description = "INVALID_COMMAND: the project named is not linked",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn definitions(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<NameQuery>,
) -> Result<Json<Answer>, Failure> {
    let asker = Asker::Person(&project);
    let only = q.project.as_deref();
    Ok(Json(s.core.code().definitions(asker, only, &q.name).await?))
}

/// Where a name is used, matched by name only: every hit says
/// `matched_by: "name"` (§15.5).
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/references",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project asked from"), NameQuery),
    responses(
        (status = 200, body = Answer),
        (status = 400, description = "INVALID_COMMAND: no `name`", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (
            status = 422,
            description = "INVALID_COMMAND: the project named is not linked",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn references(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<NameQuery>,
) -> Result<Json<Answer>, Failure> {
    let asker = Asker::Person(&project);
    let only = q.project.as_deref();
    Ok(Json(s.core.code().references(asker, only, &q.name).await?))
}

/// The definitions in a file or folder (§15.5). It reads the index and
/// opens no path.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/outline",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project asked from"), PathQuery),
    responses(
        (status = 200, body = Answer),
        (status = 400, description = "INVALID_COMMAND: no `path`", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (
            status = 422,
            description = "INVALID_COMMAND: the path must be inside the project, or the project \
                           named is not linked",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn outline(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<PathQuery>,
) -> Result<Json<Answer>, Failure> {
    let asker = Asker::Person(&project);
    let only = q.project.as_deref();
    Ok(Json(s.core.code().outline(asker, only, &q.path).await?))
}

/// How the project's index stands. Asking does not make it active.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/status",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = ProjectStatus),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn status(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<ProjectStatus>, Failure> {
    Ok(Json(s.core.code().status(&project).await?))
}

/// The projects this project reads, by slug.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/links",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Vec<ProjectLink>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn links(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<ProjectLink>>, Failure> {
    Ok(Json(s.core.code().links(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PutLink {
    /// The idempotency key (spec §13.5), scoped to the project.
    command_id: String,
}

/// Lets the project read `linked`'s index, one way (§15.6). Linking twice
/// answers the link.
#[utoipa::path(
    put,
    path = "/api/projects/{id}/code/links/{linked}",
    tag = "code",
    params(
        ("id" = ProjectId, Path, description = "The project that reads"),
        ("linked" = ProjectId, Path, description = "The project it reads"),
    ),
    request_body = PutLink,
    responses(
        (
            status = 200,
            description = "Linked, or the link already there, or the replay of the same command",
            body = ProjectLink,
        ),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (
            status = 422,
            description = "INVALID_COMMAND: a link to itself, or to no project",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn put_link(
    State(s): State<AppState>,
    Path((project, linked)): Path<(ProjectId, ProjectId)>,
    Json(body): Json<PutLink>,
) -> Result<Json<ProjectLink>, Failure> {
    let code = s.core.code();
    Ok(Json(code.link(body.command_id, &project, &linked).await?))
}

/// A `DELETE` has no body, so its idempotency key rides in the query.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveLinkQuery {
    /// The idempotency key (spec §13.5), scoped to the project.
    command_id: String,
}

/// Removes the link: the project no longer reads `linked`'s index.
#[utoipa::path(
    delete,
    path = "/api/projects/{id}/code/links/{linked}",
    tag = "code",
    params(
        ("id" = ProjectId, Path, description = "The project that reads"),
        ("linked" = ProjectId, Path, description = "The project it reads"),
        RemoveLinkQuery,
    ),
    responses(
        (status = 204, description = "Removed, or the replay of the same command"),
        (status = 400, description = "INVALID_COMMAND: no `command_id`", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "INVALID_COMMAND: no such link", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn remove_link(
    State(s): State<AppState>,
    Path((project, linked)): Path<(ProjectId, ProjectId)>,
    Query(q): Query<RemoveLinkQuery>,
) -> Result<StatusCode, Failure> {
    s.core
        .code()
        .unlink(q.command_id, &project, &linked)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The code index's settings.
#[utoipa::path(
    get,
    path = "/api/code/settings",
    tag = "code",
    responses(
        (status = 200, body = CodeSettings),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn get_settings(State(s): State<AppState>) -> Result<Json<CodeSettings>, Failure> {
    Ok(Json(s.core.code().settings().await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PutSettings {
    /// The idempotency key (spec §13.5).
    command_id: String,
    /// How many projects are active at once: 1 to 20.
    active_limit: u32,
}

/// Sets how many projects are active at once (§15.6). Lowering it stops the
/// least recently used projects' watchers at once.
#[utoipa::path(
    put,
    path = "/api/code/settings",
    tag = "code",
    request_body = PutSettings,
    responses(
        (status = 200, description = "Set, or the replay of the same command", body = CodeSettings),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "INVALID_COMMAND: outside 1 to 20", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn put_settings(
    State(s): State<AppState>,
    Json(body): Json<PutSettings>,
) -> Result<Json<CodeSettings>, Failure> {
    let code = s.core.code();
    Ok(Json(
        code.set_active_limit(body.command_id, body.active_limit)
            .await?,
    ))
}
