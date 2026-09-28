//! Spec §8.5, daemon shutdown: `Graceful` is written only when every operation
//! the runtime owns is terminal; anything short of that is `Escalated`, and a
//! stopping runtime takes no new turn. Driven through `testing::shut_down`,
//! which is what `shadows serve` runs on its stop signal, with `fake_acp`
//! standing in for the adapter.
//!
//! The storage half of the same rule — `Graceful` refused over unfinished work
//! whoever asks — is `tests/recovery.rs`, next to the recovery that relies on it.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use shadows::cli::router;
use shadows_agent::events::HarnessEvent;
use shadows_core::StartError;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::planner::Sessions;
use shadows_core::runtime::Runtime;
use shadows_core::storage::{StopKind, Storage};
use shadows_core::testing::{LiveHandles, shut_down};
use shadows_core::thread::ThreadId;
use shadows_core::{AppCore, CoreParts};
use shadows_core::{Operation, OperationId};
use shadows_http::AppState;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/turn.rs"]
mod turn;

type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

struct Fixture {
    _tmp: tempfile::TempDir,
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
    thread: ThreadId,
    bus: Bus,
}

async fn fixture() -> Fixture {
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
    let thread = runtime
        .storage
        .create_planning_thread(&tctx, &project.id, "T", "claude-code")
        .await
        .unwrap();
    let sessions = acp::fake_sessions(&tmp.path().join("s.sqlite3")).await;
    Fixture {
        _tmp: tmp,
        runtime,
        handles: Arc::new(LiveHandles::default()),
        sessions,
        thread: thread.id,
        bus: tokio::sync::broadcast::channel(64).0,
    }
}

impl Fixture {
    async fn start(&self, prompt: &str) -> Result<OperationId, StartError> {
        turn::start_direct(
            &self.runtime,
            &self.handles,
            &self.sessions,
            &self.bus,
            &self.thread,
            prompt,
        )
        .await
    }

    async fn shut_down(
        &self,
        confirm_within: Duration,
        escalate: impl Future<Output = ()>,
    ) -> StopKind {
        shut_down(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            confirm_within,
            escalate,
        )
        .await
        .expect("the stop must be recorded")
    }

    async fn operation(&self, op: &OperationId) -> Operation {
        self.runtime.storage.get_operation(op).await.unwrap()
    }

    /// What the runtime row says, read back rather than trusted from the
    /// value `shut_down` returned.
    async fn recorded_stop_kind(&self) -> Option<String> {
        sqlx::query_scalar("SELECT stop_kind FROM runtime_instance WHERE id = ?")
            .bind(self.runtime.instance_id.as_str())
            .fetch_one(self.runtime.storage.reader())
            .await
            .unwrap()
    }

