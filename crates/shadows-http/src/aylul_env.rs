//! HTTP translation for AYLUL's write-only environment values.
use axum::Json;
use axum::extract::{FromRequest, Path, Query, Request, State};
use axum::http::StatusCode;
use shadows_core::EnvEntry;

use super::failure::ErrorBody;
use super::{AppState, Failure};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SetEnv {
    command_id: String,
    value: String,
}

// Never expose serde's rejection: it may include the submitted value.
pub(super) struct EnvJson(SetEnv);

impl<S: Send + Sync> FromRequest<S> for EnvJson {
    type Rejection = Failure;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<SetEnv>::from_request(request, state)
            .await
            .map(|Json(body)| Self(body))
            .map_err(|_| Failure::invalid_env_json())
    }
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveEnv {
    command_id: String,
}

#[utoipa::path(get, path = "/api/aylul/env", tag = "aylul",
    responses(
        (status = 200, body = [EnvEntry]),
        (status = 404, description = "AYLUL_NOT_CONFIGURED", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn list(State(s): State<AppState>) -> Result<Json<Vec<EnvEntry>>, Failure> {
    let profile = s.core.harness().aylul_profile()?;
    Ok(Json(s.core.secrets().list_env(profile).await?))
}

#[utoipa::path(put, path = "/api/aylul/env/{name}", tag = "aylul",
    params(("name" = String, Path)), request_body = SetEnv,
    responses(
        (status = 200, body = EnvEntry),
        (status = 400, description = "Invalid environment name, value or JSON body",
            body = ErrorBody),
        (status = 404, description = "AYLUL_NOT_CONFIGURED", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn set(
    State(s): State<AppState>,
    Path(name): Path<String>,
    EnvJson(body): EnvJson,
) -> Result<Json<EnvEntry>, Failure> {
    let profile = s.core.harness().aylul_profile()?;
    Ok(Json(
        s.core
            .secrets()
            .set_env(body.command_id, profile, &name, &body.value)
            .await?,
    ))
}

#[utoipa::path(delete, path = "/api/aylul/env/{name}", tag = "aylul",
    params(("name" = String, Path), RemoveEnv),
    responses(
        (status = 204, description = "Variable removed"),
        (status = 400, description = "Invalid environment name or command query", body = ErrorBody),
        (status = 404, description = "AYLUL_NOT_CONFIGURED", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn remove(
    State(s): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<RemoveEnv>,
) -> Result<StatusCode, Failure> {
    let profile = s.core.harness().aylul_profile()?;
    s.core
        .secrets()
        .remove_env(query.command_id, profile, &name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "aylul_env_tests.rs"]
mod tests;
