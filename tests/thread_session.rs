//! A thread remembers its harness session (spec §12.2, §12.3): the first
//! turn's session is recorded once that turn ends, a thread whose adapter was
//! closed resumes it at its next opening, and another thread has its own.
//! `fake_acp`'s `report` prompt answers with the session it runs in and how
//! that session was opened.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use shadows::agent::events::HarnessEvent;
use shadows::command::{CommandContext, fingerprint};
use shadows::events::Actor;
use shadows::operation::OperationId;
use shadows::planner::{LiveHandles, PlannerTurn, Sessions, StopOutcome};
use shadows::project::ProjectDirectory;
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::ThreadId;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/turn.rs"]
mod turn;

struct Fixture {
    _tmp: tempfile::TempDir,
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
    bus: tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
}

async fn fixture() -> (Fixture, ThreadId, ThreadId) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let ctx = |id: &str, kind: &str| CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &serde_json::json!({ "id": id })),
    };
    let dir = ProjectDirectory::resolve(tmp.path()).unwrap();
    let project = storage
        .create_project(
            &ctx("c1", "project.create"),
            "demo",
            "Demo",
            &dir,
            &shadows::agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let mut threads = Vec::new();
    for id in ["c2", "c3"] {
        let thread = storage
            .create_planning_thread(&ctx(id, "thread.create"), &project.id, id, "claude-code")
            .await
            .unwrap();
        threads.push(thread.id);
    }
    let sessions = acp::fake_sessions(&tmp.path().join("s.sqlite3")).await;
    let fixture = Fixture {
        _tmp: tmp,
        runtime: Arc::new(runtime),
        handles: Arc::new(LiveHandles::default()),
        sessions,
        bus: tokio::sync::broadcast::channel(16).0,
    };
    let second = threads.pop().unwrap();
    (fixture, threads.pop().unwrap(), second)
}

impl Fixture {
    async fn start(&self, thread: &ThreadId, prompt: &str) -> OperationId {
        turn::start_direct(
            &self.runtime,
            &self.handles,
            &self.sessions,
            &self.bus,
            thread,
            prompt,
        )
        .await
        .unwrap()
    }

    async fn wait_for_terminal(&self, op: &OperationId) -> String {
        for _ in 0..200 {
            let status = self.runtime.storage.get_operation(op).await.unwrap();
            if status.finished_at.is_some() {
                return status.status_kind;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("operation did not reach a terminal state in time");
    }

    /// Runs one `report` turn to completion and returns how its session was
    /// opened and which session it was.
    async fn session_of_a_turn(&self, thread: &ThreadId) -> (String, String) {
        let op = self.start(thread, "report").await;
        assert_eq!(self.wait_for_terminal(&op).await, "Completed");
        let entries = self
            .runtime
            .storage
            .list_thread_entries(thread)
            .await
            .unwrap();
        let reply = entries
            .iter()
            .rev()
            .find(|e| e.kind == "AgentMessage")
            .expect("an agent reply");
        let reported: Value = serde_json::from_str(&reply.body).unwrap();
        (
            reported["how"].as_str().unwrap().to_string(),
            reported["session"].as_str().unwrap().to_string(),
        )
    }

    async fn recorded(&self, thread: &ThreadId) -> Option<String> {
        self.runtime
            .storage
            .turn_context(thread)
            .await
            .unwrap()
            .harness_session_id
    }
}

#[tokio::test]
async fn a_closed_adapter_resumes_the_session_the_first_turn_started() {
    let (f, thread, other) = fixture().await;

    let (how, session) = f.session_of_a_turn(&thread).await;
    assert_eq!(how, "new");
    assert_eq!(f.recorded(&thread).await.as_deref(), Some(session.as_str()));

    // The adapter is still open: the next turn runs in the same session.
    assert_eq!(
        f.session_of_a_turn(&thread).await,
        ("new".into(), session.clone())
    );

    f.sessions.terminate(&thread).await.unwrap();
    assert_eq!(
        f.session_of_a_turn(&thread).await,
        ("resume".into(), session.clone())
    );

    let (how, own) = f.session_of_a_turn(&other).await;
    assert_eq!(how, "new");
    assert_ne!(own, session, "two threads share one session");
}

/// A first turn whose adapter had to be terminated never ended, so its
/// session is not recorded (§12.3): the next turn opens a new one rather than
/// resuming an id the harness may never have stored.
#[tokio::test]
async fn a_first_turn_stopped_by_termination_records_no_session() {
    let (f, thread, _) = fixture().await;

    let hung = f.start(&thread, "ignore-cancel").await;
    let outcome = PlannerTurn::stop(
        f.runtime.clone(),
        f.handles.clone(),
        f.sessions.clone(),
        &hung,
        Actor::user("local"),
    )
    .await
    .unwrap();
    assert_eq!(outcome, StopOutcome::Cancelled);
    assert_eq!(f.wait_for_terminal(&hung).await, "Cancelled");
    assert_eq!(f.recorded(&thread).await, None);

    let (how, _) = f.session_of_a_turn(&thread).await;
    assert_eq!(how, "new");
}
