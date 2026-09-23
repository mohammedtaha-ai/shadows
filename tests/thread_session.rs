//! A thread remembers its harness session (evidence
//! `docs/evidence/harness/SERVE_STREAM_SPIKE.md` Finding 3): the first turn
//! starts one with `--session-id`, every later turn on the same thread
//! resumes it with `--resume`, and a session is recorded only once the
//! harness has reached its turn-end. `fake_claude`'s `report-invocation`
//! prompt echoes the arguments it was started with.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use shadows::agent::claude::ClaudeHarness;
use shadows::command::{CommandContext, fingerprint};
use shadows::events::Actor;
use shadows::operation::OperationId;
use shadows::planner::{LiveHandles, PlannerTurn, PlannerTurnRequest};
use shadows::project::ProjectDirectory;
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::ThreadId;

struct Fixture {
    _tmp: tempfile::TempDir,
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    harness: Arc<ClaudeHarness>,
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
        .create_project(&ctx("c1", "project.create"), "demo", "Demo", &dir)
        .await
        .unwrap();
    let mut threads = Vec::new();
    for id in ["c2", "c3"] {
        let thread = storage
            .create_planning_thread(&ctx(id, "thread.create"), &project.id, id)
            .await
            .unwrap();
        threads.push(thread.id);
    }
    let fixture = Fixture {
        _tmp: tmp,
        runtime: Arc::new(runtime),
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(
            PathBuf::from(env!("CARGO_BIN_EXE_fake_claude")),
            "fake-1".into(),
        )),
    };
    let second = threads.pop().unwrap();
    (fixture, threads.pop().unwrap(), second)
}

impl Fixture {
    async fn start(&self, thread: &ThreadId, prompt: &str) -> OperationId {
        let (bus, _) = tokio::sync::broadcast::channel(16);
        PlannerTurn::start(
            self.runtime.clone(),
            self.handles.clone(),
            self.harness.clone(),
            PlannerTurnRequest {
                thread_id: thread.clone(),
                prompt: prompt.into(),
            },
            bus,
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

    /// Runs one `report-invocation` turn to completion and returns the
    /// arguments the harness was started with.
    async fn args_of_a_turn(&self, thread: &ThreadId) -> Vec<String> {
        let op = self.start(thread, "report-invocation").await;
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
        serde_json::from_value(reported["args"].clone()).unwrap()
    }
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1).map(String::as_str)
}

#[tokio::test]
async fn later_turns_resume_the_session_the_first_turn_started() {
    let (f, thread, other) = fixture().await;

    let first = f.args_of_a_turn(&thread).await;
    let session = flag(&first, "--session-id").expect("the first turn starts a session");
    assert_eq!(flag(&first, "--resume"), None, "{first:?}");

    let second = f.args_of_a_turn(&thread).await;
    assert_eq!(flag(&second, "--resume"), Some(session), "{second:?}");
    assert_eq!(flag(&second, "--session-id"), None, "{second:?}");

    let elsewhere = f.args_of_a_turn(&other).await;
    assert_eq!(flag(&elsewhere, "--resume"), None, "{elsewhere:?}");
    let own = flag(&elsewhere, "--session-id").expect("another thread starts its own");
    assert_ne!(own, session, "two threads share one session");
}

/// A first turn stopped before its turn-end may have left no session in the
/// harness's store. Recording it would make every later turn on the thread a
/// `--resume` of an id the harness rejects, so the next turn starts afresh.
#[tokio::test]
async fn a_first_turn_stopped_before_its_turn_end_records_no_session() {
    let (f, thread, _) = fixture().await;

    let hung = f.start(&thread, "hang").await;
    PlannerTurn::stop(
        f.runtime.clone(),
        f.handles.clone(),
        &hung,
        Actor::user("local"),
    )
    .await
    .unwrap();
    assert_eq!(f.wait_for_terminal(&hung).await, "Cancelled");

    let next = f.args_of_a_turn(&thread).await;
    assert_eq!(flag(&next, "--resume"), None, "{next:?}");
    assert!(flag(&next, "--session-id").is_some(), "{next:?}");
}
