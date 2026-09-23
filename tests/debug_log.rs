//! `shadows serve --debug`'s log file (spec §8.7): created under the data
//! directory, holding what a run did once its guard is dropped, and never
//! holding the prompt.
//!
//! One test in its own binary on purpose: the tracing subscriber is global to
//! a process and can be installed once, so a second test here would race the
//! first for it. It runs one real turn through `fake_claude`, one stopped
//! turn, and one HTTP request, because a log file that exists but says nothing
//! about a turn is the failure this file guards against.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use shadows::agent::claude::ClaudeHarness;
use shadows::command::{CommandContext, fingerprint};
use shadows::events::Actor;
use shadows::operation::OperationId;
use shadows::planner::{LiveHandles, PlannerTurn, PlannerTurnRequest};
use shadows::protocol::{AppState, router};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use tower::ServiceExt;

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

    let (state, _stopping) = app_state(&tmp).await;
    let thread = seed_thread(&state.runtime).await;

    // Through the router, as a client starts one: the turn outlives the
    // request that started it, and its lines must not claim otherwise.
    let response = router(state.clone())
        .oneshot(
            Request::post(format!("/api/threads/{thread}/turns"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "prompt": PROMPT }).to_string(),
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
    wait_for_terminal(&state.runtime, &completed).await;

    let stopped = start(&state, &thread, "hang").await;
    PlannerTurn::stop(
        state.runtime.clone(),
        state.handles.clone(),
        &stopped,
        Actor::user("local"),
    )
    .await
    .unwrap();

    let response = router(state.clone())
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
                .any(|l| l.contains("process.spawn") && l.contains(op)),
            "process.spawn does not name its operation {op}:\n{text}"
        );
    }
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

async fn start(state: &AppState, thread: &shadows::thread::ThreadId, prompt: &str) -> OperationId {
    PlannerTurn::start(
        state.runtime.clone(),
        state.handles.clone(),
        state.harness.clone(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: prompt.into(),
        },
        state.bus.clone(),
    )
    .await
    .unwrap()
}

async fn app_state(tmp: &tempfile::TempDir) -> (AppState, tokio::sync::watch::Sender<bool>) {
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    // The sender is returned and held by the test: dropped, it would read as
    // a stopping daemon.
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let state = AppState {
        runtime: Arc::new(runtime),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(
            PathBuf::from(env!("CARGO_BIN_EXE_fake_claude")),
            "fake-1".into(),
        )),
        bus,
        allowed_origins: Vec::new(),
        shutdown,
    };
    (state, stopping)
}

async fn seed_thread(runtime: &Runtime) -> shadows::thread::ThreadId {
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
            &shadows::project::ProjectDirectory::resolve(&std::env::temp_dir()).unwrap(),
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
        .create_planning_thread(&tctx, &project.id, "T")
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
