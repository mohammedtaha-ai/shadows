//! What a code question may read: the asker's project and
//! the projects it links to, one way, over MCP and over HTTP alike.

use std::time::Duration;

use serde_json::{Value, json};
use shadows_agent::policy;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::acp;
use shadows_core::{CodebaseConfig, IndexState, ProjectId};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, fresh_command};
use listening::{Listening, listening_app, ok, project_client, refused, thread_client};

/// The status `shadows-http/src/failure.rs` gives a service's own refusal.
const REFUSED: u16 = 422;

/// A project whose folder is its own temp dir, holding one file. The dir is
/// returned so it lives as long as the test.
async fn other_project_with_folder(
    l: &Listening,
    slug: &str,
    file: &str,
    text: &str,
) -> (ProjectId, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    let project = shadows_core::testing::create_project(
        &l.app.storage,
        &ctx(&fresh_command(), "project.create"),
        slug,
        slug,
        &ProjectDirectory::resolve(dir.path()).unwrap(),
        &policy::default_modes(),
    )
    .await
    .unwrap()
    .id;
    (project, dir)
}

async fn put_json(l: &Listening, path: &str, body: Value) -> (u16, Value) {
    app::call(&l.app, "PUT", path, Some(body)).await
}

async fn get(l: &Listening, path: &str) -> (u16, Value) {
    app::call(&l.app, "GET", path, None).await
}

/// Touches each project, then waits until every one's index is `Ready`.
async fn settle_index(l: &Listening, projects: &[&ProjectId]) {
    let code = l.app.core.codebase();
    for p in projects {
        code.touch(p).await;
    }
    for p in projects {
        let mut state = None;
        for _ in 0..400 {
            let status = code.status(p).await.unwrap();
            if status.state == IndexState::Ready {
                state = None;
                break;
            }
            state = Some(status.state);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(state.is_none(), "{p} never became ready: {state:?}");
    }
}

#[tokio::test]
async fn the_scope_is_the_project_and_its_links_only() {
    let l = listening_app().await;
    let a = l.app.project.clone();
    let (b, _dir_b) =
        other_project_with_folder(&l, "backend", "src/api.rs", "pub fn orders() {}\n").await;
    let (c, _dir_c) =
        other_project_with_folder(&l, "secret", "src/k.rs", "pub fn key() {}\n").await;
    let (_, client) = project_client(&l).await;
    // `assemble` does not start the workers.
    l.app
        .core
        .codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();

    // Link A → B through the route; C stays unlinked.
    let path = format!("/api/projects/{a}/code/links/{b}");
    let (status, body) = put_json(&l, &path, json!({ "command_id": fresh_command() })).await;
    assert_eq!(status, 200, "{body}");
    settle_index(&l, &[&a, &b, &c]).await;

    // Unlinked: GRANT_SCOPE over MCP, before any server is asked.
    let args = json!({ "operation": "documentSymbol", "file": "src/k.rs", "project": "secret" });
    let refused_c = refused(&client, "code_lsp", args).await;
    assert!(refused_c.contains("GRANT_SCOPE"), "{refused_c}");

    // A path out of the folder is refused.
    let args = json!({ "operation": "documentSymbol", "file": "../x.rs" });
    let out = refused(&client, "code_lsp", args).await;
    assert!(out.contains("the path must be inside the project"), "{out}");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_person_and_the_planner_see_the_same_files() {
    let l = listening_app().await;
    let a = l.app.project.clone();
    let project = l.app.core.projects().get(&a).await.unwrap();
    let root = std::path::PathBuf::from(project.directory.unwrap());
    std::fs::create_dir_all(root.join("app/Models")).unwrap();
    std::fs::write(root.join("app/Models/User.php"), "<?php class User {}").unwrap();
    std::fs::write(root.join("app/.env"), "APP_KEY=SECRET_CANARY").unwrap();
    let (status, first) = app::call(
        &l.app,
        "POST",
        &format!("/api/projects/{a}/code/refresh"),
        None,
    )
    .await;
    assert_eq!(
        (status, first["state"].as_str()),
        (200, Some("ready")),
        "{first}"
    );
    let path = format!("/api/projects/{a}/code/files?prefix=app&limit=2");
    let (status, page) = get(&l, &path).await;
    assert_eq!(status, 200, "{page}");
    assert_eq!(page["entries"][0]["path"], "app");
    assert_eq!(page["entries"][1]["excluded_reason"], "secret");
    assert!(page["next"].is_string(), "{page}");
    assert!(!page.to_string().contains("SECRET_CANARY"));
    let planner = thread_client(&l, &l.app.thread).await;
    let args = json!({ "prefix": "app", "limit": 2 });
    assert_eq!(ok(&planner, "project_files", args).await, page);
    let next = page["next"].as_str().unwrap();
    let args = json!({ "prefix": "app", "limit": 2, "after": next });
    let rest = ok(&planner, "project_files", args).await;
    assert_eq!(rest["entries"][1]["path"], "app/Models/User.php");
    assert_eq!(
        ok(&planner, "project_refresh", json!({})).await["state"],
        "ready"
    );
    // A cursor of another generation: 409 over HTTP, the same code over MCP.
    let (status, body) = get(&l, &format!("/api/projects/{a}/code/files?after=0:app")).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (409, Some("INVENTORY_STALE"))
    );
    let stale = refused(&planner, "project_files", json!({ "after": "0:app" })).await;
    assert!(stale.starts_with("INVENTORY_STALE"), "{stale}");
    let (status, body) = get(&l, &format!("/api/projects/{a}/code/files?limit=500")).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (REFUSED, Some("INVALID_COMMAND"))
    );
    let (status, _) = get(&l, "/api/projects/no-such-project/code/files").await;
    assert_eq!(status, 404);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[test]
fn the_planner_is_told_to_look_at_the_real_files() {
    let prompt = shadows_core::testing::PROMPT;
    assert!(prompt.contains("- project_files: list the files the project's folder really holds"));
    assert!(prompt.contains("Look before you write a path"));
    assert!(prompt.contains("Never write a framework's layout from memory"));
}
