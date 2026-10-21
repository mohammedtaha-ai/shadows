//! Shared extension library HTTP translation.
use super::{AppState, Failure, aylul_env::EnvJson, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use shadows_core::{AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item};

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

#[utoipa::path(get,path="/api/aylul/extensions",tag="aylul",
    responses((status=200,body=ExtensionLibrary),(status=500,body=ErrorBody)))]
pub(super) async fn library(State(s): State<AppState>) -> Result<Json<ExtensionLibrary>, Failure> {
    Ok(Json(s.core.profiles().extensions().await?))
}
#[utoipa::path(post,path="/api/aylul/extensions",tag="aylul",request_body=Import,
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
                .profiles()
                .add_extension(b.command_id, b.kind, &b.path)
                .await?,
        ),
    ))
}
#[utoipa::path(delete,path="/api/aylul/extensions/{kind}/{id}",tag="aylul",
    params(("kind"=ExtensionKind,Path),("id"=String,Path),Delete),
    responses((status=204),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn remove(
    State(s): State<AppState>,
    Path((kind, id)): Path<(ExtensionKind, String)>,
    Query(b): Query<Delete>,
) -> Result<StatusCode, Failure> {
    s.core
        .profiles()
        .delete_extension(b.command_id, kind, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get,path="/api/aylul/agents/{slug}/extensions",tag="aylul",
    params(("slug"=String,Path)),responses((status=200,body=AgentExtensions),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn agent(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<AgentExtensions>, Failure> {
    Ok(Json(s.core.profiles().agent_extensions(&slug).await?))
}
#[utoipa::path(put,path="/api/aylul/agents/{slug}/extensions/{kind}/{id}",tag="aylul",
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
            .profiles()
            .set_agent_extension(b.command_id, &slug, kind, &id, b.enabled)
            .await?,
    ))
}
#[cfg(test)]
#[path = "aylul_extensions_tests.rs"]
mod tests;
