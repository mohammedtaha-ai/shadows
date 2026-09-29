//! The HTTP surface, driven through the real router (spec §3.3, §1.0).
//!
//! `tests/resync.rs` owns what the stream guarantees and `tests/stream_frames.rs`
//! what its frames carry; this file owns that each
//! route reaches the capability it names, that path ids deserialize into their
//! newtypes, that failures become the status the transport mapping promises,
//! and that an open live stream does not hold the daemon up after it stops.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use shadows::cli::router;
use shadows_core::OperationId;
use shadows_core::RuntimeInstanceId;
use shadows_core::ThreadId;
use shadows_core::testing::LiveHandles;
use shadows_core::testing::Runtime;
use shadows_core::testing::Sessions;
use shadows_core::testing::Storage;
use shadows_core::{AppCore, CoreParts};
use shadows_http::AppState;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

use shadows_core::testing::acp;

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    app: Router,
    stopping: tokio::sync::watch::Sender<bool>,
    runtime: RuntimeInstanceId,
    sessions: Arc<Sessions>,
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let runtime_id = runtime.instance_id.clone();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let handles = Arc::new(LiveHandles::default());
    let sessions = acp::fake_sessions(&tmp.path().join("s.sqlite3")).await;
    let app = router(AppState {
        core: AppCore::assemble(CoreParts {
            storage: storage.clone(),
            runtime: Arc::new(runtime),
            sessions: sessions.clone(),
            handles: handles.clone(),
            bus,
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.to_string(),
        }),
        allowed_origins: Vec::new(),
        shutdown,
    });
    Fixture {
        _tmp: tmp,
        storage,
        app,
        stopping,
        runtime: runtime_id,
        sessions,
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
        Some(json!({
            "command_id": command_id, "slug": "demo", "name": name,
            "directory": std::env::temp_dir(),
        })),
    )
    .await
}

