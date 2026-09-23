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
use shadows::events::Actor;
use shadows::operation::{Operation, OperationId};
use shadows::planner::{LiveHandles, PlannerTurn, PlannerTurnRequest, StopOutcome};
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
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &shadows::project::ProjectDirectory::resolve(tmp.path()).unwrap(),
        )
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

/// Duplicated from `tests/containment.rs` rather than shared: each integration
/// test is its own crate, and a three-line helper is not worth a test-support
/// module that product code would then carry. The Unix arm keeps the zombie
/// distinction that file paid for — existence is not life.
#[cfg(windows)]
fn is_alive(pid: u32) -> bool {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'yes' }} else {{ 'no' }}"),
        ])
        .output()
        .expect("powershell should run");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

#[cfg(unix)]
fn is_alive(pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let Some(rest) = stat.rsplit_once(") ") else {
        return false;
    };
    !matches!(rest.1.chars().next(), Some('Z') | None)
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
        },
        bus,
    )
    .await
    .unwrap();

    wait_for_running(&runtime, &op).await;

    let outcome = PlannerTurn::stop(runtime.clone(), handles.clone(), &op, Actor::user("local"))
        .await
        .unwrap();
    assert_eq!(outcome, StopOutcome::Cancelled);

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

    let outcome = PlannerTurn::stop(runtime.clone(), handles, &op, Actor::user("local"))
        .await
        .unwrap();
    assert_eq!(outcome, StopOutcome::NotLive);

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

    let outcome = PlannerTurn::stop(runtime.clone(), handles.clone(), &op, Actor::user("local"))
        .await
        .unwrap();
    assert_eq!(
        outcome,
        StopOutcome::TerminationFailed,
        "a failed termination must be said, not reported like a success"
    );

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

/// The turn's verdict and the process's status are two facts, and a turn is
/// only Completed when both say so. A harness that reports a failing turn end
/// and then exits 0 — the evidence report's own "structured verdict and a
/// process status, and can cross-check them" — must not be recorded as a
/// completed turn just because the process was fine.
#[tokio::test]
async fn a_failing_turn_end_is_not_completed_even_on_a_clean_exit() {
    let (_t, runtime, thread) = fixture().await;
    let handles = Arc::new(LiveHandles::default());
    let (bus, _rx) = tokio::sync::broadcast::channel(16);

    let op = PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        harness(),
        PlannerTurnRequest {
            thread_id: thread.clone(),
            prompt: "failing-turn-end".into(),
        },
        bus,
    )
    .await
    .unwrap();

    let loaded = wait_for_terminal(&runtime, &op).await;
    assert_eq!(loaded.status_kind, "Failed");
    assert_eq!(loaded.failure_stage.as_deref(), Some("Run"));
    let reason = loaded.failure_reason.unwrap_or_default();
    assert!(
        reason.contains("failing turn end"),
        "the reason must name the harness's own verdict, got: {reason}"
    );
}

/// Spec §8.4 cases 3 and 4 in the one window where they meet: the turn has
/// reported its result and the process is still alive. Both obligations hold
/// at once and neither may be dropped for the other — the registered tree is
/// terminated (case 3), and the ending the turn already produced is the one
/// persisted (case 4's rule about the outcome), not `Cancelled` and not a Run
/// failure read off the exit status of a kill Shadows itself performed.
///
/// The window is made deterministic by the bus: the test proceeds only after
/// the turn-end has been observed, which is exactly the state the arbitration
/// has to get right. `fake_claude`'s `slow-exit` then stays alive for two
/// minutes, so a `stop` that terminated nothing cannot be mistaken for one
/// that did.
#[tokio::test]
async fn a_cancelled_turn_that_had_already_ended_keeps_its_outcome_and_loses_its_tree() {
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
        },
        bus,
    )
    .await
    .unwrap();

    loop {
        let (published_for, _op, item) =
            rx.recv().await.expect("the stream must reach its turn-end");
        assert_eq!(
            published_for, thread,
            "each bus item names the turn's thread"
        );
        if matches!(item, StreamItem::TurnEnd { .. }) {
            break;
        }
    }
    let pid = handles
        .pid(&op)
        .await
        .expect("the turn is still registered");
    assert!(
        is_alive(pid),
        "the process has not exited yet, so this is the contested window"
    );

    let outcome = PlannerTurn::stop(runtime.clone(), handles.clone(), &op, Actor::user("local"))
        .await
        .unwrap();
    assert_eq!(outcome, StopOutcome::TerminatedAfterTurnEnd);

    let loaded = wait_for_terminal(&runtime, &op).await;
    assert_eq!(
        loaded.status_kind, "Completed",
        "the turn's own ending wins: not Cancelled, and not a Run failure read \
         off the exit status of a kill Shadows itself performed"
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

    // §8.4 case 3's other half. Keeping the outcome is not a reason to leave
    // the tree running: `fake_claude` would still be sleeping for two minutes
    // if `stop` had declined to terminate it.
    for _ in 0..50 {
        if !is_alive(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the cancelled turn's tree survived: pid={pid}");
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
