//! Shared apparatus: a whole daemon in-process — storage, runtime, sessions on
//! `fake_acp`, and the real router driven with `oneshot` — with one project and
//! one thread already created. Include beside `acp.rs`:
//!
//! ```ignore
//! #[path = "fixtures/acp.rs"] mod acp;
//! #[path = "fixtures/app.rs"] mod app;
//! ```

#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use shadows_agent::events::HarnessEvent;
use shadows_agent::policy;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::events::UiSignal;
use shadows_core::operation::{Operation, OperationId};
use shadows_core::planner::{LiveHandles, Sessions, SessionsConfig};
use shadows_core::project::{ProjectDirectory, ProjectId};
use shadows_core::runtime::Runtime;
use shadows_core::storage::Storage;
use shadows_core::thread::{ThreadEntry, ThreadEntryKind, ThreadId};
use shadows_http::AppState;
use tower::ServiceExt;

use super::acp;

#[path = "daemon.rs"]
pub mod daemon;
use daemon::router;

pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

pub struct App {
    /// `None` when the test owns the directory (`test_app_at`).
    _tmp: Option<tempfile::TempDir>,
    pub runtime: Arc<Runtime>,
    pub storage: Arc<Storage>,
    pub handles: Arc<LiveHandles>,
    pub sessions: Arc<Sessions>,
    pub bus: Bus,
    /// The live-only signals `plan_show` sends (§13.9).
    pub ui: tokio::sync::broadcast::Sender<UiSignal>,
    pub router: Router,
    pub project: ProjectId,
    pub thread: ThreadId,
    // Held: a dropped sender reads as a stopping daemon.
    _stopping: tokio::sync::watch::Sender<bool>,
}

pub fn ctx(id: &str, kind: &str) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &json!({ "id": id })),
    }
}

pub async fn test_app() -> App {
    let tmp = tempfile::tempdir().unwrap();
    test_app_at(tmp.path()).await.owning(tmp)
}

impl App {
    /// The app, keeping `tmp` (its directory) until it is dropped.
    pub fn owning(mut self, tmp: tempfile::TempDir) -> Self {
        self._tmp = Some(tmp);
        self
    }
}

/// A daemon on the database in `dir`. Called twice on one directory, the
/// second is the first one restarted: the project and thread are found again,
/// not created twice.
pub async fn test_app_at(dir: &Path) -> App {
    test_app_with(dir, acp::test_config(), acp::MCP_URL).await
}

