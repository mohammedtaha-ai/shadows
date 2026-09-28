//! Full-stack tests for a Planner turn over the thread's ACP connection and
//! its cancellation interlock (spec §2.3, §8.4, §12.3). `fake_acp` (a
//! test-support binary) stands in for Node and the pinned adapter, so these
//! tests run with nothing installed; its prompts script what the agent does.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use shadows::cli::router;
use shadows_agent::claude::ClaudeAdapter;
use shadows_agent::events::HarnessEvent;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::events::Actor;
use shadows_core::operation::{Operation, OperationId};
use shadows_core::planner::{LiveHandles, PlannerTurn, Sessions, StopOutcome};
use shadows_core::runtime::Runtime;
use shadows_core::thread::{ThreadEntry, ThreadEntryKind, ThreadId};
use shadows_core::{AppCore, CoreParts};
use shadows_http::AppState;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;

type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

struct App {
    _tmp: tempfile::TempDir,
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
    bus: Bus,
    router: Router,
    thread: ThreadId,
    // Held: a dropped sender reads as a stopping daemon.
    _stopping: tokio::sync::watch::Sender<bool>,
}

async fn test_app() -> App {
    test_app_with(acp::fake_adapter()).await
}

async fn test_app_with_node(node: &str) -> App {
    test_app_with(acp::adapter_at(node.into())).await
}

async fn test_app_with(adapter: Arc<ClaudeAdapter>) -> App {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("s.sqlite3");
    let storage = Arc::new(shadows_core::storage::Storage::open(&db).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let runtime = Arc::new(runtime);

    let params = json!({ "slug": "demo" });
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &params),
    };
    let project = storage
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &shadows_core::project::ProjectDirectory::resolve(tmp.path()).unwrap(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let tctx = CommandContext {
        command_id: "c2".into(),
        command_kind: "thread.create".into(),
        request_fingerprint: fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage
        .create_planning_thread(&tctx, &project.id, "T", "claude-code")
        .await
        .unwrap()
        .id;

    let sessions = Sessions::new(
        adapter,
        shadows_core::storage::Storage::open(&db).await.unwrap(),
        acp::test_config(),
    );
    let handles = Arc::new(LiveHandles::default());
    let (bus, _) = tokio::sync::broadcast::channel(256);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let router = router(AppState {
        core: AppCore::assemble(CoreParts {
            storage,
            runtime: runtime.clone(),
            sessions: sessions.clone(),
            handles: handles.clone(),
            bus: bus.clone(),
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.to_string(),
        }),
        allowed_origins: Vec::new(),
        shutdown,
    });
    App {
        _tmp: tmp,
        runtime,
        handles,
        sessions,
        bus,
        router,
        thread,
        _stopping: stopping,
    }
}

async fn http_start_raw(app: &App, body: Value) -> (u16, Value) {
    let response = app
        .router
        .clone()
        .oneshot(
            Request::post(format!("/api/threads/{}/turns", app.thread.as_str()))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn start_prompt(app: &App, prompt: &str) -> OperationId {
    let (status, body) = http_start_raw(app, json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": prompt, "model": "fake-large", "mode": "acceptEdits", "effort": "high" })).await;
    assert_eq!(status, StatusCode::ACCEPTED.as_u16(), "{body}");
    OperationId::from_literal(body["operation_id"].as_str().unwrap())
}

async fn wait_terminal(app: &App, op: &OperationId) -> Operation {
    for _ in 0..200 {
        let loaded = app.runtime.storage.get_operation(op).await.unwrap();
        if loaded.finished_at.is_some() {
            return loaded;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach a terminal state in time");
}

async fn wait_running(app: &App, op: &OperationId) {
    for _ in 0..200 {
        if app
            .runtime
            .storage
            .get_operation(op)
            .await
            .unwrap()
            .status_kind
            == "Running"
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation never reached Running");
}

async fn entries(app: &App) -> Vec<ThreadEntry> {
    app.runtime
        .storage
        .list_thread_entries(&app.thread)
        .await
        .unwrap()
}

async fn stop(app: &App, op: &OperationId) -> StopOutcome {
    PlannerTurn::stop(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        op,
        Actor::user("local"),
    )
    .await
    .unwrap()
}

/// Waits until the running turn has streamed something, so a Stop lands
/// while the prompt is in flight.
async fn wait_for_delta(
    rx: &mut tokio::sync::broadcast::Receiver<(ThreadId, OperationId, HarnessEvent)>,
) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, _, event) = rx.recv().await.unwrap();
            if matches!(event, HarnessEvent::Chunk { .. }) {
                return;
            }
        }
    })
    .await
    .expect("the turn never streamed");
}

#[tokio::test]
async fn a_turn_streams_and_stores_one_entry_per_message() {
    let app = test_app().await;
    let op = start_prompt(&app, "two-messages").await;
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Completed");
    let bodies: Vec<_> = entries(&app).await.into_iter().map(|e| e.body).collect();
    assert_eq!(
        bodies,
        ["two-messages", "first", "[tool: Read notes.md]", "second"],
        "the tool entry carries its real title, not \"Terminal\""
    );
    assert!(
        !app.handles.contains(&op).await,
        "the registration outlived the turn"
    );
}

#[tokio::test]
async fn the_first_turn_records_the_session_and_the_next_resumes_it() {
    let app = test_app().await;
    wait_terminal(&app, &start_prompt(&app, "hi").await).await;
    let session = app
        .runtime
        .storage
        .turn_context(&app.thread)
        .await
        .unwrap()
        .harness_session_id
        .unwrap();
    app.sessions.terminate(&app.thread).await.unwrap();
    let op = start_prompt(&app, "report").await;
    wait_terminal(&app, &op).await;
    let r: Value = serde_json::from_str(&entries(&app).await.last().unwrap().body).unwrap();
    assert_eq!(
        (r["how"].as_str(), r["session"].as_str()),
        (Some("resume"), Some(session.as_str()))
    );
}

#[tokio::test]
async fn stop_is_confirmed_by_the_harness_when_it_answers_cancelled() {
    let app = test_app().await;
    let mut rx = app.bus.subscribe();
    let op = start_prompt(&app, "hang").await;
    wait_for_delta(&mut rx).await;
    assert_eq!(stop(&app, &op).await, StopOutcome::ResolvedByTurn);
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.status_kind, "Cancelled");
    assert!(done.cancel_requested_at.is_some());
    assert_eq!(
        app.sessions.live_count().await,
        1,
        "the adapter survives a confirmed cancel"
    );
}

#[tokio::test]
async fn stop_terminates_the_adapter_when_the_harness_does_not_confirm() {
    let app = test_app().await;
    let mut rx = app.bus.subscribe();
    let op = start_prompt(&app, "ignore-cancel").await;
    wait_for_delta(&mut rx).await;
    assert_eq!(stop(&app, &op).await, StopOutcome::Cancelled);
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
    assert_eq!(app.sessions.live_count().await, 0);
    let next = start_prompt(&app, "hi").await; // a fresh adapter
    assert_eq!(wait_terminal(&app, &next).await.status_kind, "Completed");
}

#[tokio::test]
async fn an_adapter_that_exits_mid_turn_fails_the_turn() {
    let app = test_app().await;
    let op = start_prompt(&app, "exit").await;
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.status_kind, "Failed");
    assert!(
        done.failure_reason
            .unwrap()
            .contains("the harness exited during the turn")
    );
    assert_eq!(
        app.sessions.live_count().await,
        0,
        "the dead adapter was kept"
    );
}

