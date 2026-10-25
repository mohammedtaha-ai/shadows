//! Person settings HTTP translation.
use super::{AppState, Failure, failure::ErrorBody, transport::EnvJson};
use axum::{Json, extract::State};
use shadows_core::{Person, PersonInput};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PersonCommand {
    command_id: String,
    #[serde(flatten)]
    input: PersonInput,
}

#[utoipa::path(get, path="/api/me", tag="aylul",
    responses((status=200,body=Person),(status=500,body=ErrorBody)))]
pub(super) async fn person(State(s): State<AppState>) -> Result<Json<Person>, Failure> {
    Ok(Json(s.core.person().person().await?))
}

#[utoipa::path(put, path="/api/me", tag="aylul", request_body=PersonCommand,
    responses((status=200,body=Person),(status=400,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn set_person(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<PersonCommand>,
) -> Result<Json<Person>, Failure> {
    Ok(Json(
        s.core.person().set_person(b.command_id, b.input).await?,
    ))
}
