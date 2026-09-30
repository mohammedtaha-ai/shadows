//! One job: wiring. What shared state a route may reach, which path reaches
//! which handler, and which origins may call at all.
//!
//! CLAUDE.md names `shadows-http` an accretion point: every feature this project
//! ever adds puts a route here. So routes are split by domain, and this file
//! only wires them: `project.rs` (projects and their threads),
//! `conversation.rs` (entries, a thread's turns, starting and stopping one),
//! `harness.rs` (the harnesses and a thread's session), `thread.rs` (changing
//! a thread itself), `workflow.rs` (plan versions and their approval),
//! `grants.rs` (external agents' MCP grants), `instructions.rs` (a project's
//! Planner instructions), `code.rs` (the code index: questions, links, settings),
//! `sse.rs` (the replay-then-live stream), `fs.rs` (choosing a project directory),
//! `openapi.rs` (the document describing all of it), `failure.rs` (the
//! transport mapping), `guard.rs` (refusing requests pages were made to send).
//! A new feature adds a file or a route to one of them.
//!
//! This module is also the sole owner of HTTP and SSE types (CLAUDE.md). None
//! of them appear in a domain or application signature; a handler is where
//! `axum` stops.

mod code;
mod conversation;
mod failure;
mod fs;
mod grants;
mod guard;
mod harness;
mod instructions;
mod openapi;
mod project;
pub mod sse;
mod thread;
mod workflow;

pub use failure::Failure;
pub use openapi::document as openapi_document;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, header};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use shadows_core::AppCore;

#[derive(Clone)]
pub struct AppState {
    /// The application every route calls (spec §14.5).
    pub core: Arc<AppCore>,
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
///
/// `mcp` is `/mcp` (`shadows_mcp::service`), passed in so this crate does not
/// depend on the MCP adapter (spec §14.5). It is mounted outside
/// `rejections_as_error_bodies` and inside the guard, as it always was.
pub fn router(state: AppState, mcp: Router) -> Router {
    let cors = cors(&state.allowed_origins);
    let guard = axum::middleware::from_fn_with_state(state.clone(), guard::refuse_foreign_pages);
    let (routes, _document) = routes().split_for_parts();
    routes
        .with_state(state.clone())
        // Innermost: an extractor's plain-text refusal becomes an `ErrorBody`
        // before anything outside adds its headers to it.
        .layer(axum::middleware::map_response(
            failure::rejections_as_error_bodies,
        ))
        // `/mcp` is MCP's own transport (§13.6): its refusals are not
        // `ErrorBody`s, so it joins after that layer and before the guard.
        .merge(mcp)
        // Inside the CORS layer: a preflight is answered before it gets here,
        // and a refusal sent to an allowed origin still carries the header
        // that lets that client read why.
        .layer(guard)
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

/// The route table, and with it the OpenAPI document's paths: a route exists
/// here or not at all, so the document cannot list a route the router lacks
/// or miss one it has. Each `routes!` groups the methods of one path.
fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(openapi::base())
        .routes(routes!(project::list_projects, project::create_project))
        .routes(routes!(project::update_project, project::remove_project))
        .routes(routes!(project::list_threads, project::create_thread))
        .routes(routes!(conversation::list_entries))
        .routes(routes!(conversation::list_operations))
        .routes(routes!(conversation::start_turn))
        .routes(routes!(conversation::stop_turn))
        .routes(routes!(harness::list_harnesses))
        .routes(routes!(harness::open_session))
        .routes(routes!(harness::change_model))
        .routes(routes!(harness::thread_context))
        .routes(routes!(thread::update_thread))
        .routes(routes!(thread::fork_thread))
        .routes(routes!(workflow::list_plans))
        .routes(routes!(workflow::get_plan))
        .routes(routes!(workflow::approve_plan))
        .routes(routes!(grants::list_grants, grants::issue_grant))
        .routes(routes!(grants::revoke_grant))
        .routes(routes!(
            instructions::get_instructions,
            instructions::save_instructions
        ))
        .routes(routes!(code::definitions))
        .routes(routes!(code::references))
        .routes(routes!(code::outline))
        .routes(routes!(code::status))
        .routes(routes!(code::links))
        .routes(routes!(code::put_link, code::remove_link))
        .routes(routes!(code::get_settings, code::put_settings))
        .routes(routes!(sse::subscribe))
        .routes(routes!(fs::list_dirs, fs::create_dir))
        .routes(routes!(openapi::serve))
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
/// response. The methods and headers are exactly what the routes use: `GET`,
/// `POST`, `PUT`, `PATCH` and `DELETE` (revoking a grant), JSON bodies, and `Last-Event-ID`, which a browser's
/// `EventSource` sends when it reconnects a stream. No credentials: the API
/// has none to send (spec §1's OPEN block on remote access).
fn cors(origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("last-event-id"),
        ])
        .max_age(Duration::from_secs(600))
}
