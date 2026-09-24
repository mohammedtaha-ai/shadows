## Task 11: Durable replay with a no-gap handoff to live

**Files:**
- Create: `src/storage/sqlite/events_read.rs`, `src/protocol/mod.rs`, `src/protocol/handlers.rs`, `src/protocol/sse.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`, `src/cli/mod.rs`
- Test: `tests/resync.rs`

**Where the code below goes.** This task's steps present the protocol code as one
block, but it does not land as one file. `protocol/` is a named accretion point in
CLAUDE.md — every feature this project ever adds puts a route here — so the split
happens on the way in, not after it hurts:

| File | Its one job |
|---|---|
| `src/protocol/mod.rs` | wiring: `AppState`, `router()`, the `Failure` → HTTP mapping, and `index` |
| `src/protocol/handlers.rs` | what each route does: the seven handlers, their request structs, and the `ctx` helper |
| `src/protocol/sse.rs` | the durable-replay-then-live stream |

`handlers.rs` items are `pub(super)` — `router()` is the only thing that names them.
Splitting by responsibility is the point; do not instead create a `protocol/types.rs`
or `protocol/utils.rs`, which is the same pile under a new name.

**Interfaces:**
- Consumes: `EventCursor` (Task 3); `Storage` (Tasks 2–10); `LiveHandles`, `PlannerTurn` (Task 10).
- Produces: `Storage::current_cursor() -> Result<EventCursor, StorageError>`; `Storage::read_events_after(cursor: EventCursor, thread_id: &str, limit: i64) -> Result<Vec<StoredEvent>, StorageError>`; `StoredEvent { seq, kind, operation_id, payload_json, created_at }`; `protocol::router(AppState) -> axum::Router`; `AppState { runtime, storage, handles, harness, bus }`.

**The guarantee.** Spec §2.10: durable replay, then a no-gap handoff to live, with de-duplication by durable sequence. The subscription is opened *before* the replay is read, so an event committed during the replay is buffered rather than lost.

- [ ] **Step 1: Write the failing tests**

`tests/resync.rs`:

```rust
use shadows::events::EventCursor;
use shadows::storage::Storage;

/// Spec §2.10. Reading after a cursor returns exactly the events the client has
/// not seen, in sequence order, with no gap and no repeat.
#[tokio::test]
async fn reading_after_a_cursor_returns_the_unseen_tail_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(), principal_id: "local".into(),
        command_id: "c1".into(), command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(), command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage.create_planning_thread(&tctx, &project.id, "T").await.unwrap();

    let mid = storage.current_cursor().await.unwrap();

    for body in ["one", "two", "three"] {
        storage.append_thread_entry(&thread.id, "UserMessage", "User", "local", body, &[])
            .await.unwrap();
    }

    let tail = storage.read_events_after(mid, &thread.id, 100).await.unwrap();
    assert_eq!(tail.len(), 3, "exactly the events after the cursor");
    let seqs: Vec<i64> = tail.iter().map(|e| e.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted, "events arrive in sequence order");
    assert!(tail.iter().all(|e| e.kind == "ThreadEntryAppended"));

    // Re-reading from the same cursor is idempotent.
    let again = storage.read_events_after(mid, &thread.id, 100).await.unwrap();
    assert_eq!(seqs, again.iter().map(|e| e.seq).collect::<Vec<_>>());

    // Reading after the last delivered seq returns nothing.
    let after_all = EventCursor(*seqs.last().unwrap());
    assert!(storage.read_events_after(after_all, &thread.id, 100).await.unwrap().is_empty());
}

/// A cursor from before any event returns the whole thread history.
#[tokio::test]
async fn a_zero_cursor_replays_the_whole_thread() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(), principal_id: "local".into(),
        command_id: "c1".into(), command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(), command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage.create_planning_thread(&tctx, &project.id, "T").await.unwrap();
    storage.append_thread_entry(&thread.id, "UserMessage", "User", "local", "hi", &[])
        .await.unwrap();

    let all = storage.read_events_after(EventCursor(0), &thread.id, 100).await.unwrap();
    assert_eq!(all.len(), 2, "thread creation and the entry");
    assert_eq!(all[0].kind, "PlanningThreadCreated");
    assert_eq!(all[1].kind, "ThreadEntryAppended");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test resync`
