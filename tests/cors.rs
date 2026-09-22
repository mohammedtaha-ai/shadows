//! Cross-origin access (spec §1): every client is on another origin, so the
//! daemon answers a browser only for origins it was configured with. Driven
//! through the real router; `tests/serve_smoke.rs` checks the binary's default
//! list reaches it.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use shadows::agent::claude::ClaudeHarness;
use shadows::config::{ConfigError, allowed_origin};
use shadows::planner::LiveHandles;
use shadows::protocol::{AppState, router};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use tower::ServiceExt;

const ALLOWED: &str = "http://localhost:5173";

async fn app(tmp: &tempfile::TempDir) -> (Router, tokio::sync::watch::Sender<bool>) {
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        runtime: Arc::new(runtime),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(
            tmp.path().join("claude.exe"),
            "test".into(),
        )),
        bus,
        allowed_origins: vec![ALLOWED.to_string()],
        shutdown,
    });
    (app, stopping)
}

fn preflight(origin: &str) -> Request<Body> {
    Request::builder()
        .method("OPTIONS")
        .uri("/api/projects")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .body(Body::empty())
        .unwrap()
}

fn header_of(response: &axum::response::Response, name: header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .map(|v| v.to_str().unwrap().to_ascii_lowercase())
}

#[tokio::test]
async fn a_preflight_from_an_allowed_origin_is_answered() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(&tmp).await;

    let response = app.oneshot(preflight(ALLOWED)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(ALLOWED)
    );
    let methods = header_of(&response, header::ACCESS_CONTROL_ALLOW_METHODS).unwrap();
    assert!(methods.contains("post"), "{methods}");
    let headers = header_of(&response, header::ACCESS_CONTROL_ALLOW_HEADERS).unwrap();
    assert!(headers.contains("content-type"), "{headers}");
}

/// The browser's decision rests on `Access-Control-Allow-Origin` alone: without
/// it the preflight fails and the real request is never sent. `tower-http`
/// still lists the allowed methods and headers on every preflight — they are
/// the same for everyone and say nothing about who is allowed — so this
/// asserts the one header that grants access.
#[tokio::test]
async fn a_preflight_from_any_other_origin_is_not_allowed() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(&tmp).await;

    for origin in [
        "http://evil.example",
        "http://localhost:5174",
        "https://localhost:5173",
    ] {
        let response = app.clone().oneshot(preflight(origin)).await.unwrap();
        assert_eq!(
            header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN),
            None,
            "{origin}"
        );
    }
}

/// The live stream is read cross-origin too; its response carries the allow
/// header like any other.
#[tokio::test]
async fn the_event_stream_is_readable_from_an_allowed_origin() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(&tmp).await;

    let response = app
        .oneshot(
            Request::get("/api/subscribe?thread_id=00000000-0000-4000-8000-000000000000")
                .header(header::ORIGIN, ALLOWED)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_of(&response, header::CONTENT_TYPE).as_deref(),
        Some("text/event-stream")
    );
    assert_eq!(
        header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(ALLOWED)
    );
}

/// A browser matches its `Origin` header exactly, so an entry that could
/// never match is refused at startup instead of silently allowing nothing.
#[test]
fn an_allowed_origin_is_exactly_scheme_host_and_port() {
    for good in [
        "http://localhost:5173",
        "https://app.example.com",
        "http://[::1]:5173",
    ] {
        assert_eq!(allowed_origin(good), Ok(good.to_string()));
    }
    for bad in [
        "localhost:5173",
        "http://localhost:5173/",
        "http://localhost:5173/app",
        "ftp://localhost",
        "http://",
        "*",
    ] {
        assert_eq!(
            allowed_origin(bad),
            Err(ConfigError::OriginInvalid(bad.to_string())),
            "{bad}"
        );
    }
}
