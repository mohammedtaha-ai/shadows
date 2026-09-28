//! One job: the routes over external agents' MCP grants (spec §13.7, §13.10)
//! — a project's list, issuing one, revoking one.
//!
//! The token is in the issuing answer only, once: a replay answers the grant
//! without it, and the list never carries one. Thread grants are the
//! Planner's and never reach a route. Every handler here is `pub(super)`.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::failure::ErrorBody;
use super::project::ctx;
use super::{AppState, Failure};
use shadows_core::grant::{Grant, GrantId};
use shadows_core::project::ProjectId;

/// A project's grants for external agents, revoked ones included, newest
/// first. Never a token.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/mcp-grants",
    tag = "grants",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Vec<Grant>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_grants(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<Grant>>, Failure> {
    Ok(Json(s.core.storage().list_project_grants(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct IssueGrant {
    /// The idempotency key (spec §13.5), scoped to the project.
    command_id: String,
}

/// A new grant, with its token and the command that connects Claude Code to
/// it — or, for a replayed command, the grant alone.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct IssuedGrantBody {
    grant: Grant,
    /// The bearer token, in this answer only; `null` on a replay.
    token: Option<String>,
    /// `claude mcp add …` with the token, to copy once; `null` on a replay.
    command: Option<String>,
}

/// Connect (§13.7): issues a grant bound to this project for an external
/// agent. The token is shown once — here — and stored only as its hash.
#[utoipa::path(
    post,
    path = "/api/projects/{id}/mcp-grants",
    tag = "grants",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = IssueGrant,
    responses(
        (status = 200, description = "Issued, or the replay of the same command without its token", body = IssuedGrantBody),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn issue_grant(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<IssueGrant>,
) -> Result<Json<IssuedGrantBody>, Failure> {
    let c = ctx(
        body.command_id,
        "McpGrantIssue",
        serde_json::json!({ "project": project }),
    );
    let issued = s.core.storage().issue_project_grant(&c, &project).await?;
    let command = issued.token.as_ref().map(|token| {
        format!(
            "claude mcp add --transport http shadows {} --header \"Authorization: Bearer {}\"",
            s.core.mcp_url(),
            token.as_str()
        )
    });
    Ok(Json(IssuedGrantBody {
        grant: issued.grant,
        token: issued.token.map(|t| t.as_str().to_string()),
        command,
    }))
}

/// A `DELETE` has no body, so its idempotency key rides in the query.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RevokeQuery {
    /// The idempotency key (spec §13.5), scoped to the grant.
    command_id: String,
}

/// Revoke (§13.7): Shadows refuses the grant's token from now on. It does not
/// remove the server from the person's Claude configuration.
#[utoipa::path(
    delete,
    path = "/api/mcp-grants/{id}",
    tag = "grants",
    params(("id" = GrantId, Path, description = "The project grant"), RevokeQuery),
    responses(
        (status = 200, description = "Revoked, or the replay of the same command", body = Grant),
        (status = 400, description = "INVALID_COMMAND: no `command_id`", body = ErrorBody),
        (status = 404, description = "INVALID_COMMAND: no such project grant", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn revoke_grant(
    State(s): State<AppState>,
    Path(grant): Path<GrantId>,
    Query(q): Query<RevokeQuery>,
) -> Result<Json<Grant>, Failure> {
    let c = ctx(
        q.command_id,
        "McpGrantRevoke",
        serde_json::json!({ "grant": grant }),
    );
    Ok(Json(s.core.storage().revoke_grant(&c, &grant).await?))
}
