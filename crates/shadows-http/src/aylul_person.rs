//! Person settings HTTP translation.
use super::{AppState, Failure, failure::ErrorBody, transport::EnvJson};
use axum::{Json, extract::State};
use shadows_core::{Person, PersonPatch};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PersonCommand {
    command_id: String,
    #[serde(flatten)]
    input: PersonPatch,
}

#[utoipa::path(get, path="/api/me", tag="aylul",
    responses((status=200,body=Person),(status=500,body=ErrorBody)))]
pub(super) async fn person(State(s): State<AppState>) -> Result<Json<Person>, Failure> {
    Ok(Json(s.core.person().person().await?))
}

#[utoipa::path(patch, path="/api/me", tag="aylul", operation_id="set_person",
    request_body=PersonCommand,
    responses((status=200,body=Person),(status=400,body=ErrorBody),(status=409,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn update_person(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<PersonCommand>,
) -> Result<Json<Person>, Failure> {
    Ok(Json(
        s.core.person().update_person(b.command_id, b.input).await?,
    ))
}
