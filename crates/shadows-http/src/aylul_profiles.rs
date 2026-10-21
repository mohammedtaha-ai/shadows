//! Provider and agent HTTP translation; configuration rules live in AppCore.
use super::{AppState, Failure, aylul_env::EnvJson, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use shadows_core::{
    Agent, AgentInput, AgentWorkspace, EnvEntry, Person, PersonInput, PlanningThread, Provider,
    ProviderInput, ProviderPreset, ProviderTest, ProviderTestOutcome, SavedProviderTest,
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ProviderCommand {
    command_id: String,
    #[serde(flatten)]
    input: ProviderInput,
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct AgentCommand {
    command_id: String,
    #[serde(flatten)]
    input: AgentInput,
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct KeyCommand {
    command_id: String,
    value: String,
}
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveCommand {
    command_id: String,
}

#[utoipa::path(get, path="/api/aylul/providers", tag="aylul",
    responses((status=200,body=[Provider]),(status=500,body=ErrorBody)))]
pub(super) async fn providers(State(s): State<AppState>) -> Result<Json<Vec<Provider>>, Failure> {
    Ok(Json(s.core.profiles().providers().await?))
}
#[utoipa::path(post, path="/api/aylul/providers", tag="aylul", request_body=ProviderCommand,
    responses((status=201,body=Provider),(status=400,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn create_provider(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<ProviderCommand>,
) -> Result<(StatusCode, Json<Provider>), Failure> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.core
                .profiles()
                .create_provider(b.command_id, b.input)
                .await?,
        ),
    ))
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
            .profiles()
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
    s.core
        .profiles()
        .delete_provider(q.command_id, &slug)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(put, path="/api/aylul/providers/{slug}/key", tag="aylul",
    params(("slug"=String,Path)),request_body=KeyCommand,
    responses((status=200,body=EnvEntry),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=503,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn set_key(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    EnvJson(b): EnvJson<KeyCommand>,
) -> Result<Json<EnvEntry>, Failure> {
    Ok(Json(
        s.core
            .secrets()
            .set_provider_key(b.command_id, &slug, &b.value)
            .await?,
    ))
}
#[utoipa::path(delete, path="/api/aylul/providers/{slug}/key", tag="aylul",
    params(("slug"=String,Path),RemoveCommand),
    responses((status=204),(status=404,body=ErrorBody),(status=409,body=ErrorBody),
        (status=503,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn remove_key(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<RemoveCommand>,
) -> Result<StatusCode, Failure> {
    s.core
        .secrets()
        .remove_provider_key(q.command_id, &slug)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path="/api/aylul/agents", tag="aylul",
    responses((status=200,body=AgentWorkspace),(status=404,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn agents(State(s): State<AppState>) -> Result<Json<AgentWorkspace>, Failure> {
    Ok(Json(s.core.profiles().workspace().await?))
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
        Json(
            s.core
                .profiles()
                .create_agent(b.command_id, b.input)
                .await?,
        ),
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
            .profiles()
            .update_agent(b.command_id, &slug, b.input)
            .await?,
    ))
}
#[utoipa::path(delete, path="/api/aylul/agents/{slug}", tag="aylul",
    params(("slug"=String,Path),RemoveCommand),
    responses((status=204),(status=404,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn delete_agent(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<RemoveCommand>,
) -> Result<StatusCode, Failure> {
    s.core.harness().delete_agent(q.command_id, &slug).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path="/api/aylul/agents/{slug}/threads", tag="aylul",
    params(("slug"=String,Path)),
    responses((status=200,body=[PlanningThread]),(status=500,body=ErrorBody)))]
pub(super) async fn threads(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<PlanningThread>>, Failure> {
    Ok(Json(s.core.threads().list_for_profile(&slug).await?))
}
#[cfg(test)]
#[path = "aylul_profiles_tests.rs"]
mod tests;

#[utoipa::path(get, path="/api/aylul/provider-presets", tag="aylul",
    responses((status=200,body=[ProviderPreset])))]
pub(super) async fn provider_presets(State(s): State<AppState>) -> Json<Vec<ProviderPreset>> {
    Json(s.core.profiles().provider_presets())
}

#[utoipa::path(post, path="/api/aylul/providers/test", tag="aylul", request_body=ProviderTest,
    responses((status=200,body=ProviderTestOutcome),(status=400,body=ErrorBody)))]
pub(super) async fn test_provider(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<ProviderTest>,
) -> Result<Json<ProviderTestOutcome>, Failure> {
    Ok(Json(s.core.profiles().test_provider(b).await?))
}

#[utoipa::path(post, path="/api/aylul/providers/{slug}/test", tag="aylul",
    params(("slug"=String,Path)), request_body=SavedProviderTest,
    responses((status=200,body=ProviderTestOutcome),(status=400,body=ErrorBody),
        (status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=503,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn test_saved_provider(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    EnvJson(b): EnvJson<SavedProviderTest>,
) -> Result<Json<ProviderTestOutcome>, Failure> {
    Ok(Json(
        s.core
            .harness()
            .test_saved_provider(&slug, b.api_key)
            .await?,
    ))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PersonCommand {
    command_id: String,
    #[serde(flatten)]
    input: PersonInput,
}

#[utoipa::path(get, path="/api/me", tag="aylul",
    responses((status=200,body=Person),(status=500,body=ErrorBody)))]
pub(super) async fn person(State(s): State<AppState>) -> Result<Json<Person>, Failure> {
    Ok(Json(s.core.profiles().person().await?))
}

#[utoipa::path(put, path="/api/me", tag="aylul", request_body=PersonCommand,
    responses((status=200,body=Person),(status=400,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn set_person(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<PersonCommand>,
) -> Result<Json<Person>, Failure> {
    Ok(Json(
        s.core.profiles().set_person(b.command_id, b.input).await?,
    ))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct UsagePeriod {
    from: String,
    to: String,
}

#[utoipa::path(get, path="/api/aylul/usage", tag="aylul", params(UsagePeriod),
    responses((status=200,body=shadows_core::UsageReport),(status=400,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn usage(
    State(s): State<AppState>,
    Query(q): Query<UsagePeriod>,
) -> Result<Json<shadows_core::UsageReport>, Failure> {
    Ok(Json(s.core.profiles().usage(&q.from, &q.to).await?))
}
