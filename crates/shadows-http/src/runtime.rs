//! The binary's public readiness identity, separate from application state.
use crate::Failure;
use crate::failure::ErrorBody;
use axum::{Extension, Json};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct RuntimeIdentity {
    pub runtime_id: String,
    pub configuration_id: String,
}

#[utoipa::path(
    get, path = "/api/runtime", tag = "meta",
    responses(
        (status = 200, description = "The ready desktop runtime identity", body = RuntimeIdentity),
        (status = 404, description = "RUNTIME_NOT_CONFIGURED", body = ErrorBody)
    )
)]
pub(super) async fn identity(
    identity: Option<Extension<RuntimeIdentity>>,
) -> Result<Json<RuntimeIdentity>, Failure> {
    identity
        .map(|Extension(value)| Json(value))
        .ok_or_else(Failure::runtime_not_configured)
}
