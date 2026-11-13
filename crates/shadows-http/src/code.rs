//! One job: the routes over a project's code — its index status, its file
//! list and its links.
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
    AddedServer, Asker, InventoryPage, InventoryQuery, ProjectId, ProjectLink, ProjectStatus,
    ServerStatus,
};

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

/// Every catalog and added server, as this project sees it: whether its files
/// want it, and its state. Asking does not make the project active.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/code/servers",
    tag = "code",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Vec<ServerStatus>),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn servers(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<ServerStatus>>, Failure> {
    Ok(Json(
        s.core.codebase().servers(Asker::Person(&project)).await?,
    ))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PutServerOff {
    /// The idempotency key.
    command_id: String,
    /// `true` switches the server off for every project; `false` back on.
    off: bool,
}

/// Switches a server off, or back on. Shadows' code answers get worse without it.
#[utoipa::path(
    put,
    path = "/api/code/servers/{server}/off",
    tag = "code",
    params(("server" = String, Path, description = "The server's id")),
    request_body = PutServerOff,
    responses(
        (status = 204, description = "Set, or the replay of the same command"),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "INVALID_COMMAND: no such server", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn put_server_off(
    State(s): State<AppState>,
    Path(server): Path<String>,
    Json(body): Json<PutServerOff>,
) -> Result<StatusCode, Failure> {
    let code = s.core.codebase();
    code.set_server_off(body.command_id, &server, body.off)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PostServer {
    /// The idempotency key.
    command_id: String,
    #[serde(flatten)]
    server: AddedServer,
}

/// Adds a server beside the catalog, on the person's own responsibility.
#[utoipa::path(
    post,
    path = "/api/code/servers",
    tag = "code",
    request_body = PostServer,
    responses(
        (
            status = 200,
            description = "Added, or the replay of the same command",
            body = ServerStatus,
        ),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (
            status = 422,
            description = "INVALID_COMMAND: a catalog id, an id already added, or an extension \
                           without a dot",
            body = ErrorBody,
        ),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn post_server(
    State(s): State<AppState>,
    Json(body): Json<PostServer>,
) -> Result<Json<ServerStatus>, Failure> {
    let code = s.core.codebase();
    Ok(Json(code.add_server(body.command_id, body.server).await?))
}

/// A `DELETE` has no body, so its idempotency key rides in the query.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveServerQuery {
    /// The idempotency key.
    command_id: String,
}

/// Removes a server the person added.
#[utoipa::path(
    delete,
    path = "/api/code/servers/{server}",
    tag = "code",
    params(("server" = String, Path, description = "The added server's id"), RemoveServerQuery),
    responses(
        (status = 204, description = "Removed, or the replay of the same command"),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "INVALID_COMMAND: no such added server", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn remove_server(
    State(s): State<AppState>,
    Path(server): Path<String>,
    Query(q): Query<RemoveServerQuery>,
) -> Result<StatusCode, Failure> {
    s.core
        .codebase()
        .remove_server(q.command_id, &server)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