Expected: FAIL — `current_cursor` and `read_events_after` do not exist.

- [ ] **Step 3: Write `src/storage/sqlite/events_read.rs`**

```rust
use crate::events::EventCursor;
use super::{Storage, StorageError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredEvent {
    pub seq: i64,
    pub kind: String,
    pub operation_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
}

impl Storage {
    /// The highest sequence committed so far. Spec §2.10: the snapshot and the
    /// cursor must come from the same read, so a caller building a snapshot
    /// takes this inside that same read transaction.
    pub async fn current_cursor(&self) -> Result<EventCursor, StorageError> {
        let seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM durable_event")
            .fetch_one(self.reader())
            .await?;
        Ok(EventCursor(seq.unwrap_or(0)))
    }

    /// Thread-scoped replay. Ordering is by `seq` explicitly — never by
    /// insertion order. On SQLite, sequence order is commit order; see the
    /// OPEN block in §6.18 before assuming that on another backend.
    pub async fn read_events_after(
        &self,
        cursor: EventCursor,
        thread_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        let rows: Vec<(i64, String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT seq, kind, operation_id, payload_json, created_at
               FROM durable_event
              WHERE thread_id = ? AND seq > ?
              ORDER BY seq
              LIMIT ?",
        )
        .bind(thread_id)
        .bind(cursor.0)
        .bind(limit)
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(|r| StoredEvent {
            seq: r.0, kind: r.1, operation_id: r.2, payload_json: r.3, created_at: r.4,
        }).collect())
    }
}
```

- [ ] **Step 4: Write `src/protocol/sse.rs`**

```rust
use std::convert::Infallible;

use axum::extract::{Query, State};
use axum::response::sse::{Event, Sse};
use tokio_stream::wrappers::ReceiverStream;

use crate::agent::StreamItem;
use crate::events::EventCursor;
use super::AppState;

#[derive(serde::Deserialize)]
pub struct SubscribeQuery {
    pub thread_id: String,
    /// The last durable sequence this client has already applied.
    #[serde(default)]
    pub after: i64,
}

/// Spec §2.10: durable replay, then a no-gap handoff to live, then
/// de-duplication by durable sequence.
///
/// The order below is the whole guarantee. The live subscription is taken
/// FIRST, so anything committed while the replay is being read lands in the
/// broadcast buffer instead of falling into the gap between them. Replayed
/// events carry their `seq`; the client discards any live event whose `seq` it
/// has already applied.
pub async fn subscribe(
    State(state): State<AppState>,
    Query(q): Query<SubscribeQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1024);
    let mut live = state.bus.subscribe(); // taken before the replay is read

    tokio::spawn(async move {
        // 1. Durable replay.
        let mut last_seq = q.after;
        loop {
            let batch = match state
                .storage
                .read_events_after(EventCursor(last_seq), &q.thread_id, 500)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(Ok(Event::default().event("fatal").data(e.to_string()))).await;
                    return;
                }
            };
            if batch.is_empty() {
                break;
            }
            for ev in batch {
                last_seq = ev.seq;
                let payload = serde_json::json!({
                    "seq": ev.seq, "kind": ev.kind, "payload": ev.payload_json,
                });
                if tx.send(Ok(Event::default().event("durable").data(payload.to_string())))
                    .await.is_err()
                {
                    return;
                }
            }
        }

        // 2. Handoff. Tell the client where the durable replay ended so it can
        // de-duplicate anything the live stream repeats.
        let _ = tx
            .send(Ok(Event::default().event("caught-up").data(last_seq.to_string())))
            .await;

        // 3. Live. Transient deltas are forwarded and never stored.
        loop {
            match live.recv().await {
                Ok((op_id, item)) => {
                    let ev = match item {
                        StreamItem::Delta { text } => Event::default()
                            .event("delta")
                            .data(serde_json::json!({ "op": op_id, "text": text }).to_string()),
                        StreamItem::Entry { uuid, role, text } => Event::default()
                            .event("entry")
                            .data(serde_json::json!({
                                "op": op_id, "uuid": uuid, "role": role, "text": text
                            }).to_string()),
                        StreamItem::TurnEnd { subtype, stop_reason } => Event::default()
                            .event("turn-end")
                            .data(serde_json::json!({
                                "op": op_id, "subtype": subtype, "stop_reason": stop_reason
                            }).to_string()),
                        StreamItem::Operational { label, .. } => Event::default()
                            .event("meta")
                            .data(serde_json::json!({ "op": op_id, "label": label }).to_string()),
                        StreamItem::Unparsed(_) => continue,
                    };
                    if tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                }
                // Spec §8.4 case 7: a client falling behind or disconnecting
                // never cancels work. It resubscribes with its last seq.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let _ = tx.send(Ok(Event::default().event("lagged").data(""))).await;
                }
                Err(_) => return,
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
}
```

