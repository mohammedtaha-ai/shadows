//! Spec §8.4 case 7: a client disconnecting never cancels or strands work.
//!
//! Hyper drops a handler's future when its connection closes. Here each
//! request's future is dropped after its first poll through the real router,
//! which is that, and the work the request began must still reach its end.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use serde_json::{Value, json};
use shadows::agent::claude::ClaudeHarness;
use shadows::planner::LiveHandles;
use shadows::protocol::{AppState, router};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::ThreadId;
use tower::ServiceExt;

/// The turn still starts and runs, and the stop still terminates it and
/// records `Cancelled` — nothing is left `Pending`, or `Running` over a dead
/// tree, because the client that asked went away mid-request.
#[tokio::test]
async fn a_client_that_disconnects_mid_request_strands_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (_stopping, shutdown) = tokio::sync::watch::channel(false);
    let handles = Arc::new(LiveHandles::default());
    let app = router(AppState {
        runtime: Arc::new(runtime),
        storage: storage.clone(),
        handles: handles.clone(),
        harness: Arc::new(ClaudeHarness::new(
            PathBuf::from(env!("CARGO_BIN_EXE_fake_claude")),
            "fake-1".into(),
        )),
        bus,
        allowed_origins: Vec::new(),
        shutdown,
    });

    let project = post(
        &app,
        "/api/projects",
        json!({
            "command_id": "c1", "slug": "demo", "name": "Demo",
            "directory": std::env::temp_dir(),
        }),
    )
    .await;
    let thread = post(
        &app,
        &format!("/api/projects/{}/threads", project["id"].as_str().unwrap()),
        json!({ "command_id": "c2", "title": "T" }),
    )
    .await;
    let thread = ThreadId::from_literal(thread["id"].as_str().unwrap());

    let start = Request::post(format!("/api/threads/{}/turns", thread.as_str()))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "prompt": "hang" }).to_string()))
        .unwrap();
    let dropped = tokio::time::timeout(Duration::ZERO, app.clone().oneshot(start)).await;
    assert!(
        dropped.is_err(),
        "the request ended before it could be dropped"
    );

    let op = wait_for(|| async {
        let ops = storage.list_operations_for_thread(&thread).await.unwrap();
        ops.into_iter()
            .find(|op| op.status_kind == "Running")
            .map(|op| op.id)
    })
    .await
    .expect("the turn never ran after its request was dropped");

    let stop = Request::post(format!("/api/operations/{}/stop", op.as_str()))
        .body(Body::empty())
        .unwrap();
    let dropped = tokio::time::timeout(Duration::ZERO, app.clone().oneshot(stop)).await;
    assert!(
        dropped.is_err(),
        "the request ended before it could be dropped"
    );

    let status = wait_for(|| async {
        let op = storage.get_operation(&op).await.unwrap();
        op.finished_at.is_some().then_some(op.status_kind)
    })
    .await
    .expect("the stop never finished after its request was dropped");
    assert_eq!(status, "Cancelled");
    assert!(!handles.contains(&op).await);
}

async fn post(app: &Router, uri: &str, body: Value) -> Value {
    let request = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Polls `probe` until it answers, for up to twenty seconds.
async fn wait_for<T, F: std::future::Future<Output = Option<T>>>(
    probe: impl Fn() -> F,
) -> Option<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while tokio::time::Instant::now() < deadline {
        if let Some(found) = probe().await {
            return Some(found);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}
