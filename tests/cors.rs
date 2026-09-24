//! Cross-origin access (spec §1): every client is on another origin, so the
//! daemon answers a browser only for origins it was configured with. Driven
//! through the real router; `tests/serve_smoke.rs` checks the binary's default
//! list reaches it.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use shadows::config::{ConfigError, allowed_origin};
use shadows::planner::LiveHandles;
use shadows::protocol::{AppState, router};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;

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
        sessions: acp::fake_sessions(&tmp.path().join("s.sqlite3")).await,
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

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// DNS rebinding: a page whose own name was re-pointed at this machine is
/// same-origin with the daemon, so CORS never applies to it. Its requests
/// still carry its own name in `Host`, and that is refused before any route;
/// this machine's names and IP addresses are not.
#[tokio::test]
async fn a_request_addressed_to_another_hostname_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(&tmp).await;

    for host in [
        "evil.example:4318",
        "evil.example",
        "127.0.0.1.evil.example",
    ] {
        let (status, body) = send(
            &app,
            Request::get("/api/projects")
                .header(header::HOST, host)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{host}: {body}");
        assert_eq!(body["code"], "ORIGIN_REFUSED", "{host}");
    }
    for host in [
        "127.0.0.1:4318",
        "localhost:4318",
        "LOCALHOST",
        "[::1]:4318",
        "192.168.1.20:4318",
    ] {
        let (status, body) = send(
            &app,
            Request::get("/api/projects")
                .header(header::HOST, host)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{host}: {body}");
    }
}

/// CORS stops a page reading an answer, not the request running. A simple
/// request from any page — an `<img>` pointed at the disk route, a form
/// posting to it — would otherwise run with the page none the wiser. Each is
/// refused before its handler runs: nothing is listed, nothing created.
#[tokio::test]
async fn a_page_on_another_site_cannot_make_the_daemon_act() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(&tmp).await;
    let parent = tmp.path().to_str().unwrap().to_string();
    let listing = format!("/api/fs/dirs?path={}", encode(&parent));
    let create = |origin: Option<&str>| {
        let request =
            Request::post("/api/fs/dirs").header(header::CONTENT_TYPE, "application/json");
        let request = match origin {
            Some(origin) => request.header(header::ORIGIN, origin),
            None => request,
        };
        request
            .body(Body::from(
                serde_json::json!({ "parent": parent, "name": "made" }).to_string(),
            ))
            .unwrap()
    };

    // No-cors, from another site: the browser sends no Origin, but says where
    // the request came from.
    let (status, body) = send(
        &app,
        Request::get(&listing)
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "ORIGIN_REFUSED");

    let (status, body) = send(&app, create(Some("http://evil.example"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(!tmp.path().join("made").exists(), "a refused request ran");

    // The configured client, and a caller that is not a browser at all.
    let (status, body) = send(
        &app,
        Request::get(&listing)
            .header(header::ORIGIN, ALLOWED)
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(&app, Request::get(&listing).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(&app, create(Some(ALLOWED))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(tmp.path().join("made").is_dir());
}

/// A query-string value, percent-encoded byte by byte.
fn encode(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
