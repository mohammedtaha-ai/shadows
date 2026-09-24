//! Spec §2.10: durable replay, then a no-gap handoff to live, then
//! de-duplication by durable sequence.
//!
//! The first two tests own the replay half — what a cursor means and what
//! reading after one returns. The rest own the half that cannot be checked by
//! reading storage at all: that the live subscriptions exist *before* the
//! replay is read, so nothing published or committed during the replay falls
//! into the gap between the two; that a durable event committed after the
//! handoff reaches the open stream exactly once; and that a stream carries only
//! its own thread.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use shadows::agent::events::HarnessEvent;
use shadows::command::{CommandContext, fingerprint};
use shadows::events::{Actor, EventCursor};
use shadows::operation::OperationId;
use shadows::planner::LiveHandles;
use shadows::project::Project;
use shadows::protocol::AppState;
use shadows::protocol::sse::{SubscribeQuery, subscribe};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::{NewThreadEntry, PlanningThread, ThreadId};
use tokio_stream::StreamExt;

#[path = "fixtures/acp.rs"]
mod acp;

/// A project and a planning thread, which between them have already written
/// two durable events: `ProjectCreated` (no thread) and `PlanningThreadCreated`.
async fn seed(storage: &Storage) -> (Project, PlanningThread) {
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &params),
    };
    let dir = shadows::project::ProjectDirectory::resolve(&std::env::temp_dir()).unwrap();
    let project = storage
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &dir,
            &shadows::agent::policy::default_modes(),
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
        .unwrap();
    (project, thread)
}

fn user_message(body: &str) -> NewThreadEntry<'_> {
    NewThreadEntry {
        kind: "UserMessage",
        author: Actor::user("local"),
        body,
        refs: &[],
        operation_id: None,
    }
}

/// Spec §2.10. Reading after a cursor returns exactly the events the client has
/// not seen, in sequence order, with no gap and no repeat.
#[tokio::test]
async fn reading_after_a_cursor_returns_the_unseen_tail_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let (_project, thread) = seed(&storage).await;

    let mid = storage.current_cursor().await.unwrap();

    for body in ["one", "two", "three"] {
        storage
            .append_thread_entry(&thread.id, user_message(body))
            .await
            .unwrap();
    }

    let tail = storage
        .read_events_after(mid, &thread.id, 100)
        .await
        .unwrap();
    assert_eq!(tail.len(), 3, "exactly the events after the cursor");
    let seqs: Vec<i64> = tail.iter().map(|e| e.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted, "events arrive in sequence order");
    assert!(tail.iter().all(|e| e.kind == "ThreadEntryAppended"));

    // Re-reading from the same cursor is idempotent.
    let again = storage
        .read_events_after(mid, &thread.id, 100)
        .await
        .unwrap();
    assert_eq!(seqs, again.iter().map(|e| e.seq).collect::<Vec<_>>());

    // Reading after the last delivered seq returns nothing.
    let after_all = EventCursor(*seqs.last().unwrap());
    assert!(
        storage
            .read_events_after(after_all, &thread.id, 100)
            .await
            .unwrap()
            .is_empty()
    );
}

/// A cursor from before any event returns the whole thread history.
#[tokio::test]
async fn a_zero_cursor_replays_the_whole_thread() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let (_project, thread) = seed(&storage).await;
    storage
        .append_thread_entry(&thread.id, user_message("hi"))
        .await
        .unwrap();

    let all = storage
        .read_events_after(EventCursor(0), &thread.id, 100)
        .await
        .unwrap();
    assert_eq!(all.len(), 2, "thread creation and the entry");
    assert_eq!(all[0].kind, "PlanningThreadCreated");
    assert_eq!(all[1].kind, "ThreadEntryAppended");
}