    async fn wait_for_status(&self, op: &OperationId, wanted: &str) -> Operation {
        for _ in 0..200 {
            let loaded = self.operation(op).await;
            if loaded.status_kind == wanted {
                return loaded;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("operation never reached {wanted}");
    }
}

/// No second stop signal ever arrives.
fn no_second_signal() -> impl Future<Output = ()> {
    std::future::pending()
}

/// Duplicated from `crates/shadows-process/tests/containment.rs`, for the reason
/// `tests/planner_turn.rs` gives.
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

/// The ordinary stop: one turn already finished, one still running. The
/// running one is cancelled and confirmed (`Cancelled`), the adapter is closed
/// (its process gone), and only then is `Graceful` written.
#[tokio::test]
async fn a_shutdown_that_confirms_every_operation_records_graceful() {
    let f = fixture().await;
    let finished = f.start("hi").await.unwrap();
    f.wait_for_status(&finished, "Completed").await;
    let running = f.start("hang").await.unwrap();
    f.wait_for_status(&running, "Running").await;
    let pid = f
        .sessions
        .pid(&f.thread)
        .await
        .expect("the adapter is live");

    let kind = f
        .shut_down(Duration::from_secs(20), no_second_signal())
        .await;

    assert_eq!(kind, StopKind::Graceful);
    assert_eq!(f.recorded_stop_kind().await.as_deref(), Some("Graceful"));
    assert_eq!(f.operation(&running).await.status_kind, "Cancelled");
    assert_eq!(f.operation(&finished).await.status_kind, "Completed");
    assert!(!is_alive(pid), "Cancelled was written over a live process");
}

/// The window the old shutdown fell into: the prompt has answered but the
/// watcher is still writing the turn's ending, so `stop` finds it resolved by
/// the turn and returns before that write. `Graceful` must wait for it rather
/// than be recorded over a `Running` row.
#[tokio::test]
async fn a_turn_whose_outcome_is_still_being_written_is_terminal_before_graceful() {
    let f = fixture().await;
    let mut rx = f.bus.subscribe();
    let op = f.start("hi").await.unwrap();
    loop {
        let (_, _, item) = rx.recv().await.expect("the turn must reach its turn-end");
        if matches!(item, HarnessEvent::TurnEnd { .. }) {
            break;
        }
    }

    let kind = f
        .shut_down(Duration::from_secs(20), no_second_signal())
        .await;

    // Read at once, with nothing waited on in between: `Graceful` is a claim
    // about the moment it was written.
    let loaded = f.operation(&op).await;
    assert_eq!(kind, StopKind::Graceful);
    assert_eq!(
        loaded.status_kind, "Completed",
        "Graceful was recorded before the turn's own ending was"
    );
}

/// Spec §8.4 case 6 during shutdown. A tree that could not be terminated is not
/// stopped: the operation is not `Cancelled`, and the runtime records
/// `Escalated` — at once, not after waiting out the bound on a tree already
/// known to be alive.
#[tokio::test]
async fn a_termination_failure_during_shutdown_is_escalated_and_never_cancelled() {
    let f = fixture().await;
    // `ignore-cancel`: the harness never confirms, so Stop must terminate.
    let op = f.start("ignore-cancel").await.unwrap();
    f.wait_for_status(&op, "Running").await;
    assert!(
        f.sessions.force_termination_failure(&f.thread).await,
        "the adapter must still be live for this test to mean anything"
    );

    let kind = tokio::time::timeout(
        Duration::from_secs(10),
        f.shut_down(Duration::from_secs(120), no_second_signal()),
    )
    .await
    .expect("a known-unconfirmable termination must not wait out the bound");

    assert_eq!(kind, StopKind::Escalated);
    assert_eq!(f.recorded_stop_kind().await.as_deref(), Some("Escalated"));
    let loaded = f.operation(&op).await;
    assert_eq!(
        loaded.status_kind, "Running",
        "termination failed, so the operation must not be Cancelled"
    );
    assert!(loaded.cancel_requested_at.is_some());
}

/// Work this runtime owns but holds no handle for — a `Pending` row whose
/// spawn never registered — can never be confirmed. The bound runs out and
/// the stop is `Escalated`; the row is left for recovery, not invented a
/// terminal state.
#[tokio::test]
async fn unconfirmed_work_is_escalated_when_the_bound_runs_out() {
    let f = fixture().await;
    let pending = f
        .runtime
        .storage
        .create_pending_operation(&f.thread, &f.runtime.instance_id)
        .await
        .unwrap();

    let kind = tokio::time::timeout(
        Duration::from_secs(10),
        f.shut_down(Duration::from_millis(300), no_second_signal()),
    )
    .await
    .expect("the bound must end the wait");

    assert_eq!(kind, StopKind::Escalated);
    assert_eq!(f.recorded_stop_kind().await.as_deref(), Some("Escalated"));
    assert_eq!(f.operation(&pending).await.status_kind, "Pending");
}

/// Spec §8.5: a second stop signal stops the waiting. The bound here is far
/// longer than the test allows, so only the signal can end it.
#[tokio::test]
async fn a_second_stop_signal_escalates_at_once() {
    let f = fixture().await;
    let pending = f
        .runtime
        .storage
        .create_pending_operation(&f.thread, &f.runtime.instance_id)
        .await
        .unwrap();
    let (signal, second) = tokio::sync::oneshot::channel::<()>();
    let second = async move {
        let _ = second.await;
    };
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = signal.send(());
    });

    let kind = tokio::time::timeout(
        Duration::from_secs(10),
        f.shut_down(Duration::from_secs(600), second),
    )
    .await
    .expect("the second signal must end the wait");

    assert_eq!(kind, StopKind::Escalated);
    assert_eq!(f.recorded_stop_kind().await.as_deref(), Some("Escalated"));
    assert_eq!(f.operation(&pending).await.status_kind, "Pending");
}

/// A stopping runtime takes no new turn: refused with a typed error, with
/// nothing durable created for it — at the planner, at storage, and over
/// HTTP, where the refusal is a 503 and the prompt is not recorded either.
#[tokio::test]
async fn a_turn_requested_after_shutdown_began_is_refused() {
    let f = fixture().await;
    let kind = f
        .shut_down(Duration::from_secs(20), no_second_signal())
        .await;
    assert_eq!(kind, StopKind::Graceful);

    let refused = f.start("hi").await;
    assert!(
        matches!(refused, Err(StartError::RuntimeStopping)),
        "got {refused:?}"
    );
    let stored = f
        .runtime
        .storage
        .create_pending_operation(&f.thread, &f.runtime.instance_id)
        .await;
    assert!(
        stored.is_err(),
        "storage must not create an operation for a stopped runtime"
    );

    let (_stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        core: AppCore::assemble(CoreParts {
            storage: f.runtime.storage.clone(),
            runtime: f.runtime.clone(),
            sessions: f.sessions.clone(),
            handles: f.handles.clone(),
            bus: f.bus.clone(),
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.to_string(),
        }),
        allowed_origins: Vec::new(),
        shutdown,
    });
    let response = app
        .oneshot(
            Request::post(format!("/api/threads/{}/turns", f.thread.as_str()))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"command_id":"after-stop","prompt":"hi","model":"fake-large","mode":"acceptEdits","effort":"high"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["code"], "RUNTIME_STOPPING", "{body}");

    let operations = f
        .runtime
        .storage
        .list_operations_for_thread(&f.thread)
        .await
        .unwrap();
    assert!(operations.is_empty(), "{operations:?}");
    let entries = f
        .runtime
        .storage
        .list_thread_entries(&f.thread)
        .await
        .unwrap();
    assert!(entries.is_empty(), "a refused turn records no prompt");
}
