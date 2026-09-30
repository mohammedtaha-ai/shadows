//! What a code question may read (spec §15.5–§15.7): the asker's project and
//! the projects it links to, one way, over MCP and over HTTP alike.

use std::time::Duration;

use serde_json::{Value, json};
use shadows_agent::policy;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::acp;
use shadows_core::{CodeConfig, IndexState, ProjectId};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, fresh_command};
use listening::{Client, Listening, call, listening_app, project_client, refused};

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
    let project = l
        .app
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
    (project, dir)
}

async fn put_json(l: &Listening, path: &str, body: Value) -> (u16, Value) {
    app::call(&l.app, "PUT", path, Some(body)).await
}

async fn get(l: &Listening, path: &str) -> (u16, Value) {
    app::call(&l.app, "GET", path, None).await
}

/// The text a tool call answered, asserting it succeeded.
async fn text(client: &Client, tool: &str, args: Value) -> String {
    let (is_error, text) = call(client, tool, args).await;
    assert!(!is_error, "{tool} failed: {text}");
    text
}

/// Touches each project, then waits until every one's index is `Ready`.
async fn settle_index(l: &Listening, projects: &[&ProjectId]) {
    let code = l.app.core.code();
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
        .code()
        .start(CodeConfig::default())
        .await
        .unwrap();

    // Link A → B through the route; C stays unlinked.
    let path = format!("/api/projects/{a}/code/links/{b}");
    let (status, body) = put_json(&l, &path, json!({ "command_id": fresh_command() })).await;
    assert_eq!(status, 200, "{body}");
    settle_index(&l, &[&a, &b, &c]).await;

    let found = text(&client, "where_is", json!({ "name": "orders" })).await;
    assert!(
        found.contains("backend: src/api.rs:1 function orders"),
        "{found}"
    );
    let args = json!({ "name": "orders", "project": "backend" });
    let only_b = text(&client, "where_is", args).await;
    assert!(only_b.contains("src/api.rs:1"), "{only_b}");

    // Unlinked: GRANT_SCOPE over MCP, INVALID_COMMAND over HTTP, and never its content.
    let args = json!({ "name": "key", "project": "secret" });
    let refused_c = refused(&client, "where_is", args).await;
    assert!(refused_c.contains("GRANT_SCOPE"), "{refused_c}");
    let all = text(&client, "where_is", json!({ "name": "key" })).await;
    assert!(!all.contains("secret"), "{all}");
    let path = format!("/api/projects/{a}/code/definitions?name=key&project=secret");
    let (status, body) = get(&l, &path).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (REFUSED, Some("INVALID_COMMAND"))
    );

    // A path out of the folder is refused.
    let out = refused(&client, "outline", json!({ "path": "../" })).await;
    assert!(out.contains("the path must be inside the project"), "{out}");

    // One way: B does not see A.
    let path = format!("/api/projects/{b}/code/definitions?name=orders&project=demo");
    let (status, body) = get(&l, &path).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (REFUSED, Some("INVALID_COMMAND"))
    );
}
