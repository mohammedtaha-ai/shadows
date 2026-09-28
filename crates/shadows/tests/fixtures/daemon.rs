//! Shared apparatus: the daemon's router as `cli::serve` assembles it — the
//! HTTP routes with `/mcp` mounted, built from the same state (spec §14.5).
//! Include it where a test drives the router:
//!
//! ```ignore
//! #[path = "fixtures/daemon.rs"] mod daemon;
//! use daemon::router;
//! ```

use axum::Router;
use shadows_http::AppState;
use shadows_mcp::McpState;

pub fn router(state: AppState) -> Router {
    let mcp = shadows_mcp::service(McpState {
        storage: state.storage.clone(),
        handles: state.handles.clone(),
        ui: state.ui.clone(),
    });
    shadows_http::router(state, mcp)
}
