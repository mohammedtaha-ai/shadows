//! The person's language-server settings over HTTP: switching a server off,
//! adding one beside the catalog and removing it, each by command.

use serde_json::{Value, json};
#[allow(unused_imports)]
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;

use app::{App, call, fresh_command, shut_down_app, test_app};

/// The project's server listed under `id`, if any.
async fn listed(app: &App, id: &str) -> Option<Value> {
    let path = format!("/api/projects/{}/code/servers", app.project);
    let (status, all) = call(app, "GET", &path, None).await;
    assert_eq!(status, 200, "{all}");
    all.as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .cloned()
}

fn added(command_id: &str, id: &str) -> Value {
    json!({
        "command_id": command_id,
        "id": id,
        "command": "no-such-server-xyz",
        "args": ["--stdio"],
        "extensions": [".zz"],
    })
}

#[tokio::test]
async fn switching_a_server_off_lists_it_off() {
    let app = test_app().await;
    let body = json!({ "command_id": fresh_command(), "off": true });
    let (status, answer) = call(&app, "PUT", "/api/code/servers/gopls/off", Some(body)).await;
    assert_eq!(status, 204, "{answer}");
    assert_eq!(
        listed(&app, "gopls").await.unwrap()["state"]["state"],
        "off"
    );
    let body = json!({ "command_id": fresh_command(), "off": false });
    let (status, _) = call(&app, "PUT", "/api/code/servers/gopls/off", Some(body)).await;
    assert_eq!(status, 204);
    assert_ne!(
        listed(&app, "gopls").await.unwrap()["state"]["state"],
        "off"
    );
    shut_down_app(app).await;
}

#[tokio::test]
async fn an_added_server_is_listed_and_removable() {
    let app = test_app().await;
    let (status, server) = call(&app, "POST", "/api/code/servers", Some(added("a-1", "zz"))).await;
    assert_eq!(status, 200, "{server}");
    assert_eq!(server["added"], true);
    assert_eq!(server["state"]["state"], "not_installed");
    let listed_one = listed(&app, "zz").await.expect("listed");
    assert_eq!(listed_one["extensions"], json!([".zz"]));
    let (status, _) = call(&app, "DELETE", "/api/code/servers/zz?command_id=r-1", None).await;
    assert_eq!(status, 204);
    assert!(listed(&app, "zz").await.is_none());
    let (status, answer) = call(
        &app,
        "DELETE",
        "/api/code/servers/gopls?command_id=r-2",
        None,
    )
    .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (422, Some("INVALID_COMMAND"))
    );
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_catalog_id_cannot_be_added() {
    let app = test_app().await;
    let (status, answer) = call(
        &app,
        "POST",
        "/api/code/servers",
        Some(added("a-2", "gopls")),
    )
    .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (422, Some("INVALID_COMMAND"))
    );
    let mut no_dot = added("a-3", "yy");
    no_dot["extensions"] = json!(["yy"]);
    let (status, _) = call(&app, "POST", "/api/code/servers", Some(no_dot)).await;
    assert_eq!(status, 422);
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_replayed_command_changes_nothing() {
    let app = test_app().await;
    let (first, _) = call(&app, "POST", "/api/code/servers", Some(added("a-4", "zz"))).await;
    let (again, _) = call(&app, "POST", "/api/code/servers", Some(added("a-4", "zz"))).await;
    assert_eq!((first, again), (200, 200));
    let mut other = added("a-4", "zz");
    other["command"] = json!("another");
    let (status, answer) = call(&app, "POST", "/api/code/servers", Some(other)).await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    let all = call(
        &app,
        "GET",
        &format!("/api/projects/{}/code/servers", app.project),
        None,
    )
    .await
    .1;
    let count = all
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["id"] == "zz")
        .count();
    assert_eq!(count, 1);
    shut_down_app(app).await;
}
