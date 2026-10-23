//! Provider service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody, transport::EnvJson};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use shadows_core::{
    EnvEntry, Provider, ProviderInput, ProviderPreset, ProviderTest, ProviderTestOutcome,
    SavedProviderTest,
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ProviderCommand {
    pub(super) command_id: String,
    #[serde(flatten)]
    pub(super) input: ProviderInput,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct KeyCommand {
    command_id: String,
    value: String,
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveCommand {
    pub(super) command_id: String,
}

#[utoipa::path(get, path="/api/aylul/providers", tag="aylul",
    responses((status=200,body=[Provider]),(status=500,body=ErrorBody)))]
pub(super) async fn providers(State(s): State<AppState>) -> Result<Json<Vec<Provider>>, Failure> {
    Ok(Json(s.core.providers().providers().await?))
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
                .providers()
                .create_provider(b.command_id, b.input)
                .await?,
        ),
    ))
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
            .providers()
            .set_key(b.command_id, &slug, &b.value)
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
    s.core.providers().remove_key(q.command_id, &slug).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path="/api/aylul/provider-presets", tag="aylul",
    responses((status=200,body=[ProviderPreset])))]
pub(super) async fn provider_presets(State(s): State<AppState>) -> Json<Vec<ProviderPreset>> {
    Json(s.core.providers().provider_presets())
}

#[utoipa::path(post, path="/api/aylul/providers/test", tag="aylul", request_body=ProviderTest,
    responses((status=200,body=ProviderTestOutcome),(status=400,body=ErrorBody)))]
pub(super) async fn test_provider(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<ProviderTest>,
) -> Result<Json<ProviderTestOutcome>, Failure> {
    Ok(Json(s.core.providers().test_provider(b).await?))
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
            .providers()
            .test_saved_provider(&slug, b.api_key)
            .await?,
    ))
}
