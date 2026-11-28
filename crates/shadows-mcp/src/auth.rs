//! One job: answering a `/mcp` request that carries no live grant with 401.
//!
//! `Authorization: Bearer <token>` is looked up by its hash; a missing,
//! unknown or revoked token is refused before `rmcp` reads the body, with an
//! empty 401 and no `WWW-Authenticate` — Claude Code 2.1.281 reports that as a
//! failed connection and tries no OAuth. The grant found
//! travels on in the request's extensions, where the tools read it.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use shadows_core::{AppCore, ExternalConnection, Grant, Grants};

#[derive(Clone)]
pub(super) enum Credential {
    Scoped(Grant),
    External(ExternalConnection),
}

pub(super) async fn require_grant(
    State(core): State<Arc<AppCore>>,
    mut request: Request,
    next: Next,
) -> Response {
    let authorization = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let Some(token) = Grants::bearer(authorization) else {
        tracing::info!("mcp.unauthorized");
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let grants = core.grants();
    let checked = async {
        if let Some(connection) = grants.authorize_external(token).await? {
            return Ok(Some(Credential::External(connection)));
        }
        grants
            .authorize(token)
            .await
            .map(|grant| grant.map(Credential::Scoped))
    }
    .await;
    match checked {
        Ok(Some(grant)) => {
            request.extensions_mut().insert(grant);
            next.run(request).await
        }
        Ok(None) => {
            tracing::info!("mcp.unauthorized");
            StatusCode::UNAUTHORIZED.into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "mcp.grant_lookup_failed");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
