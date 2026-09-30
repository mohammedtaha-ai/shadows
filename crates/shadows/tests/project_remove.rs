//! Removing a project (spec §4.2, §15.4): refused while it holds a planning
//! thread; otherwise it is listed nowhere again, its index, its links both
//! ways and its MCP grants go, and every later request about it is NotFound.

use serde_json::{Value, json};
use shadows_agent::policy;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::acp;
use shadows_core::{CodeConfig, ProjectId};

#[path = "fixtures/app.rs"]
mod app;

use app::{App, call, ctx, fresh_command, test_app};

/// Writes one Rust file defining `name` into `dir` and indexes `project`.
async fn index_file(app: &App, project: &ProjectId, dir: &std::path::Path, name: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), format!("pub fn {name}() {{}}\n")).unwrap();
    app.core.code().scan_for_test(project).await.unwrap();
}

/// A project with no thread, whose folder holds `pub fn <slug>_fn`, indexed.
async fn indexed_project(app: &App, slug: &str) -> (ProjectId, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let project = app
        .storage
        .create_project(
            &ctx(&fresh_command(), "project.create"),
            slug,
            slug,
            &ProjectDirectory::resolve(dir.path()).unwrap(),
            &policy::default_modes(),
        )
        .await
        .unwrap()
        .id;
    index_file(app, &project, dir.path(), &format!("{slug}_fn")).await;
    (project, dir)
}

async fn link(app: &App, project: &ProjectId, linked: &ProjectId) {
    let code = app.core.code();
    code.link(fresh_command(), project, linked).await.unwrap();
}

async fn remove(app: &App, project: &ProjectId, command_id: &str) -> (u16, Value) {
    let path = format!("/api/projects/{project}?command_id={command_id}");
    call(app, "DELETE", &path, None).await
}

async fn slugs(app: &App) -> Vec<String> {
    let (status, body) = call(app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200, "{body}");
    let list = body.as_array().unwrap();
    list.iter()
        .map(|p| p["slug"].as_str().unwrap().to_string())
        .collect()
}

/// Rows of `table` whose `column` (or, for links, either end) is `project`.
async fn rows(app: &App, sql: &str, project: &ProjectId) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_string()))
        .bind(project.as_str())
        .bind(project.as_str())
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

const FILES: &str = "SELECT COUNT(*) FROM code_file WHERE project_id = ? OR project_id = ?";
const TAGS: &str = "SELECT COUNT(*) FROM code_tag WHERE project_id = ? OR project_id = ?";
const LINKS: &str =
    "SELECT COUNT(*) FROM project_link WHERE project_id = ? OR linked_project_id = ?";

#[tokio::test]
async fn a_project_with_a_thread_is_not_removed() {
    let app = test_app().await;
    // The app's project holds the fixture's thread.
    let demo = app.project.clone();
    let dir = app.storage.get_project(&demo).await.unwrap().directory;
    index_file(&app, &demo, std::path::Path::new(&dir.unwrap()), "demo_fn").await;
    let (other, _dir) = indexed_project(&app, "other").await;
    link(&app, &demo, &other).await;
    let (files, links) = (
        rows(&app, FILES, &demo).await,
        rows(&app, LINKS, &demo).await,
    );
    assert!(files > 0);

    let (status, body) = remove(&app, &demo, &fresh_command()).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (409, Some("PROJECT_HAS_THREADS")),
        "{body}"
    );
    assert!(slugs(&app).await.contains(&"demo".to_string()));
    assert_eq!(rows(&app, FILES, &demo).await, files);
    assert_eq!(rows(&app, LINKS, &demo).await, links);
}

#[tokio::test]
async fn a_project_without_threads_is_removed_with_its_index_links_and_grants() {
    let app = test_app().await;
    let (gone, _d1) = indexed_project(&app, "gone").await;
    let (other, _d2) = indexed_project(&app, "other").await;
    link(&app, &gone, &other).await;
    link(&app, &other, &gone).await;
    link(&app, &app.project, &gone).await;
    let issued = app
        .core
        .grants()
        .issue(fresh_command(), &gone)
        .await
        .unwrap();
    let token = issued.token.unwrap();
    assert!(app.core.grants().authorize(&token).await.unwrap().is_some());
    app.core.code().start(CodeConfig::default()).await.unwrap();
    app.core.code().touch(&gone).await;
    assert!(app.core.code().active_for_test().await.contains(&gone));

    let (status, body) = remove(&app, &gone, &fresh_command()).await;
    assert_eq!((status, body["id"].as_str()), (200, Some(gone.as_str())));

    assert_eq!(slugs(&app).await, ["demo", "other"]);
    assert_eq!(rows(&app, FILES, &gone).await, 0);
    assert_eq!(rows(&app, TAGS, &gone).await, 0);
    assert_eq!(rows(&app, LINKS, &gone).await, 0);
    assert_eq!(rows(&app, FILES, &other).await, 1, "another index stays");
    assert!(app.core.grants().authorize(&token).await.unwrap().is_none());
    assert!(!app.core.code().active_for_test().await.contains(&gone));
    app.core.code().shut_down_for_test().await;
}

#[tokio::test]
async fn a_replayed_remove_answers_the_same_project_and_changes_nothing() {
    let app = test_app().await;
    let (gone, _dir) = indexed_project(&app, "gone").await;
    let command = fresh_command();
    let (status, first) = remove(&app, &gone, &command).await;
    assert_eq!(status, 200, "{first}");
    let events = "SELECT COUNT(*) FROM durable_event WHERE project_id = ? OR project_id = ?";
    let before = rows(&app, events, &gone).await;

    let (status, again) = remove(&app, &gone, &command).await;
    assert_eq!((status, &again), (200, &first));
    assert_eq!(rows(&app, events, &gone).await, before);

    // Another request under the same id, in the same project's scope.
    let body = json!({ "command_id": command, "allowed_modes": {} });
    let (status, body) = call(&app, "PATCH", &format!("/api/projects/{gone}"), Some(body)).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
}

#[tokio::test]
async fn a_removed_project_is_not_found() {
    let app = test_app().await;
    let (gone, _dir) = indexed_project(&app, "gone").await;
    let core = &app.core;
    core.instructions()
        .save(fresh_command(), &gone, "be brief")
        .await
        .unwrap();
    core.grants().issue(fresh_command(), &gone).await.unwrap();
    let (status, body) = remove(&app, &gone, &fresh_command()).await;
    assert_eq!(status, 200, "{body}");

    // Its reads answer as for an unknown project: nothing it held shows.
    for (read, empty) in [
        ("planner-instructions", json!(null)),
        ("mcp-grants", json!([])),
    ] {
        let (status, body) = call(&app, "GET", &format!("/api/projects/{gone}/{read}"), None).await;
        assert_eq!((status, &body), (200, &empty), "{read}");
    }

    let thread = json!({ "command_id": fresh_command(), "title": "T" });
    let path = format!("/api/projects/{gone}/threads");
    let (status, body) = call(&app, "POST", &path, Some(thread)).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (404, Some("INVALID_COMMAND"))
    );

    let path = format!("/api/projects/{gone}/code/definitions?name=gone_fn");
    let (status, body) = call(&app, "GET", &path, None).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (404, Some("INVALID_COMMAND"))
    );
}
