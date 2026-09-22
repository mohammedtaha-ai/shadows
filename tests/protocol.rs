//! The HTTP surface, driven through the real router (spec §3.3, §1.0).
//!
//! `tests/resync.rs` owns what the stream guarantees; this file owns that each
//! route reaches the capability it names, that path ids deserialize into their
//! newtypes, that failures become the status the transport mapping promises,
//! and that an open live stream does not hold the daemon up after it stops.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use shadows::agent::claude::ClaudeHarness;
use shadows::operation::OperationId;
use shadows::planner::LiveHandles;
use shadows::protocol::{AppState, router};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    app: Router,
    stopping: tokio::sync::watch::Sender<bool>,
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        runtime: Arc::new(runtime),
        storage: storage.clone(),
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(
            PathBuf::from(env!("CARGO_BIN_EXE_fake_claude")),
            "fake-1".into(),
        )),
        bus,
        project_root: tmp.path().to_path_buf(),
        shutdown,
    });
    Fixture {
        _tmp: tmp,
        storage,
        app,
        stopping,
    }
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(b) => request
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn create_project(app: &Router, command_id: &str, name: &str) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/api/projects",
        Some(json!({ "command_id": command_id, "slug": "demo", "name": name })),
    )
    .await
}

#[tokio::test]
async fn the_index_serves_the_web_client() {
    let f = fixture().await;
    let response = f
        .app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers()["content-type"].to_str().unwrap();
    assert!(content_type.starts_with("text/html"), "{content_type}");
}

/// Spec §3.2: the command id is the idempotency key. A replay with the same
/// body returns the first result; the same id with a different body is a
/// `CommandConflict`, which the transport maps to 409.
#[tokio::test]
async fn project_routes_create_list_replay_and_conflict() {
    let f = fixture().await;

    let (status, created) = create_project(&f.app, "c1", "Demo").await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = created["id"].as_str().expect("a project id").to_string();

    let (status, replayed) = create_project(&f.app, "c1", "Demo").await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(
        replayed["id"], created["id"],
        "a replay returns the first result"
    );

    let (status, conflict) = create_project(&f.app, "c1", "Other").await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");
    assert_eq!(conflict["code"], "COMMAND_CONFLICT");

    let (status, listed) = call(&f.app, "GET", "/api/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [id.as_str()]);
}

/// The id-bearing routes: a path segment must deserialize into its newtype,
/// or every one of them would answer 400.
#[tokio::test]
async fn thread_routes_create_list_and_run_a_turn_to_its_entries() {
    let f = fixture().await;
    let (_, project) = create_project(&f.app, "c1", "Demo").await;
    let project_id = project["id"].as_str().unwrap();

    let threads_uri = format!("/api/projects/{project_id}/threads");
    let (status, thread) = call(
        &f.app,
        "POST",
        &threads_uri,
        Some(json!({ "command_id": "c2", "title": "T" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{thread}");
    let thread_id = thread["id"].as_str().unwrap().to_string();

    let (status, listed) = call(&f.app, "GET", &threads_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], thread["id"]);

    // Spec §3.3: a long-running command answers 202 and an operation id.
    let (status, started) = call(
        &f.app,
        "POST",
        &format!("/api/threads/{thread_id}/turns"),
        Some(json!({ "prompt": "hi" })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{started}");
    let op_id = started["operation_id"].as_str().expect("an operation id");

    // The turn ends on its own; wait for the operation row to say so.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let op = f
            .storage
            .get_operation(&OperationId::from_literal(op_id))
            .await
            .unwrap();
        if op.finished_at.is_some() {
            assert_eq!(op.status_kind, "Completed", "{op:?}");
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "turn never ended");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let (status, entries) = call(
        &f.app,
        "GET",
        &format!("/api/threads/{thread_id}/entries"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let kinds: Vec<&str> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["UserMessage", "AgentMessage"],
        "the prompt is recorded before the agent's reply: {entries}"
    );
}

/// Stopping an operation that does not exist reaches `get_operation`'s
/// `NotFound`, which the transport maps to 404 rather than 500.
#[tokio::test]
async fn stopping_an_unknown_operation_is_not_found() {
    let f = fixture().await;
    let (status, body) = call(
        &f.app,
        "POST",
        "/api/operations/00000000-0000-4000-8000-000000000000/stop",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

/// Spec §8.5 says shutdown has no drain mode. A graceful HTTP shutdown still
/// waits for every open response, and a live stream never finishes on its
/// own — so a daemon with one browser tab open would never exit. The stream
/// must end when the daemon says it is stopping.
#[tokio::test]
async fn an_open_live_stream_does_not_hold_up_shutdown() {
    let f = fixture().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (signal, signalled) = tokio::sync::oneshot::channel::<()>();
    let stopping = f.stopping;
    let server = tokio::spawn(async move {
        axum::serve(listener, f.app)
            .with_graceful_shutdown(async move {
                let _ = signalled.await;
                stopping.send_replace(true);
            })
            .await
            .unwrap();
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(
            b"GET /api/subscribe?thread_id=00000000-0000-4000-8000-000000000000 HTTP/1.1\r\n\
              Host: localhost\r\n\r\n",
        )
        .await
        .unwrap();
    let mut buf = vec![0u8; 4096];
    let mut text = String::new();
    while !text.contains("caught-up") {
        let n = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf))
            .await
            .expect("the stream never reached its live phase")
            .unwrap();
        assert!(n > 0, "the connection closed before the live phase: {text}");
        text.push_str(&String::from_utf8_lossy(&buf[..n]));
    }

    signal.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("serve never returned: an open live stream held up shutdown")
        .unwrap();
}