/// As `test_app_at`, with the sessions' own configuration and the `/mcp` URL
/// the daemon reports.
pub async fn test_app_with(dir: &Path, config: SessionsConfig, mcp_url: &str) -> App {
    let db = dir.join("s.sqlite3");
    let storage = Arc::new(Storage::open(&db).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let runtime = Arc::new(runtime);
    let project = storage
        .create_project(
            &ctx("c1", "project.create"),
            "demo",
            "Demo",
            &ProjectDirectory::resolve(dir).unwrap(),
            &policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx("c2", "thread.create"),
            &project.id,
            "T",
            policy::CLAUDE_CODE,
        )
        .await
        .unwrap()
        .id;
    let sessions = Sessions::new(
        acp::fake_adapter(),
        Storage::open(&db).await.unwrap(),
        config,
    );
    let handles = Arc::new(LiveHandles::default());
    let (bus, _) = tokio::sync::broadcast::channel(256);
    let (ui, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let router = router(AppState {
        runtime: runtime.clone(),
        storage: storage.clone(),
        handles: handles.clone(),
        sessions: sessions.clone(),
        bus: bus.clone(),
        ui: ui.clone(),
        allowed_origins: Vec::new(),
        mcp_url: mcp_url.to_string(),
        shutdown,
    });
    App {
        _tmp: None,
        runtime,
        storage,
        handles,
        sessions,
        bus,
        ui,
        router,
        project: project.id,
        thread,
        _stopping: stopping,
    }
}

/// Stops the daemon as `serve` does on a signal, so the directory can be
/// opened again by `test_app_at`.
pub async fn shut_down_app(app: App) {
    shadows_core::planner::shut_down(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        Duration::from_secs(5),
        std::future::pending::<()>(),
    )
    .await
    .unwrap();
}

pub async fn call(app: &App, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let request = Request::builder().method(method).uri(path);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

pub async fn post(app: &App, path: &str, body: Value) -> (u16, Value) {
    call(app, "POST", path, Some(body)).await
}

pub async fn patch(app: &App, path: &str, body: Value) -> (u16, Value) {
    call(app, "PATCH", path, Some(body)).await
}

pub async fn get_json<T: DeserializeOwned>(app: &App, path: &str) -> T {
    let (status, body) = call(app, "GET", path, None).await;
    assert_eq!(status, 200, "GET {path}: {body}");
    serde_json::from_value(body).unwrap()
}

/// A second project, on the system's temp directory, with one thread of its
/// own: what a test needs to show something stays inside its own project.
pub async fn other_project(app: &App) -> (ProjectId, ThreadId) {
    let project = app
        .storage
        .create_project(
            &ctx("other-project", "project.create"),
            "other",
            "Other",
            &ProjectDirectory::resolve(&std::env::temp_dir()).unwrap(),
            &policy::default_modes(),
        )
        .await
        .unwrap()
        .id;
    let thread = app
        .storage
        .create_planning_thread(
            &ctx("other-thread", "thread.create"),
            &project,
            "Other",
            policy::CLAUDE_CODE,
        )
        .await
        .unwrap()
        .id;
    (project, thread)
}

/// Creates a thread in the app's project over HTTP; answers its JSON.
pub async fn create_thread(app: &App, body: Value) -> Value {
    let (status, thread) = post(app, &format!("/api/projects/{}/threads", app.project), body).await;
    assert_eq!(status, 200, "{thread}");
    thread
}

pub fn names(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

/// The fake's own default settings, which every turn can run with.
pub fn default_settings() -> Value {
    json!({ "model": "fake-large", "mode": "acceptEdits", "effort": "high" })
}

static NEXT_COMMAND: AtomicUsize = AtomicUsize::new(1);

/// A command id no other request in this test binary has used.
pub fn fresh_command() -> String {
    format!("cmd-{}", NEXT_COMMAND.fetch_add(1, Ordering::SeqCst))
}

/// `POST /api/threads/{thread}/turns` with `body` as given.
pub async fn http_start(app: &App, thread: &str, body: Value) -> (u16, Value) {
    post(app, &format!("/api/threads/{thread}/turns"), body).await
}

/// Starts `prompt` on `thread` with `settings` and a fresh command id;
/// answers the operation, asserting it was accepted.
pub async fn start_on(app: &App, thread: &str, prompt: &str, settings: Value) -> OperationId {
    let mut body = settings;
    body["command_id"] = json!(fresh_command());
    body["prompt"] = json!(prompt);
    let (status, answer) = http_start(app, thread, body).await;
    assert_eq!(status, 202, "{answer}");
    OperationId::from_literal(answer["operation_id"].as_str().unwrap())
}

/// Starts `prompt` on the app's thread with the fake's default settings.
pub async fn start_settled(app: &App, prompt: &str) -> OperationId {
    start_on(app, app.thread.as_str(), prompt, default_settings()).await
}

pub async fn start_and_finish_on(
    app: &App,
    thread: &str,
    prompt: &str,
    settings: Value,
) -> Operation {
    let op = start_on(app, thread, prompt, settings).await;
    wait_terminal(app, &op).await
}

pub async fn start_and_finish(app: &App, prompt: &str, settings: Value) -> Operation {
    start_and_finish_on(app, app.thread.as_str(), prompt, settings).await
}

pub async fn wait_terminal(app: &App, op: &OperationId) -> Operation {
    for _ in 0..200 {
        let loaded = app.storage.get_operation(op).await.unwrap();
        if loaded.finished_at.is_some() {
            return loaded;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach a terminal state in time");
}

pub async fn entries_on(app: &App, thread: &ThreadId) -> Vec<ThreadEntry> {
    app.storage.list_thread_entries(thread).await.unwrap()
}

pub async fn entries(app: &App) -> Vec<ThreadEntry> {
    entries_on(app, &app.thread).await
}

pub async fn last_agent_entry_on(app: &App, thread: &str) -> ThreadEntry {
    entries_on(app, &ThreadId::from_literal(thread))
        .await
        .into_iter()
        .rev()
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("an agent reply")
}

pub async fn last_agent_entry(app: &App) -> ThreadEntry {
    last_agent_entry_on(app, app.thread.as_str()).await
}

/// A live `/api/subscribe` stream, read frame by frame.
pub struct Subscription {
    body: axum::body::BodyDataStream,
    buffer: String,
}

/// Subscribes to `thread` from its start and waits until the replay is over,
/// so what the next frames carry happened after this call.
pub async fn subscribe(app: &App, thread: &ThreadId) -> Subscription {
    let mut sub = subscribe_from(app, thread, 0).await;
    next_frame_named(&mut sub, "caught-up").await;
    sub
}

/// Subscribes to `thread` after `after`, reading nothing yet: the replay's
/// frames are still to come.
pub async fn subscribe_from(app: &App, thread: &ThreadId, after: i64) -> Subscription {
    let response = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("/api/subscribe?thread_id={thread}&after={after}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    Subscription {
        body: response.into_body().into_data_stream(),
        buffer: String::new(),
    }
}

/// Every frame as `(event, data)`, up to and including the first for which
/// `last` holds.
pub async fn frames_until(
    sub: &mut Subscription,
    last: impl Fn(&str, &Value) -> bool,
) -> Vec<(String, Value)> {
    let mut frames = Vec::new();
    loop {
        let (event, data) = next_frame(sub).await;
        let done = last(&event, &data);
        frames.push((event, data));
        if done {
            return frames;
        }
    }
}

/// The next frame's `event:` and data, whatever it is.
async fn next_frame(sub: &mut Subscription) -> (String, Value) {
    use tokio_stream::StreamExt;
    loop {
        if let Some(end) = sub.buffer.find("\n\n") {
            let frame: String = sub.buffer.drain(..end + 2).collect();
            let event = frame.lines().find_map(|l| l.strip_prefix("event: "));
            let data = frame.lines().find_map(|l| l.strip_prefix("data: "));
            let data = serde_json::from_str(data.unwrap_or("null")).unwrap_or(Value::Null);
            return (event.unwrap_or_default().to_string(), data);
        }
        let chunk = tokio::time::timeout(Duration::from_secs(10), sub.body.next())
            .await
            .unwrap_or_else(|_| panic!("no frame in time: {}", sub.buffer))
            .expect("the stream ended")
            .unwrap();
        sub.buffer.push_str(std::str::from_utf8(&chunk).unwrap());
    }
}

/// The data of the next frame whose `event:` is `name`, skipping others.
pub async fn next_frame_named(sub: &mut Subscription, name: &str) -> Value {
    loop {
        let (event, data) = next_frame(sub).await;
        if event == name {
            return data;
        }
    }
}
