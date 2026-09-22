//! Full-stack tests for the two-phase Planner turn and its cancellation
//! interlock (spec §2.3, §2.7, §8.3, §8.4). `fake_claude` (a test-support
//! binary, not the real `claude` CLI) stands in for the harness executable so
//! these tests run without any external tool installed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use shadows::agent::StreamItem;
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

    // Without this the whole TurnEnd branch could be deleted and the suite
    // would still pass: the default outcome also yields Completed.
    let outcome: serde_json::Value =
        serde_json::from_str(loaded.outcome_json.as_deref().expect("an outcome")).unwrap();
    assert_eq!(
        outcome,
        serde_json::json!({ "subtype": "success", "stop_reason": "end_turn" }),
        "the persisted outcome must be the one the harness's turn-end reported"
    );

    let entries = runtime.storage.list_thread_entries(&thread).await.unwrap();
    assert_eq!(
        entries.len(),
        1,
        "the fake harness's one durable line must be recorded"
    );
    assert_eq!(entries[0].body, "hello from fake_claude");
    // Spec §4.2. The author is the agent whose turn this is, the same actor on
    // every line — not the harness's per-line uuid, which would make every
    // message look like a different author.
    assert_eq!(entries[0].author.kind, "Agent");
    assert_eq!(entries[0].author.id, "Planner");

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

/// Spec §8.4 case 6, the branch the no-handle test cannot reach: a live
/// registration whose termination fails. `stop` must not take ownership of an
/// outcome it cannot confirm — if it did, the registration would be gone, the
/// reader would find nothing to claim, and the still-running tree could never
/// be terminated again by this runtime.
#[tokio::test]
async fn unconfirmed_termination_keeps_the_handle_and_leaves_the_operation_non_terminal() {
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
    assert!(
        handles.force_termination_failure(&op).await,
        "the turn must still be registered for this test to mean anything"
    );

    PlannerTurn::stop(runtime.clone(), handles.clone(), &op)
        .await
        .unwrap();

    let loaded = runtime.storage.get_operation(&op).await.unwrap();
    assert_eq!(
        loaded.status_kind, "Running",
        "termination was not confirmed, so Cancelled must not be written"
    );
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
    assert!(
        handles.contains(&op).await,
        "an unconfirmed kill must leave the handle registered — it is the only \
         thing that can still reach the tree"
    );
}

/// Spec §8.4 case 4, in the window the prior implementation inverted: the turn
/// produced its own ending and the process is still exiting when `stop` takes
/// the lock. The tiebreak is the natural exit, not whoever got there first, so
/// the operation is Completed with the harness's own outcome and never
/// Cancelled. The window is made deterministic by the bus: the test proceeds
/// only after the turn-end has been observed, which is exactly the state the
/// arbitration has to get right.
#[tokio::test]
async fn a_turn_that_ended_on_its_own_is_never_overwritten_by_a_cancellation() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());
    let (bus, mut rx) = tokio::sync::broadcast::channel(16);

    let op = PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        harness(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: "slow-exit".into(),
            cwd: std::env::temp_dir(),
            resume_session_id: None,
        },
        bus,
    )
    .await
    .unwrap();

    loop {
        let (_op, item) = rx.recv().await.expect("the stream must reach its turn-end");
        if matches!(item, StreamItem::TurnEnd { .. }) {
            break;
        }
    }
    assert!(
        handles.contains(&op).await,
        "the process has not exited yet, so this is the contested window"
    );

    PlannerTurn::stop(runtime.clone(), handles.clone(), &op)
        .await
        .unwrap();

    let loaded = wait_for_terminal(&runtime, &op).await;
    assert_eq!(
        loaded.status_kind, "Completed",
        "a turn that ended on its own is not something Shadows stopped"
    );
    let outcome: serde_json::Value =
        serde_json::from_str(loaded.outcome_json.as_deref().expect("an outcome")).unwrap();
    assert_eq!(
        outcome,
        serde_json::json!({ "subtype": "success", "stop_reason": "end_turn" }),
        "the outcome the reader had already built must not be discarded"
    );
    assert!(
        loaded.cancel_requested_at.is_some(),
        "the request stays as history on a terminal non-cancelled record"
    );
}

/// Spec §8.4 case 4's other half: "persist Completed or **Failed** from the
/// real exit". A child that dies without a turn-end result did not complete,
/// and the exit status is the fact that says so.
#[tokio::test]
async fn a_child_that_dies_without_a_turn_end_is_failed_at_the_run_stage() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());
    let (bus, _rx) = tokio::sync::broadcast::channel(16);

    let op = PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        harness(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: "crash".into(),
            cwd: std::env::temp_dir(),
            resume_session_id: None,
        },
        bus,
    )
    .await
    .unwrap();

    let loaded = wait_for_terminal(&runtime, &op).await;
    assert_eq!(
        loaded.status_kind, "Failed",
        "a crashed turn is not a completed one"
    );
    assert_eq!(loaded.failure_stage.as_deref(), Some("Run"));
    let reason = loaded.failure_reason.unwrap_or_default();
    assert!(
        reason.contains("without a turn-end result"),
        "the reason must say which of the two failures this was, got: {reason}"
    );
    assert!(
        loaded.outcome_json.is_none(),
        "a failed turn has no outcome to report"
    );
}
