//! A project owns a directory (spec §4.2, §6.3, §8.3): it is checked at the
//! HTTP boundary, stored canonical, and it is where every turn on the
//! project's threads runs. A project from before projects owned one fails its
//! turns at Prepare instead of running in the daemon's working directory.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use daemon::router;
use serde_json::{Value, json};
use shadows_core::operation::{Operation, OperationId};
use shadows_core::planner::LiveHandles;
use shadows_core::runtime::Runtime;
use shadows_core::storage::Storage;
use shadows_http::AppState;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/daemon.rs"]
mod daemon;

async fn app(storage: Arc<Storage>, db: &Path) -> (Router, tokio::sync::watch::Sender<bool>) {
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        runtime: Arc::new(runtime),
        storage,
        handles: Arc::new(LiveHandles::default()),
        sessions: acp::fake_sessions(db).await,
        bus,
        allowed_origins: Vec::new(),
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
        shutdown,
    });
    (app, stopping)
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(b) => request
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn create_project(app: &Router, command_id: &str, directory: &str) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/api/projects",
        Some(json!({
            "command_id": command_id, "slug": command_id, "name": "Demo", "directory": directory,
        })),
    )
    .await
}

async fn wait_for_terminal(storage: &Storage, op: &str) -> Operation {
    let op = OperationId::from_literal(op);
    for _ in 0..200 {
        let loaded = storage.get_operation(&op).await.unwrap();
        if loaded.finished_at.is_some() {
            return loaded;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation did not reach a terminal state in time");
}

/// The directory is resolved to its canonical form, returned with the
/// project, and is the working directory of a turn on that project's thread
/// — which `fake_acp` reports from inside the child.
#[tokio::test]
async fn a_turn_runs_in_its_projects_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (app, _stopping) = app(storage.clone(), &tmp.path().join("s.sqlite3")).await;
    let work = tmp.path().join("work");
    std::fs::create_dir(&work).unwrap();

    // A spelling with a redundant `.` component, which canonicalisation removes.
    let spelled = work.join(".").display().to_string();
    let (status, project) = create_project(&app, "c1", &spelled).await;
    assert_eq!(status, StatusCode::OK, "{project}");
    let canonical = dunce::canonicalize(&work).unwrap().display().to_string();
    assert_eq!(project["directory"], canonical.as_str(), "{project}");

    let (_, listed) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(listed[0]["directory"], canonical.as_str(), "{listed}");

    let project_id = project["id"].as_str().unwrap();
    let (_, thread) = call(
        &app,
        "POST",
        &format!("/api/projects/{project_id}/threads"),
        Some(json!({ "command_id": "c2", "title": "T" })),
    )
    .await;
    let thread_id = thread["id"].as_str().unwrap();
    let (status, started) = call(
        &app,
        "POST",
        &format!("/api/threads/{thread_id}/turns"),
        Some(json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "report", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{started}");
    let op = wait_for_terminal(&storage, started["operation_id"].as_str().unwrap()).await;
    assert_eq!(op.status_kind, "Completed", "{op:?}");

    let (_, entries) = call(
        &app,
        "GET",
        &format!("/api/threads/{thread_id}/entries"),
        None,
    )
    .await;
    let reply = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "AgentMessage")
        .unwrap_or_else(|| panic!("no agent reply in {entries}"));
    let reported: Value = serde_json::from_str(reply["body"].as_str().unwrap()).unwrap();
    assert_eq!(reported["cwd"], canonical.as_str(), "{reported}");
}

/// Spec §3.4: a path the daemon cannot use is refused with a 4xx and a code
/// a client can match on — never a 500 — and nothing is created.
#[tokio::test]
async fn an_unusable_directory_is_refused_with_a_stable_code() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (app, _stopping) = app(storage, &tmp.path().join("s.sqlite3")).await;
    let file = tmp.path().join("a-file.txt");
    std::fs::write(&file, "not a directory").unwrap();

    let cases = [
        (
            "relative".to_string(),
            StatusCode::BAD_REQUEST,
            "PATH_INVALID",
        ),
        (
            tmp.path().join("missing").display().to_string(),
            StatusCode::NOT_FOUND,
            "PATH_NOT_FOUND",
        ),
        (
            file.display().to_string(),
            StatusCode::BAD_REQUEST,
            "PATH_NOT_A_DIRECTORY",
        ),
    ];
    for (i, (path, status, code)) in cases.iter().enumerate() {
        let (got, body) = create_project(&app, &format!("c{i}"), path).await;
        assert_eq!(
            (got, body["code"].as_str()),
            (*status, Some(*code)),
            "{path}: {body}"
        );
    }

    let (_, listed) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(listed, json!([]), "a refused request created a project");
}

/// A project whose directory was deleted after it was created: the turn is
/// refused with the reason, naming the directory, and nothing is written.
#[tokio::test]
async fn a_turn_on_a_deleted_directory_is_refused_with_its_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("s.sqlite3");
    let storage = Arc::new(Storage::open(&db).await.unwrap());
    let (app, _stopping) = app(storage.clone(), &db).await;
    let work = tmp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let (_, project) = create_project(&app, "c1", &work.display().to_string()).await;
    let (_, thread) = call(
        &app,
        "POST",
        &format!("/api/projects/{}/threads", project["id"].as_str().unwrap()),
        Some(json!({ "command_id": "c2", "title": "T" })),
    )
    .await;
    let thread_id = thread["id"].as_str().unwrap();
    std::fs::remove_dir_all(&work).unwrap();

    let (status, refused) = call(
        &app,
        "POST",
        &format!("/api/threads/{thread_id}/turns"),
        Some(json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "hi", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(refused["code"], "PATH_NOT_FOUND", "{refused}");
    let message = refused["message"].as_str().unwrap();
    assert!(
        message.contains("missing") && message.contains("work"),
        "{refused}"
    );
    let (_, entries) = call(
        &app,
        "GET",
        &format!("/api/threads/{thread_id}/entries"),
        None,
    )
    .await;
    assert_eq!(entries, json!([]), "a refused turn wrote its prompt");
}

/// A database written before `0003_project_directory.sql` holds projects with
/// no directory. The migration cannot invent one, so the row keeps NULL, the
/// API says so, and a turn is refused (no session can open there) rather than
/// running in whatever directory the daemon was started in. New rows cannot repeat it.
#[tokio::test]
async fn a_project_from_before_directories_has_its_turns_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("s.sqlite3");
    seed_pre_directory_database(tmp.path(), &db).await;

    let storage = Arc::new(Storage::open(&db).await.unwrap());
    let (app, _stopping) = app(storage.clone(), &db).await;

    let (_, listed) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(listed[0]["directory"], Value::Null, "{listed}");

    let (status, started) = call(
        &app,
        "POST",
        &format!("/api/threads/{LEGACY_THREAD}/turns"),
        Some(json!({ "command_id": uuid::Uuid::new_v4().to_string(), "prompt": "report", "model": "fake-large", "mode": "acceptEdits", "effort": "high" })),
    )
    .await;
    // Spec §12.7: the session is opened before any write, and a directory
    // that cannot be run in is a failure to open it — nothing durable.
    assert_eq!(status, StatusCode::CONFLICT, "{started}");
    assert_eq!(started["code"], "PATH_NOT_FOUND", "{started}");
    assert!(
        started["message"]
            .as_str()
            .is_some_and(|m| m.contains("no directory")),
        "the client is told why: {started}"
    );
    let (_, operations) = call(
        &app,
        "GET",
        &format!("/api/threads/{LEGACY_THREAD}/operations"),
        None,
    )
    .await;
    assert_eq!(operations, json!([]), "a refused turn wrote an operation");
    let (_, entries) = call(
        &app,
        "GET",
        &format!("/api/threads/{LEGACY_THREAD}/entries"),
        None,
    )
    .await;
    assert_eq!(entries, json!([]), "a refused turn wrote its prompt");

    let refused = sqlx::query(
        "INSERT INTO project (id, slug, name, created_at) VALUES ('p2', 'new', 'New', 'now')",
    )
    .execute(storage.reader())
    .await;
    assert!(
        refused.is_err(),
        "a new project row was written without a directory"
    );
}

const LEGACY_THREAD: &str = "00000000-0000-4000-8000-0000000000a2";

/// Applies only the migrations that predate project directories — from copies
/// of the checked-in files, so their checksums match what `Storage::open`
/// then finds recorded — and writes a project and thread the way that schema
/// allowed.
async fn seed_pre_directory_database(tmp: &Path, db: &Path) {
    let old = tmp.join("pre-0003");
    std::fs::create_dir(&old).unwrap();
    let migrations = shadows_core::testing::migrations_dir();
    for name in ["0001_milestone0.sql", "0002_operation_event_thread.sql"] {
        std::fs::copy(migrations.join(name), old.join(name)).unwrap();
    }
    let url = format!(
        "sqlite://{}?mode=rwc",
        db.display().to_string().replace('\\', "/")
    );
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::migrate::Migrator::new(old.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO project (id, slug, name, created_at)
         VALUES ('00000000-0000-4000-8000-0000000000a1', 'legacy', 'Legacy', '2026-09-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO planning_thread (id, project_id, title, status, created_at)
         VALUES (?, '00000000-0000-4000-8000-0000000000a1', 'Old', 'Open', '2026-09-01T00:00:00Z')",
    )
    .bind(LEGACY_THREAD)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
}
