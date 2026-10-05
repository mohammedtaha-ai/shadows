//! One job: Shadows' MCP server (spec §13.6), served at `/mcp`.
//!
//! `shadows_core::grants` says who may do what (§13.7); `auth.rs` answers a request
//! without a live grant with 401; `server.rs` is the `rmcp` handler, which
//! lists the tools a grant's kind holds; `tools.rs` maps each tool onto one
//! `Plans` method; `refusal.rs` is what a tool answers, refusals included. The route
//! itself is mounted by `shadows_http::router`, under the daemon's request guard.
//!
//! Streamable HTTP without protocol sessions: `rmcp` serves MCP `2026-07-28`
//! statelessly always, and the `2025-11-25` fallback Claude Code 2.1.281 uses
//! statelessly because `legacy_session_mode` is off (`MCP_PROBE.md` §4). Every
//! request stands alone, with its own bearer.

mod agreement_tools;
mod auth;
mod refusal;
mod server;
mod tools;
mod workspace_tools;

use std::sync::Arc;

use axum::Router;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};

use shadows_core::AppCore;

/// `/mcp`: the bearer check, then `rmcp`'s Streamable HTTP service.
///
/// `rmcp`'s own `Host` check stays at its default, loopback names on any port,
/// which is what the daemon binds; its `Origin` list stays empty, because the
/// daemon's guard in front of this router owns `Origin`.
pub fn service(core: Arc<AppCore>) -> Router {
    let tools = server::Tools::new();
    let handler_core = core.clone();
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let mcp = StreamableHttpService::new(
        move || Ok(server::Shadows::new(handler_core.clone(), tools.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    Router::new()
        .route_service("/mcp", mcp)
        .route_layer(axum::middleware::from_fn_with_state(
            core,
            auth::require_grant,
        ))
}