/// Spec §2.10's second clause, and the only one that can observe a gap.
///
/// The subscription is taken while the handler is still running, before a
/// single durable row is read. This test publishes a live item in the window
/// the replay occupies: on a `#[tokio::test]` current-thread runtime the task
/// that performs the replay cannot have been polled yet when `subscribe`
/// returns, so an implementation that subscribes after the replay has no
/// receiver at that instant and loses the item outright. Here it must both
/// accept the send and deliver it after the handoff marker.
#[tokio::test]
async fn an_event_published_during_the_replay_survives_the_handoff() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (_project, thread) = seed(&storage).await;
    for body in ["one", "two"] {
        storage
            .append_thread_entry(&thread.id, user_message(body))
            .await
            .unwrap();
    }
    let live = Live::start(&tmp, storage).await;
    let mut stream = live.open(&thread.id).await;

    // No await between the line above and this one: the replay task has not
    // run. If the subscription were taken inside it, there would be no
    // receiver here and this send would fail.
    live.bus
        .send(delta(&thread.id, "during-replay"))
        .expect("the live subscription must exist before the replay is read");

    let text = stream.read_until("during-replay").await;

    let durable = text.find("event: durable").expect("durable replay first");
    let caught_up = text
        .find("event: caught-up")
        .expect("then the handoff marker");
    let live_at = text.find("during-replay").unwrap();
    assert!(
        durable < caught_up && caught_up < live_at,
        "replay, then handoff, then live: {text}"
    );
    assert_eq!(
        text.matches("event: durable").count(),
        3,
        "thread creation and both entries replay exactly once: {text}"
    );
}

/// Spec §2.4's live publication after commit, seen from the stream: a durable
/// event committed after the handoff reaches a connected client, with its
/// `seq`, without reconnecting.
#[tokio::test]
async fn a_durable_event_committed_after_the_handoff_arrives_live() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (_project, thread) = seed(&storage).await;
    let live = Live::start(&tmp, storage.clone()).await;
    let mut stream = live.open(&thread.id).await;
    stream.read_until("event: caught-up").await;

    storage
        .append_thread_entry(&thread.id, user_message("after the handoff"))
        .await
        .unwrap();
    let seq = storage.current_cursor().await.unwrap().0;

    let text = stream.read_until(&format!("\"seq\":{seq}")).await;
    assert_eq!(durable_seqs(&text).last(), Some(&seq), "{text}");
}

/// Spec §2.10's no-gap handoff for durable events. The entry is committed
/// after both live subscriptions exist and before the replay task has run, so
/// it is reachable from both the replay and the live phase's re-read. It must
/// arrive exactly once: not lost between them, and not repeated by both.
#[tokio::test]
async fn a_durable_event_committed_during_the_replay_arrives_exactly_once() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (_project, thread) = seed(&storage).await;
    let live = Live::start(&tmp, storage.clone()).await;
    let mut stream = live.open(&thread.id).await;

    storage
        .append_thread_entry(&thread.id, user_message("during the replay"))
        .await
        .unwrap();
    let seq = storage.current_cursor().await.unwrap().0;

    // A marker committed after the handoff: once it is read, the replay, the
    // handoff and the live re-read triggered by the first commit have all had
    // their turn, so any repeat would already be on the stream.
    stream.read_until("event: caught-up").await;
    storage
        .append_thread_entry(&thread.id, user_message("marker"))
        .await
        .unwrap();
    let marker = storage.current_cursor().await.unwrap().0;
    let text = stream.read_until(&format!("\"seq\":{marker}")).await;

    let seqs = durable_seqs(&text);
    assert_eq!(
        seqs.iter().filter(|s| **s == seq).count(),
        1,
        "the entry is delivered once: {text}"
    );
    let mut deduped = seqs.clone();
    deduped.dedup();
    assert_eq!(seqs, deduped, "no durable event is repeated: {text}");
}

/// A subscription names one thread. Another thread's transient items are not
/// delivered to it, while its own are.
#[tokio::test]
async fn a_subscriber_receives_only_its_own_threads_live_items() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (project, thread_a) = seed(&storage).await;
    let thread_b = storage
        .create_planning_thread(&thread_ctx("c3"), &project.id, "B", "claude-code")
        .await
        .unwrap();
    let live = Live::start(&tmp, storage).await;
    let mut stream = live.open(&thread_a.id).await;
    stream.read_until("event: caught-up").await;

    // Sent in this order on one broadcast channel, so B's item would be read
    // first if it were forwarded at all.
    live.bus.send(delta(&thread_b.id, "for-thread-b")).unwrap();
    live.bus.send(delta(&thread_a.id, "for-thread-a")).unwrap();

    let text = stream.read_until("for-thread-a").await;
    assert!(
        !text.contains("for-thread-b"),
        "thread B's item reached thread A: {text}"
    );
}

