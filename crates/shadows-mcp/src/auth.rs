//! One job: answering a `/mcp` request that carries no live grant with 401
//! (spec §13.6).
//!
//! `Authorization: Bearer <token>` is looked up by its hash; a missing,
//! unknown or revoked token is refused before `rmcp` reads the body, with an
//! empty 401 and no `WWW-Authenticate` — Claude Code 2.1.281 reports that as a
//! failed connection and tries no OAuth (`MCP_PROBE.md` §5). The grant found
//! travels on in the request's extensions, where the tools read it.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use shadows_core::storage::Storage;

pub(super) async fn require_grant(
    State(storage): State<Arc<Storage>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer(request.headers()) else {
        tracing::info!("mcp.unauthorized");
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match storage.grant_for_token(token).await {
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

/// The token of an `Authorization: Bearer <token>` header; the scheme's case
/// does not matter (RFC 9110 §11.1).
fn bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}
