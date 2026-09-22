//! One job: wiring. What shared state a route may reach, which path reaches
//! which handler, and how a failure becomes a status code.
//!
//! CLAUDE.md names `protocol/` an accretion point: every feature this project
//! ever adds puts a route here. So the split is made on the way in — this file
//! holds the wiring, `handlers.rs` holds what each route does, `sse.rs` holds
//! the replay-then-live stream.
//!
//! This module is also the sole owner of HTTP and SSE types (CLAUDE.md). None
//! of them appear in a domain or application signature; a handler is where
//! `axum` stops.

mod handlers;
pub mod sse;

use std::sync::Arc;

use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};

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
    pub project_root: std::path::PathBuf,
    /// Becomes `true` once the daemon is stopping. A live stream has no end of
    /// its own, and a graceful HTTP shutdown waits for every open response to
    /// finish — so without this, one open browser tab holds the daemon up
    /// forever after its stop signal. A dropped sender means the same thing.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
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
        .with_state(state)
}

/// The whole web client. Spec §1.0: the daemon serves it and never opens it.
async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

/// Transport mapping lives here and nowhere else. Spec §3.3: `Blocked` and
/// `Rejected` are domain outcomes, not HTTP failures, and would be returned as
/// 200 with the outcome — they are not reachable in Milestone 0.
pub struct Failure(crate::storage::StorageError);

impl From<crate::storage::StorageError> for Failure {
    fn from(e: crate::storage::StorageError) -> Self {
        Failure(e)
    }
}

impl axum::response::IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        use crate::error::ErrorCode;
        use crate::storage::StorageError as E;
        let (status, code) = match &self.0 {
            E::CommandConflict => (axum::http::StatusCode::CONFLICT, ErrorCode::CommandConflict),
            E::NotFound(_) => (axum::http::StatusCode::NOT_FOUND, ErrorCode::InvalidCommand),
            E::TransitionConflict { .. } => (
                axum::http::StatusCode::CONFLICT,
                ErrorCode::StorageConstraintViolation,
            ),
            _ => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::StorageUnavailable,
            ),
        };
        (
            status,
            Json(serde_json::json!({ "code": code, "message": self.0.to_string() })),
        )
            .into_response()
    }
}