- [ ] **Step 5: Write `src/protocol/mod.rs`**

```rust
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::agent::{claude::ClaudeHarness, StreamItem};
use crate::command::{fingerprint, CommandContext};
use crate::planner::{LiveHandles, PlannerTurn};
use crate::runtime::Runtime;
use crate::storage::Storage;

pub mod sse;

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<Runtime>,
    pub storage: Arc<Storage>,
    pub handles: Arc<LiveHandles>,
    pub harness: Arc<ClaudeHarness>,
    pub bus: tokio::sync::broadcast::Sender<(String, StreamItem)>,
    pub project_root: std::path::PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/threads", get(list_threads).post(create_thread))
        .route("/api/threads/{id}/entries", get(list_entries))
        .route("/api/threads/{id}/turns", post(start_turn))
        .route("/api/operations/{id}/stop", post(stop_turn))
        .route("/api/subscribe", get(sse::subscribe))
        .with_state(state)
}

/// The whole web client. Spec §1.0: the daemon serves it and never opens it.
async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

#[derive(serde::Deserialize)]
struct CreateProject {
    command_id: String,
    slug: String,
    name: String,
}

fn ctx(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

async fn list_projects(State(s): State<AppState>) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(s.storage.list_projects().await?)))
}

async fn create_project(
    State(s): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Result<Json<serde_json::Value>, Failure> {
    let params = serde_json::json!({ "slug": body.slug, "name": body.name });
    let c = ctx(body.command_id, "project.create", params);
    Ok(Json(serde_json::json!(
        s.storage.create_project(&c, &body.slug, &body.name).await?
    )))
}

async fn list_threads(
    State(s): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_threads_for_project(&project_id).await?
    )))
}

#[derive(serde::Deserialize)]
struct CreateThread {
    command_id: String,
    title: String,
}

async fn create_thread(
    State(s): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateThread>,
) -> Result<Json<serde_json::Value>, Failure> {
    let params = serde_json::json!({ "project": project_id, "title": body.title });
    let c = ctx(body.command_id, "thread.create", params);
    Ok(Json(serde_json::json!(
        s.storage.create_planning_thread(&c, &project_id, &body.title).await?
    )))
}

async fn list_entries(
    State(s): State<AppState>,
    Path(thread_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_thread_entries(&thread_id).await?
    )))
}

#[derive(serde::Deserialize)]
struct StartTurn {
    prompt: String,
    #[serde(default)]
    resume_session_id: Option<String>,
}

/// Spec §3.3: a long-running command returns 202 and an operation id. The
/// operation reaches its terminal outcome later.
async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<StartTurn>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), Failure> {
    // Record the user's message as a durable entry before the turn starts, so
    // a restart mid-turn still shows what was asked.
    s.storage
        .append_thread_entry(
            &thread_id,
            "UserMessage",
            "User",
            "local",
            &body.prompt,
            &[],
        )
        .await?;

    let op = PlannerTurn::start(
        s.runtime.clone(),
        s.handles.clone(),
        s.harness.clone(),
        thread_id,
        body.prompt,
        s.project_root.clone(),
        body.resume_session_id,
        s.bus.clone(),
    )
    .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(serde_json::json!({ "operation_id": op })),
    ))
}

async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    PlannerTurn::stop(s.runtime.clone(), s.handles.clone(), &op_id).await?;
    Ok(Json(serde_json::json!(s.storage.get_operation(&op_id).await?)))
}

/// Transport mapping lives here and nowhere else. Spec §3.3: `Blocked` and
/// `Rejected` are domain outcomes, not HTTP failures, and would be returned as
/// 200 with the outcome — they are not reachable in Milestone 0.
pub struct Failure(crate::storage::StorageError);

impl From<crate::storage::StorageError> for Failure {
    fn from(e: crate::storage::StorageError) -> Self {
        Failure(e)
    }
}

impl axum::response::IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        use crate::error::ErrorCode;
        use crate::storage::StorageError as E;
        let (status, code) = match &self.0 {
            E::CommandConflict => (axum::http::StatusCode::CONFLICT, ErrorCode::CommandConflict),
            E::NotFound(_) => (axum::http::StatusCode::NOT_FOUND, ErrorCode::InvalidCommand),
            E::TransitionConflict { .. } => {
                (axum::http::StatusCode::CONFLICT, ErrorCode::StorageConstraintViolation)
            }
            _ => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::StorageUnavailable,
            ),
        };
        (status, Json(serde_json::json!({ "code": code, "message": self.0.to_string() })))
            .into_response()
    }
}
```

