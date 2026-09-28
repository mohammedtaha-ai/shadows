//! The shape of the frames `/api/subscribe` sends, driven through the real
//! router.
//!
//! `tests/resync.rs` owns what the stream guarantees about order and gaps; this
//! file owns what each frame carries, which is the contract the Web client's
//! `web/src/stream/frames.ts` parses. A client that reloads mid-turn learns
//! which operation is running from the operations route, then follows it by the
//! `operation_id` its durable frames name — so a frame that drops that field
//! strands the client with a Stop button it cannot aim.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use serde_json::Value;
use shadows::cli::router;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::planner::LiveHandles;
use shadows_core::runtime::Runtime;
use shadows_core::storage::Storage;
use shadows_http::AppState;
use tokio_stream::StreamExt;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;

/// A durable frame names its operation and its thread, and its `payload` is
/// the event's JSON object, not a string holding JSON. `caught-up` is JSON
/// like every other frame.
#[tokio::test]
async fn durable_frames_name_their_operation_and_thread_and_caught_up_is_json() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let runtime_id = runtime.instance_id.clone();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (_stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        runtime: Arc::new(runtime),
        storage: storage.clone(),
        handles: Arc::new(LiveHandles::default()),
        sessions: acp::fake_sessions(&tmp.path().join("s.sqlite3")).await,
        bus,
        allowed_origins: Vec::new(),
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
        shutdown,
    });

    let params = serde_json::json!({});
    let ctx = |id: &str, kind: &str| CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    };
    let dir = shadows_core::project::ProjectDirectory::resolve(&std::env::temp_dir()).unwrap();
    let project = storage
        .create_project(
            &ctx("c1", "project.create"),
            "demo",
            "Demo",
            &dir,
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(&ctx("c2", "thread.create"), &project.id, "T", "claude-code")
        .await
        .unwrap();
    let op = storage
        .create_pending_operation(&thread.id, &runtime_id)
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::get(format!("/api/subscribe?thread_id={}", thread.id.as_str()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut body = response.into_body().into_data_stream();
    let mut text = String::new();
    while !text.contains("event: caught-up") {
        let chunk = tokio::time::timeout(Duration::from_secs(10), body.next())
            .await
            .unwrap_or_else(|_| panic!("stalled before caught-up: {text}"))
            .expect("the stream ended early")
            .unwrap();
        text.push_str(std::str::from_utf8(&chunk).unwrap());
    }

    let frames = frames(&text);
    let durable: Vec<&Value> = frames
        .iter()
        .filter(|(event, _)| event == "durable")
        .map(|(_, data)| data)
        .collect();

    let created = durable
        .iter()
        .find(|f| f["kind"] == "OperationCreated")
        .unwrap_or_else(|| panic!("the operation's creation replays: {text}"));
    assert_eq!(created["operation_id"], op.as_str(), "{created}");
    assert_eq!(created["thread_id"], thread.id.as_str(), "{created}");
    assert_eq!(
        created["payload"]["kind"], "PlannerTurn",
        "the payload is an object, not a string: {created}"
    );

    let thread_created = durable
        .iter()
        .find(|f| f["kind"] == "PlanningThreadCreated")
        .unwrap_or_else(|| panic!("the thread's creation replays: {text}"));
    assert!(
        thread_created
            .as_object()
            .unwrap()
            .get("operation_id")
            .is_some_and(Value::is_null),
        "an event with no operation says so with null: {thread_created}"
    );

    let (_, caught_up) = frames
        .iter()
        .find(|(event, _)| event == "caught-up")
        .unwrap();
    let last_seq = durable.last().unwrap()["seq"].as_i64().unwrap();
    assert_eq!(caught_up["seq"], last_seq, "{text}");
}

/// Every frame as `(event, data parsed as JSON)`. A frame whose data is not
/// JSON fails the test: every frame this file reads is promised to be JSON.
fn frames(text: &str) -> Vec<(String, Value)> {
    text.split("\n\n")
        .filter_map(|frame| {
            let event = frame.lines().find_map(|l| l.strip_prefix("event: "))?;
            let data = frame.lines().find_map(|l| l.strip_prefix("data: "))?;
            let parsed = serde_json::from_str(data)
                .unwrap_or_else(|e| panic!("`{event}` data is not JSON ({e}): {data}"));
            Some((event.to_string(), parsed))
        })
        .collect()
}
