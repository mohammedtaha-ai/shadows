//! One job: the project's Planner switch routes.
use super::{AppState, Failure, aylul_extensions::Toggle, failure::ErrorBody, transport::EnvJson};
use axum::{
    Json,
    extract::{Path, State},
};
use shadows_core::{AgentExtensions, AgentItem, ExtensionKind, Project, ProjectId};

/// The library's skills and MCP servers, each with whether this project's
/// Planner uses it. `plugins` is always empty.
#[utoipa::path(get,path="/api/projects/{id}/planner-extensions",tag="projects",
    operation_id="project_planner_extensions",
    params(("id"=ProjectId,Path)),responses((status=200,body=AgentExtensions),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn extensions(
    State(s): State<AppState>,
    Path(id): Path<ProjectId>,
) -> Result<Json<AgentExtensions>, Failure> {
    Ok(Json(s.core.extensions().project_extensions(&id).await?))
}

/// Switches one library skill or MCP server on or off for the project's Planner.
#[utoipa::path(put,path="/api/projects/{id}/planner-extensions/{kind}/{item}",tag="projects",
    operation_id="set_project_planner_extension",
    params(("id"=ProjectId,Path),("kind"=ExtensionKind,Path),("item"=String,Path)),
    request_body=Toggle,
    responses((status=200,body=AgentItem),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn set_extension(
    State(s): State<AppState>,
    Path((id, kind, item)): Path<(ProjectId, ExtensionKind, String)>,
    EnvJson(b): EnvJson<Toggle>,
) -> Result<Json<AgentItem>, Failure> {
    Ok(Json(
        s.core
            .extensions()
            .set_project_extension(b.command_id, &id, kind, &item, b.enabled)
            .await?,
    ))
}

/// Switches whether the project's Planner reads the project's CLAUDE.md.
#[utoipa::path(put,path="/api/projects/{id}/planner-claude-md",tag="projects",
    operation_id="set_project_planner_claude_md",
    params(("id"=ProjectId,Path)),request_body=Toggle,
    responses((status=200,body=Project),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn set_claude_md(
    State(s): State<AppState>,
    Path(id): Path<ProjectId>,
    EnvJson(b): EnvJson<Toggle>,
) -> Result<Json<Project>, Failure> {
    Ok(Json(
        s.core
            .projects()
            .set_planner_claude_md(b.command_id, &id, b.enabled)
            .await?,
    ))
}