/// Spec §1: the daemon does not serve or embed a client. There is no page at
/// `/` to fall back on; a client is served from its own origin.
#[tokio::test]
async fn the_daemon_serves_no_page() {
    let f = fixture().await;
    let response = f
        .app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

    // Watch the thread live before the turn starts: the turn's entries must
    // reach an open stream as durable events, not only the entries route.
    let response = f
        .app
        .clone()
        .oneshot(
            Request::get(format!("/api/subscribe?thread_id={thread_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut stream = response.into_body().into_data_stream();
    let mut seen = String::new();
    read_until(&mut stream, &mut seen, |s| s.contains("event: caught-up")).await;

    // Spec §3.3: a long-running command answers 202 and an operation id.
    let (status, started) = call(
        &f.app,
        "POST",
        &format!("/api/threads/{thread_id}/turns"),
        Some(json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "hi", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })),
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

    // The same two entries, live, each exactly once and in order, as durable
    // events carrying their `seq`. The transient `entry` event is gone: it
    // would repeat the durable one with nothing to de-duplicate it by.
    read_until(&mut stream, &mut seen, |s| {
        s.contains("AgentMessage") && s.contains("event: turn-end")
    })
    .await;
    let appended: Vec<(i64, String)> = seen
        .split("\n\n")
        .filter(|frame| frame.lines().any(|l| l == "event: durable"))
        .filter_map(|frame| frame.lines().find_map(|l| l.strip_prefix("data: ")))
        .map(|data| serde_json::from_str::<Value>(data).unwrap())
        .filter(|ev| ev["kind"] == "ThreadEntryAppended")
        .map(|ev| {
            (
                ev["seq"].as_i64().unwrap(),
                ev["payload"]["kind"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let live_kinds: Vec<&str> = appended.iter().map(|(_, k)| k.as_str()).collect();
    assert_eq!(live_kinds, ["UserMessage", "AgentMessage"], "{seen}");
    assert!(appended[0].0 < appended[1].0, "{seen}");
    assert!(!seen.contains("event: entry"), "{seen}");
}

/// Reads the open stream until `done` holds for everything read so far.
async fn read_until(
    stream: &mut axum::body::BodyDataStream,
    seen: &mut String,
    done: impl Fn(&str) -> bool,
) {
    use tokio_stream::StreamExt;
    while !done(seen) {
        let chunk = tokio::time::timeout(Duration::from_secs(20), stream.next())
            .await
            .unwrap_or_else(|_| panic!("the stream stalled: {seen}"))
            .expect("the stream ended early")
            .unwrap();
        seen.push_str(std::str::from_utf8(&chunk).unwrap());
    }
}

/// A client reloaded mid-turn learns from this route whether a turn is running
/// on the thread and which one, so it can show Running and aim Stop: the
/// thread's operations, newest first, each with its status. Only that thread's.
#[tokio::test]
async fn a_threads_operations_are_listed_newest_first_with_their_status() {
    let f = fixture().await;
    let (_, project) = create_project(&f.app, "c1", "Demo").await;
    let threads_uri = format!("/api/projects/{}/threads", project["id"].as_str().unwrap());
    let mut threads = Vec::new();
    for (command_id, title) in [("c2", "T"), ("c3", "Other")] {
        let (_, thread) = call(
            &f.app,
            "POST",
            &threads_uri,
            Some(json!({ "command_id": command_id, "title": title })),
        )
        .await;
        threads.push(ThreadId::from_literal(thread["id"].as_str().unwrap()));
    }
    let (thread, other) = (&threads[0], &threads[1]);

    let first = f
        .storage
        .create_pending_operation(thread, &f.runtime)
        .await
        .unwrap();
    f.storage
        .mark_operation_started(&first, &f.runtime)
        .await
        .unwrap();
    f.storage
        .mark_operation_completed(&first, json!({}), &Default::default())
        .await
        .unwrap();
    f.storage
        .create_pending_operation(other, &f.runtime)
        .await
        .unwrap();
    let second = f
        .storage
        .create_pending_operation(thread, &f.runtime)
        .await
        .unwrap();
    f.storage
        .mark_operation_started(&second, &f.runtime)
        .await
        .unwrap();

    let (status, listed) = call(
        &f.app,
        "GET",
        &format!("/api/threads/{}/operations", thread.as_str()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let rows: Vec<(&str, &str)> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            (
                op["id"].as_str().unwrap(),
                op["status_kind"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [(second.as_str(), "Running"), (first.as_str(), "Completed")],
        "{listed}"
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

/// Spec §8.4 case 6 over HTTP. A Stop whose tree could not be terminated is a
/// failure of the daemon, and says so: 500 `PROCESS_TERMINATION_FAILED`, with
/// the operation left non-terminal — not a 200 carrying a `Running` row that a
/// client would read as "Stopping" forever.
#[tokio::test]
async fn a_stop_whose_termination_fails_answers_500_and_cancels_nothing() {
    let f = fixture().await;
    let (_, project) = create_project(&f.app, "c1", "Demo").await;
    let (_, thread) = call(
        &f.app,
        "POST",
        &format!("/api/projects/{}/threads", project["id"].as_str().unwrap()),
        Some(json!({ "command_id": "c2", "title": "T" })),
    )
    .await;
    let (status, started) = call(
        &f.app,
        "POST",
        &format!("/api/threads/{}/turns", thread["id"].as_str().unwrap()),
        Some(json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "ignore-cancel", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{started}");
    let op = OperationId::from_literal(started["operation_id"].as_str().unwrap());
    for _ in 0..200 {
        if f.storage.get_operation(&op).await.unwrap().status_kind == "Running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let thread_id = ThreadId::from_literal(thread["id"].as_str().unwrap());
    assert!(f.sessions.force_termination_failure(&thread_id).await);

    let (status, body) = call(
        &f.app,
        "POST",
        &format!("/api/operations/{}/stop", op.as_str()),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["code"], "PROCESS_TERMINATION_FAILED", "{body}");
    assert_eq!(
        f.storage.get_operation(&op).await.unwrap().status_kind,
        "Running"
    );
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

/// Spec §3.2: a refusal the schema makes is a 409 the client can act on, and
/// what reaches the client is written for it — not the driver's text, which
/// names tables and columns. A thread asked for under a project that does not
/// exist is missing, not in conflict.
#[tokio::test]
async fn a_refused_write_answers_its_own_code_without_database_text() {
    let f = fixture().await;
    let (_, first) = create_project(&f.app, "c1", "Demo").await;
    let (status, body) = create_project(&f.app, "c2", "Demo again").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "STORAGE_CONSTRAINT_VIOLATION");
    let message = body["message"].as_str().unwrap();
    assert!(
        !message.contains("UNIQUE") && !message.contains("project.slug"),
        "{message}"
    );

    let (status, body) = call(
        &f.app,
        "POST",
        "/api/projects/00000000-0000-4000-8000-000000000000/threads",
        Some(json!({ "command_id": "c3", "title": "T" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["code"], "INVALID_COMMAND");
    assert!(first["id"].is_string());
}
