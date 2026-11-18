//! HTTP translation for external connection issue, list and revocation.

use axum::Json;
use axum::extract::{Path, Query, State};
use shadows_core::{ExternalConnection, ExternalConnectionId, ProjectId};

use super::failure::ErrorBody;
use super::{AppState, Failure};

#[utoipa::path(
    get,
    path = "/api/external-connections",
    tag = "grants",
    responses(
        (status = 200, body = Vec<ExternalConnection>),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn list_grants(
    State(s): State<AppState>,
) -> Result<Json<Vec<ExternalConnection>>, Failure> {
    Ok(Json(s.core.grants().list_external().await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct IssueGrant {
    command_id: String,
    project_ids: Vec<ProjectId>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct IssuedGrantBody {
    connection: ExternalConnection,
    /// The bearer token on first issue. A replay omits the token.
    token: Option<String>,
    mcp_url: String,
}

#[utoipa::path(
    post,
    path = "/api/external-connections",
    tag = "grants",
    request_body = IssueGrant,
    responses(
        (status = 200, body = IssuedGrantBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn issue_grant(
    State(s): State<AppState>,
    Json(body): Json<IssueGrant>,
) -> Result<Json<IssuedGrantBody>, Failure> {
    let issued = s
        .core
        .grants()
        .issue_external(body.command_id, body.project_ids)
        .await?;
    Ok(Json(IssuedGrantBody {
        connection: issued.connection,
        token: issued.token,
        mcp_url: issued.mcp_url,
    }))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RevokeQuery {
    command_id: String,
}

#[utoipa::path(
    delete,
    path = "/api/external-connections/{id}",
    tag = "grants",
    params(
        ("id" = ExternalConnectionId, Path, description = "The external connection"),
        RevokeQuery
    ),
    responses(
        (status = 200, body = ExternalConnection),
        (status = 400, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn revoke_grant(
    State(s): State<AppState>,
    Path(id): Path<ExternalConnectionId>,
    Query(q): Query<RevokeQuery>,
) -> Result<Json<ExternalConnection>, Failure> {
    Ok(Json(
        s.core.grants().revoke_external(q.command_id, &id).await?,
    ))
}