- [ ] **Step 6: Wire it into `src/cli/mod.rs`**

Replace the placeholder router from Task 1:

```rust
use std::sync::Arc;

use crate::agent::claude::ClaudeHarness;
use crate::config::Config;
use crate::planner::LiveHandles;
use crate::protocol::{router, AppState};
use crate::runtime::Runtime;
use crate::storage::{StopKind, Storage};

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let storage = Arc::new(Storage::open(&config.db_path).await?);
    let (runtime, report) = Runtime::start(storage.clone()).await?;
    let runtime = Arc::new(runtime);
    tracing::info!(
        interrupted = report.interrupted.len(),
        anomalies = report.anomalies.len(),
        "startup recovery complete"
    );

    let version = harness_version(&config.harness_path).await;
    let (bus, _) = tokio::sync::broadcast::channel(4096);
    let state = AppState {
        runtime: runtime.clone(),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(config.harness_path.clone(), version)),
        bus,
        project_root: std::env::current_dir()?,
    };

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");

    // Spec §8.5: shutdown reuses the cancellation path. There is no drain mode.
    let shutdown_runtime = runtime.clone();
    let shutdown_state = state.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("stop signal received; cancelling this runtime's operations");
            let ids: Vec<String> = {
                let map = shutdown_state.handles.0.lock().await;
                map.keys().cloned().collect()
            };
            let mut all_confirmed = true;
            for op in ids {
                if crate::planner::PlannerTurn::stop(
                    shutdown_runtime.clone(),
                    shutdown_state.handles.clone(),
                    &op,
                )
                .await
                .is_err()
                {
                    all_confirmed = false;
                }
            }
            let kind = if all_confirmed { StopKind::Graceful } else { StopKind::Escalated };
            let _ = shutdown_runtime.stop(kind).await;
        })
        .await?;
    Ok(())
}

/// Spec §1.4: read the harness's self-reported version and record it. The
/// measured stream contract belongs to one installation at one version.
async fn harness_version(path: &std::path::Path) -> String {
    match tokio::process::Command::new(path).arg("--version").output().await {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}
```

`LiveHandles`'s inner map must be `pub(crate)` for `serve` to enumerate it.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test`
Expected: PASS, the whole suite.

- [ ] **Step 8: Commit**

```bash
git add src/protocol src/storage src/cli src/lib.rs tests/resync.rs
git commit -m "feat(protocol): durable replay with a no-gap handoff to live SSE"
```

---

