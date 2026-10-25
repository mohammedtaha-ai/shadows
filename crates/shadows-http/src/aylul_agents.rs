//! Agent service HTTP translation.
use super::aylul_providers::{ProviderCommand, RemoveCommand};
use super::{AppState, Failure, failure::ErrorBody, transport::EnvJson};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use shadows_core::{Agent, AgentInput, AgentWorkspace, Provider};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct AgentCommand {
    command_id: String,
    #[serde(flatten)]
    input: AgentInput,
}

#[utoipa::path(put, path="/api/aylul/providers/{slug}", tag="aylul",
    params(("slug"=String,Path)),request_body=ProviderCommand,
    responses((status=200,body=Provider),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn update_provider(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    EnvJson(b): EnvJson<ProviderCommand>,
) -> Result<Json<Provider>, Failure> {
    Ok(Json(
        s.core
            .agents()
            .update_provider(b.command_id, &slug, b.input)
            .await?,
    ))
}

#[utoipa::path(delete, path="/api/aylul/providers/{slug}", tag="aylul",
    params(("slug"=String,Path),RemoveCommand),
    responses((status=204),(status=404,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn delete_provider(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<RemoveCommand>,
) -> Result<StatusCode, Failure> {
    s.core.agents().delete_provider(q.command_id, &slug).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path="/api/aylul/agents", tag="aylul",
    responses((status=200,body=AgentWorkspace),(status=404,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn agents(State(s): State<AppState>) -> Result<Json<AgentWorkspace>, Failure> {
    Ok(Json(s.core.agents().workspace().await?))
}

#[utoipa::path(post, path="/api/aylul/agents", tag="aylul",request_body=AgentCommand,
    responses((status=201,body=Agent),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn create_agent(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<AgentCommand>,
) -> Result<(StatusCode, Json<Agent>), Failure> {
    Ok((
        StatusCode::CREATED,
        Json(s.core.agents().create_agent(b.command_id, b.input).await?),
    ))
}

#[utoipa::path(put, path="/api/aylul/agents/{slug}", tag="aylul",
    params(("slug"=String,Path)),request_body=AgentCommand,
    responses((status=200,body=Agent),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn update_agent(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    EnvJson(b): EnvJson<AgentCommand>,
) -> Result<Json<Agent>, Failure> {
    Ok(Json(
        s.core
            .agents()
            .update_agent(b.command_id, &slug, b.input)
            .await?,
    ))
}
