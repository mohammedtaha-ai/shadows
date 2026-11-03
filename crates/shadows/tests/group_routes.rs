//! Agent group HTTP operations, delivery recovery and durable frames.
use axum::body::Body;
use axum::http::Request;
use serde_json::{Value, json};
use shadows_core::{
    AylulConfig,
    testing::{SessionsConfig, acp},
};
use std::{path::Path, time::Duration};
use tokio_stream::StreamExt;
use tower::ServiceExt;

#[path = "fixtures/app.rs"]
mod app;

async fn world(dir: &Path) -> app::App {
    let root = dir.join("home");
    std::fs::create_dir_all(&root).unwrap();
    let app = app::test_app_unseeded_with(
        dir,
        SessionsConfig {
            aylul: Some(AylulConfig {
                home_root: root.clone(),
                project_slug: "aylul".into(),
            }),
            ..acp::test_config()
        },
        acp::MCP_URL,
    )
    .await;
    app.core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &root)
        .await
        .unwrap();
    app.core
        .providers()
        .create_provider(
            "provider".into(),
            serde_json::from_value(json!({
                "name":"Fixture","base_url":"https://example.com/anthropic",
                "models":[{"id":"fake-large","context_window":200000},
                          {"id":"fake-small","context_window":200000}],
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    app.core
        .providers()
        .set_key("key".into(), "fixture", "synthetic")
        .await
        .unwrap();
    for name in ["Alpha", "Beta", "Delta", "Gamma"] {
        app.core
            .agents()
            .create_agent(
                name.into(),
                serde_json::from_value(json!({
                    "name":name,"provider":"fixture","default_model":"fake-large",
                }))
                .unwrap(),
            )
            .await
            .unwrap();
    }
    app
}

async fn group(app: &app::App, command: &str) -> Value {
    create(app, command, "Group", &["alpha", "beta"]).await
}

async fn create(app: &app::App, command: &str, name: &str, members: &[&str]) -> Value {
    let (status, body) = app::post(
        app,
        "/api/groups",
        json!({
            "command_id":command,"name":name,"members":members,
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

async fn messages_until(
    app: &app::App,
    path: &str,
    ready: impl Fn(&[Value]) -> bool,
) -> Vec<Value> {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (status, body) = app::call(app, "GET", path, None).await;
            assert_eq!(status, 200, "{body}");
            let rows = body.as_array().unwrap();
            if ready(rows) {
                return rows.clone();
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("the group did not reach the expected state")
}

async fn send(app: &app::App, path: &str, command: &str) -> Value {
    let (status, body) = app::post(
        app,
        path,
        json!({
            "command_id":command,"text":"@alpha hello",
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

async fn reply(app: &app::App, path: &str, after: i64) -> Value {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (status, body) =
                app::call(app, "GET", &format!("{path}?after={after}"), None).await;
            assert_eq!(status, 200, "{body}");
            if !body.as_array().unwrap().is_empty() {
                assert_eq!(body[0]["speaker"]["kind"], "agent", "{body}");
                return body[0].clone();
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("member reply was not recorded")
}

#[tokio::test]
async fn a_group_is_created_written_to_and_read_back() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    app.core.start_messages_for_test();
    let group = group(&app, "create").await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    let posted = send(&app, &path, "send").await;
    assert_eq!(posted["seq"], 1);
    let response = reply(&app, &path, 1).await;
    assert_eq!(response["speaker"]["slug"], "alpha");
    assert_eq!(response["body"], "hello from fake_acp");
    assert_eq!(response["to"], json!([]));
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

async fn event(app: &app::App, after: i64) -> Value {
    let project = app.core.agents().assistant_project().await.unwrap();
    let response = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("/api/projects/{project}/events?after={after}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let mut stream = response.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut buffer = String::new();
        loop {
            while let Some(end) = buffer.find("\n\n") {
                let frame: String = buffer.drain(..end + 2).collect();
                if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                    let value: Value = serde_json::from_str(data).unwrap();
                    if value["kind"] == "MessageAdded" {
                        return value;
                    }
                }
            }
            let chunk = stream.next().await.unwrap().unwrap();
            buffer.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    })
    .await
    .expect("MessageAdded was not streamed")
}

#[tokio::test]
async fn a_message_reaches_the_project_stream() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let a = group(&app, "a").await;
    let b = group(&app, "b").await;
    for group in [&a, &b] {
        let id = group["id"].as_str().unwrap();
        send(&app, &format!("/api/groups/{id}/messages"), "send").await;
    }
    let first = event(&app, 0).await;
    let second = event(&app, first["seq"].as_i64().unwrap()).await;
    assert_eq!(first["payload"], json!({"group_id":a["id"],"seq":1}));
    assert_eq!(second["payload"], json!({"group_id":b["id"],"seq":1}));
    assert!(second["seq"].as_i64().unwrap() > first["seq"].as_i64().unwrap());
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_turn_that_ends_before_its_link_is_still_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = group(&app, "create").await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    app.storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "CREATE TRIGGER hold_group_link
                     BEFORE UPDATE OF operation_id ON message_delivery
                     BEGIN SELECT RAISE(ABORT, 'held for recovery test'); END",
                )
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    let thread = group["members"][0]["thread_id"].as_str().unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let finished: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM operation WHERE thread_id=? AND status_kind='Completed'",
            )
            .bind(thread)
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
            if finished == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("turn did not finish while its link was held");
    let (_, rows) = app::call(&app, "GET", &path, None).await;
    assert_eq!(rows.as_array().unwrap().len(), 1);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    drop(app);
    let restarted = world(tmp.path()).await;
    restarted
        .storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("DROP TRIGGER hold_group_link")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    restarted
        .core
        .agents()
        .update_agent(
            "change-model".into(),
            "alpha",
            serde_json::from_value(json!({
                "name":"Alpha","provider":"fixture","default_model":"fake-small",
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    send(&restarted, &path, "later").await;
    restarted.core.start_messages_for_test();
    let answer = reply(&restarted, &path, 2).await;
    assert_eq!(answer["body"], "hello from fake_acp");
    let originals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM message WHERE group_id=? AND cause_seq=1
         AND operation_id IS NOT NULL",
    )
    .bind(group["id"].as_str().unwrap())
    .fetch_one(restarted.storage.reader())
    .await
    .unwrap();
    assert_eq!(originals, 1);
    restarted
        .core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_failed_turn_writes_a_notice_and_the_next_delivery_starts() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Fixture failure", &["alpha"]).await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    let (status, error) = app::post(
        &app,
        &path,
        json!({
            "command_id":"fail", "text":"@alpha fail this fixture",
        }),
    )
    .await;
    assert_eq!(status, 200, "{error}");
    send(&app, &path, "next").await;
    app.core.start_messages_for_test();
    let rows = messages_until(&app, &path, |rows| rows.len() >= 4).await;
    assert_eq!(rows[2]["kind"], "notice", "{rows:?}");
    assert_eq!(
        rows[2]["body"],
        "Alpha could not answer: the turn failed while running"
    );
    assert_eq!(rows[3]["body"], "hello from fake_acp");
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn parallel_mentions_stop_at_ten_deliveries_with_one_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(
        &app,
        "create",
        "Fixture loop",
        &["alpha", "beta", "delta", "gamma"],
    )
    .await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    let (status, body) = app::post(
        &app,
        &path,
        json!({
            "command_id":"loop", "text":"Begin",
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    app.core.start_messages_for_test();
    let rows = messages_until(&app, &path, |rows| rows.len() >= 12).await;
    assert_eq!(
        rows.iter()
            .filter(|row| row["speaker"]["kind"] == "agent")
            .count(),
        10
    );
    assert_eq!(rows.iter().filter(|row| row["kind"] == "notice").count(), 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message_delivery WHERE group_id=?")
        .bind(id)
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 10);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_busy_delivery_retries_after_a_lease_releases_without_a_write() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Group", &["alpha"]).await;
    let thread: shadows_core::ThreadId =
        serde_json::from_value(group["members"][0]["thread_id"].clone()).unwrap();
    let harness = app.core.harness();
    let session = harness.open_for_images(&thread, false).await.unwrap();
    let lease = harness.lease_events(&thread, &session).await.unwrap();
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let state: String =
                sqlx::query_scalar("SELECT state FROM message_delivery WHERE group_id=?")
                    .bind(id)
                    .fetch_one(app.storage.reader())
                    .await
                    .unwrap();
            if state == "starting" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    // This exceeds the fixture's cancel_wait + 1s lease deadline.
    tokio::time::sleep(Duration::from_millis(2400)).await;
    harness.give_back_events(&thread, &session, lease).await;
    assert_eq!(reply(&app, &path, 1).await["speaker"]["slug"], "alpha");
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn shutdown_is_bounded_while_group_settings_are_unanswered() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let marker = tmp.path().join("waiting-settings");
    app.core
        .agents()
        .set_env(
            "hold".into(),
            "alpha",
            "FAKE_ACP_HOLD_SETTINGS_AT",
            marker.to_str().unwrap(),
        )
        .await
        .unwrap();
    let group = create(&app, "create", "Group", &["alpha"]).await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    tokio::time::timeout(Duration::from_secs(15), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("fixture did not reach settings RPC");
    tokio::time::timeout(
        Duration::from_secs(5),
        app.core
            .shut_down(Duration::from_secs(2), std::future::pending()),
    )
    .await
    .expect("shutdown exceeded its bound")
    .unwrap();
}

#[tokio::test]
async fn a_reply_to_mohammed_only_wakes_no_one_and_marks_needs_you() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Fixture attention", &["alpha", "beta"]).await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}");
    let messages = format!("{path}/messages");
    app.core.start_messages_for_test();
    send(&app, &messages, "send").await;
    let answer = reply(&app, &messages, 1).await;
    assert_eq!(answer["to"], json!([{"kind":"person"}]));
    let (status, group) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 200);
    assert_eq!(group["needs_you"], true);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message_delivery WHERE group_id=?")
        .bind(id)
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
