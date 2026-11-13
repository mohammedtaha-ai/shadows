//! One job: the routes over the code index — a project's three
//! questions, its index status, its file list, its links, and the active limit.
//!
//! A person asks over HTTP, so a question is `Asker::Person`: a project
//! outside the scope is refused INVALID_COMMAND, never GRANT_SCOPE. Every
//! handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{
    Answer, Asker, InventoryPage, InventoryQuery, ProjectId, ProjectLink, ProjectStatus,
};

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
/// signature.
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
    Ok(Json(
        s.core.codebase().definitions(asker, only, &q.name).await?,
    ))
}

/// Where a name is used, matched by name only: every hit says
/// `matched_by: "name"`.
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
    Ok(Json(
        s.core.codebase().references(asker, only, &q.name).await?,
    ))
}

/// The definitions in a file or folder. It reads the index and
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
    Ok(Json(s.core.codebase().outline(asker, only, &q.path).await?))
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
    Ok(Json(s.core.codebase().status(&project).await?))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct FilesQuery {
    /// A folder relative to the project's folder; none for all of it.
    prefix: Option<String>,
    /// The `next` of the page before this one.
    after: Option<String>,
    /// 1 to 200; 100 when absent.
    limit: Option<u16>,
}

/// One page of the files the project's folder holds, in path order: files,
/// folders, and what is left out with its reason. A file's content is never
/// read.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/files",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project"), FilesQuery),
    responses(
        (status = 200, body = InventoryPage),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (
            status = 409,
            description = "INVENTORY_STALE: the file list changed since the cursor's page",
            body = ErrorBody,
        ),
        (
            status = 422,
            description = "INVALID_COMMAND: a limit outside 1 to 200, a path outside the \
                           project, or a cursor this listing did not give",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn files(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<FilesQuery>,
) -> Result<Json<InventoryPage>, Failure> {
    let default = InventoryQuery::default();
    let query = InventoryQuery {
        prefix: q.prefix.unwrap_or(default.prefix),
        after: q.after,
        limit: q.limit.unwrap_or(default.limit),
    };
    let asker = Asker::Person(&project);
    Ok(Json(s.core.codebase().inventory(asker, query).await?))
}

/// Lists the project's folder again now, and answers the first page.
#[utoipa::path(
    post,
    path = "/api/projects/{id}/code/refresh",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = InventoryPage),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn refresh(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<InventoryPage>, Failure> {
    let asker = Asker::Person(&project);
    Ok(Json(s.core.codebase().refresh_inventory(asker).await?))
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
    Ok(Json(s.core.codebase().links(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PutLink {
    /// The idempotency key, scoped to the project.
    command_id: String,
}

/// Lets the project read `linked`'s index, one way. Linking twice
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
    Ok(Json(
        s.core
            .plans()
            .link_projects(body.command_id, &project, &linked)
            .await?,
    ))
}

/// A `DELETE` has no body, so its idempotency key rides in the query.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveLinkQuery {
    /// The idempotency key, scoped to the project.
    command_id: String,
}

/// Removes the link, ending this project's access to `linked`'s index.
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
        .plans()
        .unlink_projects(q.command_id, &project, &linked)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