/// Spec §6.18 scope semantics, seen from the client: an operation's
/// transitions reach its thread's stream as `durable` events — the start in the
/// replay, the terminal transition live.
#[tokio::test]
async fn operation_transitions_reach_their_threads_stream() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (_project, thread) = seed(&storage).await;
    let live = Live::start(&tmp, storage.clone()).await;
    let runtime = &live.state.runtime.instance_id;
    let op = storage
        .create_pending_operation(&thread.id, runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, runtime).await.unwrap();

    let mut stream = live.open(&thread.id).await;
    let replayed = stream.read_until("event: caught-up").await;
    assert!(
        durable_kinds(&replayed).contains(&"OperationStarted".to_string()),
        "the start replays on the thread's stream: {replayed}"
    );

    storage
        .mark_operation_completed(&op, serde_json::json!({}))
        .await
        .unwrap();
    let text = stream.read_until("OperationCompleted").await;
    assert_eq!(
        durable_kinds(&text).last().map(String::as_str),
        Some("OperationCompleted"),
        "the terminal transition arrives live as a durable event: {text}"
    );
}

/// Every `durable` event's `kind`, in the order the stream delivered them.
fn durable_kinds(text: &str) -> Vec<String> {
    text.split("\n\n")
        .filter(|frame| frame.lines().any(|l| l == "event: durable"))
        .filter_map(|frame| frame.lines().find_map(|l| l.strip_prefix("data: ")))
        .map(|data| {
            let event: serde_json::Value = serde_json::from_str(data).unwrap();
            event["kind"].as_str().unwrap().to_string()
        })
        .collect()
}

/// The daemon's state as `subscribe` sees it, with a bus the test publishes on.
struct Live {
    state: AppState,
    bus: tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
    // Held for the whole test: a dropped sender reads as a stopping daemon,
    // which ends the live phase the test is waiting on.
    _stopping: tokio::sync::watch::Sender<bool>,
}

impl Live {
    async fn start(tmp: &tempfile::TempDir, storage: Arc<Storage>) -> Self {
        let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
        let (bus, _) = tokio::sync::broadcast::channel(64);
        let (stopping, shutdown) = tokio::sync::watch::channel(false);
        let state = AppState {
            runtime: Arc::new(runtime),
            storage,
            handles: Arc::new(LiveHandles::default()),
            sessions: acp::fake_sessions(&tmp.path().join("s.sqlite3")).await,
            bus: bus.clone(),
            allowed_origins: Vec::new(),
            shutdown,
        };
        Live {
            state,
            bus,
            _stopping: stopping,
        }
    }

    /// Subscribes from the beginning of the thread. The replay task has not
    /// run when this returns.
    async fn open(&self, thread_id: &ThreadId) -> Stream {
        let response = subscribe(
            State(self.state.clone()),
            Query(SubscribeQuery {
                thread_id: thread_id.clone(),
                after: 0,
            }),
        )
        .await
        .into_response();
        Stream {
            body: response.into_body().into_data_stream(),
            text: String::new(),
        }
    }
}

struct Stream {
    body: axum::body::BodyDataStream,
    text: String,
}

impl Stream {
    /// Reads until `needle` has arrived; returns everything read so far.
    async fn read_until(&mut self, needle: &str) -> String {
        while !self.text.contains(needle) {
            let chunk = tokio::time::timeout(Duration::from_secs(10), self.body.next())
                .await
                .unwrap_or_else(|_| panic!("stalled before {needle:?}: {}", self.text))
                .expect("the stream ended early")
                .unwrap();
            self.text.push_str(std::str::from_utf8(&chunk).unwrap());
        }
        self.text.clone()
    }
}

fn delta(thread_id: &ThreadId, text: &str) -> (ThreadId, OperationId, HarnessEvent) {
    (
        thread_id.clone(),
        OperationId::from_literal("op-live"),
        HarnessEvent::Chunk {
            message_id: None,
            text: text.into(),
        },
    )
}

fn thread_ctx(command_id: &str) -> CommandContext {
    let params = serde_json::json!({ "title": command_id });
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: "thread.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("thread.create", &params),
    }
}

/// Every `durable` event's `seq`, in the order the stream delivered them.
fn durable_seqs(text: &str) -> Vec<i64> {
    text.split("\n\n")
        .filter(|frame| frame.lines().any(|l| l == "event: durable"))
        .filter_map(|frame| frame.lines().find_map(|l| l.strip_prefix("data: ")))
        .map(|data| {
            let event: serde_json::Value = serde_json::from_str(data).unwrap();
            event["seq"].as_i64().unwrap()
        })
        .collect()
}
