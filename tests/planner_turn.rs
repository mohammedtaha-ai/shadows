//! Full-stack tests for the two-phase Planner turn and its cancellation
//! interlock (spec §2.3, §2.7, §8.3, §8.4). `fake_claude` (a test-support
//! binary, not the real `claude` CLI) stands in for the harness executable so
//! these tests run without any external tool installed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use shadows::agent::claude::ClaudeHarness;
use shadows::command::{CommandContext, fingerprint};
use shadows::operation::{Operation, OperationId};
use shadows::planner::{LiveHandles, PlannerTurn, PlannerTurnRequest};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::ThreadId;

async fn fixture() -> (tempfile::TempDir, Arc<Runtime>, ThreadId) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage).await.unwrap();
    let runtime = Arc::new(runtime);

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
        .create_project(&ctx, "demo", "Demo")
        .await
        .unwrap();
    let tctx = CommandContext {
        command_id: "c2".into(),
        command_kind: "thread.create".into(),
        request_fingerprint: fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = runtime
        .storage
        .create_planning_thread(&tctx, &project.id, "T")
        .await
        .unwrap();
    (tmp, runtime, thread.id)
}

fn harness() -> Arc<ClaudeHarness> {
    Arc::new(ClaudeHarness::new(
        PathBuf::from(env!("CARGO_BIN_EXE_fake_claude")),
        "fake-1".into(),
    ))
}

/// Polls `get_operation` until it leaves Pending/Running, bounded so a stuck
/// interlock fails the test instead of hanging the suite.
async fn wait_for_terminal(runtime: &Runtime, op: &OperationId) -> Operation {
    for _ in 0..100 {
        let loaded = runtime.storage.get_operation(op).await.unwrap();
        if loaded.status_kind != "Pending" && loaded.status_kind != "Running" {
            return loaded;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach a terminal state in time");
}

async fn wait_for_running(runtime: &Runtime, op: &OperationId) {
    for _ in 0..100 {
        let loaded = runtime.storage.get_operation(op).await.unwrap();
        if loaded.status_kind == "Running" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach Running in time");
}

/// Spec §2.7, §8.3. A turn that runs to completion on its own persists
/// Pending -> Running -> Completed, writes the durable entry the fake harness
/// streamed, and releases its live handle once the process has exited.
#[tokio::test]
async fn a_completed_turn_persists_the_stream_and_releases_its_handle() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());
    let (bus, _rx) = tokio::sync::broadcast::channel(16);

    let op = PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        harness(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: "quick".into(),
            cwd: std::env::temp_dir(),
            resume_session_id: None,
        },
        bus,
    )
    .await
    .unwrap();

    let loaded = wait_for_terminal(&runtime, &op).await;
    assert_eq!(loaded.status_kind, "Completed");
    assert!(loaded.finished_at.is_some());

    let entries = runtime.storage.list_thread_entries(&thread).await.unwrap();
    assert_eq!(
        entries.len(),
        1,
        "the fake harness's one durable line must be recorded"
    );
    assert_eq!(entries[0].body, "hello from fake_claude");

    assert!(
        !handles.contains(&op).await,
        "a finished operation's handle must not linger in LiveHandles"
    );
}

/// Spec §2.3, §8.3. Cancelling a live turn requests, then terminates, then
/// confirms, then writes Cancelled — never only the request.
#[tokio::test]
async fn cancelling_a_running_turn_confirms_termination_before_writing_cancelled() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());
    let (bus, _rx) = tokio::sync::broadcast::channel(16);

    let op = PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        harness(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: "hang".into(),
            cwd: std::env::temp_dir(),
            resume_session_id: None,
        },
        bus,
    )
    .await
    .unwrap();

    wait_for_running(&runtime, &op).await;

    PlannerTurn::stop(runtime.clone(), handles.clone(), &op)
        .await
        .unwrap();

    let loaded = runtime.storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Cancelled");
    assert!(loaded.finished_at.is_some());
    assert!(loaded.cancel_requested_at.is_some());
    assert!(
        !handles.contains(&op).await,
        "the terminated operation's handle must be gone from LiveHandles"
    );
}

/// Spec §8.4 case 6. `stop` on an operation this runtime does not hold a live
/// handle for (never started through `PlannerTurn::start`, so containment was
/// never registered) can only record the request — it must not invent
/// confirmation it does not have, and the operation is left non-terminal.
#[tokio::test]
async fn stop_without_a_live_handle_records_the_request_and_stays_non_terminal() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());

    let op = runtime
        .storage
        .create_pending_operation(&thread, &runtime.instance_id)
        .await
        .unwrap();
    runtime
        .storage
        .mark_operation_started(&op, &runtime.instance_id)
        .await
        .unwrap();

    PlannerTurn::stop(runtime.clone(), handles, &op)
        .await
        .unwrap();

    let loaded = runtime.storage.get_operation(&op).await.unwrap();
    assert_eq!(
        loaded.status_kind, "Running",
        "no live handle means termination cannot be confirmed"
    );
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
}
