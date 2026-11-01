//! Shared extension library HTTP translation.
use super::{AppState, Failure, failure::ErrorBody, transport::EnvJson};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use shadows_core::{
    AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item, McpInput, McpItem,
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct McpRequest {
    command_id: String,
    #[serde(flatten)]
    input: McpInput,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct Import {
    command_id: String,
    kind: ExtensionKind,
    #[schema(value_type=String)]
    path: std::path::PathBuf,
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct Toggle {
    command_id: String,
    enabled: bool,
}
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in=Query)]
pub(super) struct Delete {
    command_id: String,
}

#[utoipa::path(get,path="/api/aylul/extensions",tag="aylul",operation_id="list_extensions",
    responses((status=200,body=ExtensionLibrary),(status=500,body=ErrorBody)))]
pub(super) async fn library(State(s): State<AppState>) -> Result<Json<ExtensionLibrary>, Failure> {
    Ok(Json(s.core.extensions().extensions().await?))
}
#[utoipa::path(post,path="/api/aylul/extensions",tag="aylul",operation_id="import_extension",
    request_body=Import,
    responses((status=201,body=Item),(status=400,body=ErrorBody),(status=409,body=ErrorBody),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn import(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<Import>,
) -> Result<(StatusCode, Json<Item>), Failure> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.core
                .extensions()
                .add_extension(b.command_id, b.kind, &b.path)
                .await?,
        ),
    ))
}

#[utoipa::path(post,path="/api/aylul/extensions/mcp",tag="aylul",operation_id="add_mcp",
    request_body=McpRequest,responses((status=201,body=McpItem),(status=400,body=ErrorBody),
    (status=409,body=ErrorBody),(status=503,body=ErrorBody)))]
pub(super) async fn add_mcp(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<McpRequest>,
) -> Result<(StatusCode, Json<McpItem>), Failure> {
    Ok((
        StatusCode::CREATED,
        Json(s.core.extensions().add_mcp(b.command_id, b.input).await?),
    ))
}

#[utoipa::path(put,path="/api/aylul/extensions/mcp/{id}",tag="aylul",operation_id="update_mcp",
    params(("id"=String,Path)),request_body=McpRequest,responses((status=200,body=McpItem),
    (status=400,body=ErrorBody),(status=404,body=ErrorBody),(status=409,body=ErrorBody),
    (status=503,body=ErrorBody)))]
pub(super) async fn update_mcp(
    State(s): State<AppState>,
    Path(id): Path<String>,
    EnvJson(b): EnvJson<McpRequest>,
) -> Result<Json<McpItem>, Failure> {
    Ok(Json(
        s.core
            .extensions()
            .update_mcp(b.command_id, &id, b.input)
            .await?,
    ))
}

pub(super) async fn remove_mcp(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(b): Query<Delete>,
) -> Result<StatusCode, Failure> {
    s.core
        .extensions()
        .delete_extension(b.command_id, ExtensionKind::Mcp, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(delete,path="/api/aylul/extensions/{kind}/{id}",tag="aylul",
    operation_id="remove_extension",
    params(("kind"=ExtensionKind,Path),("id"=String,Path),Delete),
    responses((status=204),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn remove(
    State(s): State<AppState>,
    Path((kind, id)): Path<(ExtensionKind, String)>,
    Query(b): Query<Delete>,
) -> Result<StatusCode, Failure> {
    s.core
        .extensions()
        .delete_extension(b.command_id, kind, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get,path="/api/aylul/agents/{slug}/extensions",tag="aylul",
    operation_id="agent_extensions",
    params(("slug"=String,Path)),responses((status=200,body=AgentExtensions),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn agent(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<AgentExtensions>, Failure> {
    Ok(Json(s.core.extensions().agent_extensions(&slug).await?))
}
#[utoipa::path(put,path="/api/aylul/agents/{slug}/extensions/{kind}/{id}",tag="aylul",
    operation_id="set_agent_extension",
    params(("slug"=String,Path),("kind"=ExtensionKind,Path),("id"=String,Path)),request_body=Toggle,
    responses((status=200,body=AgentItem),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn toggle(
    State(s): State<AppState>,
    Path((slug, kind, id)): Path<(String, ExtensionKind, String)>,
    EnvJson(b): EnvJson<Toggle>,
) -> Result<Json<AgentItem>, Failure> {
    Ok(Json(
        s.core
            .extensions()
            .set_agent_extension(b.command_id, &slug, kind, &id, b.enabled)
            .await?,
    ))
}
#[cfg(test)]
#[path = "aylul_extensions_tests.rs"]
mod tests;