#[tokio::test]
async fn a_refused_stop_reason_fails_the_turn_naming_it() {
    let app = test_app().await;
    let done = wait_terminal(&app, &start_prompt(&app, "refuse").await).await;
    assert_eq!(done.status_kind, "Failed");
    assert!(done.failure_reason.unwrap().contains("max_tokens"));
}

#[tokio::test]
async fn a_permission_request_is_refused_and_recorded() {
    let app = test_app().await;
    wait_terminal(&app, &start_prompt(&app, "ask-permission").await).await;
    let refused: Vec<_> = entries(&app)
        .await
        .into_iter()
        .filter(|e| e.kind == ThreadEntryKind::PermissionRefused)
        .collect();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].body, "Run echo probe");
}

#[tokio::test]
async fn a_turn_whose_adapter_cannot_start_writes_nothing() {
    let app = test_app_with_node("C:/definitely/missing/node.exe").await;
    let (status, body) = http_start_raw(&app, json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "hi", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (502, Some("HARNESS_START_FAILED"))
    );
    assert!(entries(&app).await.is_empty());
    let operations = app
        .runtime
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert!(operations.is_empty(), "{operations:?}");
}

/// Spec §8.4 case 6. `stop` on an operation this runtime holds no live
/// registration for can only record the request — it must not invent a
/// confirmation it does not have, and the operation is left non-terminal.
#[tokio::test]
async fn stop_without_a_live_turn_records_the_request_and_stays_non_terminal() {
    let app = test_app().await;
    let op = app
        .runtime
        .storage
        .create_pending_operation(&app.thread, &app.runtime.instance_id)
        .await
        .unwrap();
    app.runtime
        .storage
        .mark_operation_started(&op, &app.runtime.instance_id)
        .await
        .unwrap();

    assert_eq!(stop(&app, &op).await, StopOutcome::NotLive);

    let loaded = app.runtime.storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running");
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
}

/// Spec §8.4 case 6 with a live turn whose adapter cannot be terminated. Stop
/// must not take an outcome it cannot confirm: the registration stays, so the
/// still-running tree can be reached again, and nothing terminal is written.
#[tokio::test]
async fn unconfirmed_termination_keeps_the_registration_and_leaves_the_operation_non_terminal() {
    let app = test_app().await;
    let op = start_prompt(&app, "ignore-cancel").await;
    wait_running(&app, &op).await;
    assert!(
        app.sessions.force_termination_failure(&app.thread).await,
        "the adapter must be live for this test to mean anything"
    );

    assert_eq!(stop(&app, &op).await, StopOutcome::TerminationFailed);

    let loaded = app.runtime.storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running");
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
    assert!(app.handles.contains(&op).await);
}
