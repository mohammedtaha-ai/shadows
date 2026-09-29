//! `shadows serve --debug`'s log file (spec §8.7): created under the data
//! directory, holding what a run did once its guard is dropped, and never
//! holding the prompt.
//!
//! One test in its own binary on purpose: the tracing subscriber is global to
//! a process and can be installed once, so a second test here would race the
//! first for it. It runs one real turn through `fake_acp`, one stopped
//! turn, and one HTTP request, because a log file that exists but says nothing
//! about a turn is the failure this file guards against.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use shadows::cli::router;
use shadows_core::Actor;
use shadows_core::OperationId;
use shadows_core::testing::Bus;
use shadows_core::testing::Runtime;
use shadows_core::testing::Sessions;
use shadows_core::testing::Storage;
use shadows_core::testing::{CommandContext, fingerprint};
use shadows_core::testing::{LiveHandles, PlannerTurn};
use shadows_core::{AppCore, CoreParts};
use shadows_http::AppState;
use tower::ServiceExt;

use shadows_core::testing::acp;
use shadows_core::testing::turn;

const PROMPT: &str = "a prompt that must never reach a log 7f3a";

#[tokio::test]
async fn debug_mode_writes_a_run_to_a_file_under_the_data_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let log = shadows::tracing::init(false, Some(tmp.path()))
        .unwrap()
        .expect("debug mode returns the log it opened");
    let path = log.path().to_path_buf();
    assert_eq!(path.parent(), Some(tmp.path().join("logs").as_path()));
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("shadows-") && name.ends_with(".log"),
        "{name}"
    );

    let (daemon, _stopping) = app_state(&tmp).await;
    let thread = seed_thread(&daemon.runtime).await;

    // Through the router, as a client starts one: the turn outlives the
    // request that started it, and its lines must not claim otherwise.
    let response = router(daemon.state.clone())
        .oneshot(
            Request::post(format!("/api/threads/{thread}/turns"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": PROMPT, "model": "fake-large", "mode": "acceptEdits", "effort": "high" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let started: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let completed = OperationId::from_literal(started["operation_id"].as_str().unwrap());
    wait_for_terminal(&daemon.runtime, &completed).await;

    // `ignore-cancel`: the harness never confirms, so Stop terminates the
    // adapter and its log shows the tree reaped.
    let stopped = start(&daemon, &thread, "ignore-cancel").await;
    PlannerTurn::stop(
        daemon.runtime.clone(),
        daemon.handles.clone(),
        daemon.sessions.clone(),
        &stopped,
        Actor::user("local"),
    )
    .await
    .unwrap();

    let response = router(daemon.state.clone())
        .oneshot(Request::get("/api/projects").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(response.status().is_success());

    // A debug-level line, which only debug mode lets through.
    tracing::debug!(target: "shadows::debug_log_test", "debug-level-probe");

    // Dropping the guard flushes the background writer; before this the last
    // lines may still be queued.
    drop(log);
    let text = std::fs::read_to_string(&path).unwrap();
    println!("{text}");

    for needle in [
        "debug-level-probe",
        "agent.invocation.start",
        "process.spawn",
        "planner.first_output",
        "planner.turn_end",
        "process.exit",
        "planner.stop",
        "process.terminate",
        "tree reaped",
        "http.response",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
    for (op, to) in [(&completed, "Completed"), (&stopped, "Cancelled")] {
        let op = op.as_str();
        assert!(
            text.lines()
                .any(|l| l.contains("operation.transition.committed")
                    && l.contains(op)
                    && l.contains(&format!("to={to}"))),
            "no committed transition to {to} for {op} in:\n{text}"
        );
        assert!(
            text.lines()
                .any(|l| l.contains("agent.invocation.start") && l.contains(op)),
            "agent.invocation.start does not name its operation {op}:\n{text}"
        );
    }
    // The adapter is spawned when the thread's session opens, before any
    // operation exists, so its line names the thread.
    assert!(
        text.lines()
            .any(|l| l.contains("process.spawn") && l.contains(thread.as_str())),
        "process.spawn does not name its thread:\n{text}"
    );
    // The HTTP-started turn's lines are under `planner.turn` alone. Nested
    // under the request's `http{...}` span, every one of them would name a
    // POST that returned 202 long before the line was written.
    let turn_lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("planner.turn{") && l.contains(completed.as_str()))
        .collect();
    assert!(
        turn_lines
            .iter()
            .any(|l| l.contains("planner.first_output")),
        "no planner line for the HTTP-started turn in:\n{text}"
    );
    for line in &turn_lines {
        assert!(
            !line.contains("http{"),
            "a turn line inherited the request span: {line}"
        );
    }
    assert!(
        !text.contains(PROMPT),
        "the prompt reached the log:\n{text}"
    );
    assert!(
        !text.contains('\u{1b}'),
        "the file is plain text, with no terminal colour codes"
    );
}

async fn start(state: &Daemon, thread: &shadows_core::ThreadId, prompt: &str) -> OperationId {
    turn::start_direct(
        &state.runtime,
        &state.handles,
        &state.sessions,
        &state.bus,
        thread,
        prompt,
    )
    .await
    .unwrap()
}

/// The daemon's state, and the handles on its `Arc`s the test drives turns
/// through directly.
struct Daemon {
    state: AppState,
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
    bus: Bus,
}

async fn app_state(tmp: &tempfile::TempDir) -> (Daemon, tokio::sync::watch::Sender<bool>) {
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let runtime = Arc::new(runtime);
    let handles = Arc::new(LiveHandles::default());
    let sessions = acp::fake_sessions(storage.clone());
    let (bus, _) = tokio::sync::broadcast::channel(64);
    // The sender is returned and held by the test: dropped, it would read as
    // a stopping daemon.
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let state = AppState {
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
    };
    let daemon = Daemon {
        state,
        runtime,
        handles,
        sessions,
        bus,
    };
    (daemon, stopping)
}

async fn seed_thread(runtime: &Runtime) -> shadows_core::ThreadId {
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &params),
    };
    let project = runtime
        .storage
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &shadows_core::testing::ProjectDirectory::resolve(&std::env::temp_dir()).unwrap(),
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
    runtime
        .storage
        .create_planning_thread(&tctx, &project.id, "T", "claude-code")
        .await
        .unwrap()
        .id
}

async fn wait_for_terminal(runtime: &Runtime, op: &OperationId) {
    for _ in 0..100 {
        let status = runtime.storage.get_operation(op).await.unwrap().status_kind;
        if status != "Pending" && status != "Running" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach a terminal state in time");
}
