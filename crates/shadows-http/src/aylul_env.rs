//! HTTP translation for AYLUL's write-only environment values.
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use shadows_core::EnvEntry;

use super::failure::ErrorBody;
use super::{AppState, Failure, transport::EnvJson};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SetEnv {
    command_id: String,
    value: String,
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RemoveEnv {
    command_id: String,
}

#[utoipa::path(get, path = "/api/aylul/agents/{slug}/env", tag = "aylul",
    params(("slug" = String, Path)),
    responses(
        (status = 200, body = [EnvEntry]),
        (status = 404, description = "AYLUL_PROFILE_NOT_FOUND", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn list(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<EnvEntry>>, Failure> {
    Ok(Json(s.core.agents().list_env(&slug).await?))
}

#[utoipa::path(put, path = "/api/aylul/agents/{slug}/env/{name}", tag = "aylul",
    params(("slug" = String, Path), ("name" = String, Path)), request_body = SetEnv,
    responses(
        (status = 200, body = EnvEntry),
        (status = 400, description = "Invalid environment name, value or JSON body",
            body = ErrorBody),
        (status = 404, description = "AYLUL_PROFILE_NOT_FOUND", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT or SECRET_NAME_RESERVED", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn set(
    State(s): State<AppState>,
    Path((slug, name)): Path<(String, String)>,
    EnvJson(body): EnvJson<SetEnv>,
) -> Result<Json<EnvEntry>, Failure> {
    Ok(Json(
        s.core
            .agents()
            .set_env(body.command_id, &slug, &name, &body.value)
            .await?,
    ))
}

#[utoipa::path(delete, path = "/api/aylul/agents/{slug}/env/{name}", tag = "aylul",
    params(("slug" = String, Path), ("name" = String, Path), RemoveEnv),
    responses(
        (status = 204, description = "Variable removed"),
        (status = 400, description = "Invalid environment name or command query", body = ErrorBody),
        (status = 404, description = "AYLUL_PROFILE_NOT_FOUND", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT or SECRET_NAME_RESERVED", body = ErrorBody),
        (status = 503, description = "SECRETS_UNAVAILABLE", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody)
    ))]
pub(super) async fn remove(
    State(s): State<AppState>,
    Path((slug, name)): Path<(String, String)>,
    Query(query): Query<RemoveEnv>,
) -> Result<StatusCode, Failure> {
    s.core
        .agents()
        .remove_env(query.command_id, &slug, &name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "aylul_env_tests.rs"]
pub(crate) mod tests;
