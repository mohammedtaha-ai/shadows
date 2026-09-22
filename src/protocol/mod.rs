//! One job: wiring. What shared state a route may reach, which path reaches
//! which handler, and which origins may call at all.
//!
//! CLAUDE.md names `protocol/` an accretion point: every feature this project
//! ever adds puts a route here. So the split is made on the way in — this file
//! holds the wiring, `handlers.rs` holds what each route does, `sse.rs` holds
//! the replay-then-live stream, `failure.rs` holds the transport mapping,
//! `fs.rs` holds the disk routes for choosing a project directory.
//!
//! This module is also the sole owner of HTTP and SSE types (CLAUDE.md). None
//! of them appear in a domain or application signature; a handler is where
//! `axum` stops.

mod failure;
mod fs;
mod handlers;
pub mod sse;

pub use failure::Failure;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, header};
use axum::routing::{get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::agent::StreamItem;
use crate::agent::claude::ClaudeHarness;
use crate::operation::OperationId;
use crate::planner::LiveHandles;
use crate::runtime::Runtime;
use crate::storage::Storage;
use crate::thread::ThreadId;

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<Runtime>,
    pub storage: Arc<Storage>,
    pub handles: Arc<LiveHandles>,
    pub harness: Arc<ClaudeHarness>,
    pub bus: tokio::sync::broadcast::Sender<(ThreadId, OperationId, StreamItem)>,
    /// Spec §1: the only origins a browser may call this daemon from. Every
    /// client is cross-origin, because the daemon serves no page. Validated
    /// by `config::allowed_origin` before it gets here.
    pub allowed_origins: Vec<String>,
    /// Becomes `true` once the daemon is stopping. A live stream has no end of
    /// its own, and a graceful HTTP shutdown waits for every open response to
    /// finish — so without this, one open browser tab holds the daemon up
    /// forever after its stop signal. A dropped sender means the same thing.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}

/// Spec §1: the daemon serves no page — there is no `GET /`. A client is
/// served from its own origin and reaches this API cross-origin.
pub fn router(state: AppState) -> Router {
    let cors = cors(&state.allowed_origins);
    Router::new()
        .route(
            "/api/projects",
            get(handlers::list_projects).post(handlers::create_project),
        )
        .route(
            "/api/projects/{id}/threads",
            get(handlers::list_threads).post(handlers::create_thread),
        )
        .route("/api/threads/{id}/entries", get(handlers::list_entries))
        .route("/api/threads/{id}/turns", post(handlers::start_turn))
        .route("/api/operations/{id}/stop", post(handlers::stop_turn))
        .route("/api/subscribe", get(sse::subscribe))
        .route("/api/fs/dirs", get(fs::list_dirs).post(fs::create_dir))
        .with_state(state)
        // Inside the trace layer, so a refused or answered preflight is
        // logged like any other request.
        .layer(cors)
        // One `http.response` line per request: method, path, status,
        // latency. The path is logged without its query string and no body
        // ever is — a body can hold a prompt; at debug the request's size is
        // added, from `Content-Length`, never its contents. For
        // `/api/subscribe` the latency is time to the stream's headers, not
        // its lifetime; `sse.closed` records when it ends.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<Body>| {
                    tracing::info_span!("http", method = %req.method(), path = %req.uri().path())
                })
                .on_request(|req: &Request<Body>, _: &tracing::Span| {
                    tracing::debug!(bytes = content_length(req.headers()), "http.request");
                })
                .on_response(|res: &Response<Body>, latency: Duration, _: &tracing::Span| {
                    tracing::info!(
                        status = res.status().as_u16(),
                        latency_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX),
                        "http.response"
                    );
                })
                // A 5xx is logged by `Failure` with its cause; the default
                // failure line would repeat it without one.
                .on_failure(()),
        )
}

fn content_length(headers: &axum::http::HeaderMap) -> Option<u64> {
    headers
        .get(axum::http::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

/// Cross-origin access for the configured origins only; any other origin's
/// request gets no `Access-Control-Allow-Origin` and the browser withholds the
/// response. The methods and headers are exactly what the routes use: `GET`
/// and `POST`, JSON bodies, and `Last-Event-ID`, which a browser's
/// `EventSource` sends when it reconnects a stream. No credentials: the API
/// has none to send (spec §1's OPEN block on remote access).
fn cors(origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("last-event-id"),
        ])
        .max_age(Duration::from_secs(600))
}
