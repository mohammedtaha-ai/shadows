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
    say(app, path, command, "@alpha hello").await
}

async fn say(app: &app::App, path: &str, command: &str, text: &str) -> Value {
    let (status, body) = app::post(
        app,
        path,
        json!({
            "command_id":command,"text":text,
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

async fn execute(app: &app::App, sql: &'static str) {
    app.storage
        .write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(sql).execute(conn).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
}

async fn count(app: &app::App, sql: &'static str, bind: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(bind)
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

/// Each delivery of a group, in trigger order: its member, round, state, operation and prompt.
async fn deliveries(
    app: &app::App,
    group: &str,
) -> Vec<(String, i64, String, Option<String>, String)> {
    sqlx::query_as(
        "SELECT slug, round, state, operation_id, prompt FROM message_delivery
         WHERE group_id=? ORDER BY trigger_seq, slug",
    )
    .bind(group)
    .fetch_all(app.storage.reader())
    .await
    .unwrap()
}

const HOLD_LINK: &str = "CREATE TRIGGER hold BEFORE UPDATE OF operation_id ON message_delivery
     BEGIN SELECT RAISE(ABORT, 'held for recovery test'); END";
const HOLD_REPLY: &str = "CREATE TRIGGER hold BEFORE INSERT ON message
     WHEN NEW.operation_id IS NOT NULL
     BEGIN SELECT RAISE(ABORT, 'held for recovery test'); END";

/// Alpha's turn for `@alpha hello` completes while `hold` refuses one write of its
/// delivery; Core stops, and starts again over the same database with the write allowed.
/// Answers the restarted app (its worker not yet started), the group and the turn's id.
async fn complete_while_held(
    dir: &Path,
    hold: &'static str,
    state: &str,
) -> (app::App, Value, String) {
    let app = world(dir).await;
    let group = group(&app, "create").await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    execute(&app, hold).await;
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    let thread = group["members"][0]["thread_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let id = group["id"].as_str().unwrap().to_owned();
    let operation = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let done: Option<String> = sqlx::query_scalar(
                "SELECT o.id FROM operation o, message_delivery d
                 WHERE o.thread_id=? AND o.status_kind='Completed'
                   AND d.group_id=? AND d.state=?",
            )
            .bind(&thread)
            .bind(&id)
            .bind(state)
            .fetch_optional(app.storage.reader())
            .await
            .unwrap();
            if let Some(done) = done {
                return done;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("turn did not finish while its delivery was held");
    let recorded = "SELECT COUNT(*) FROM message WHERE group_id=? AND operation_id IS NOT NULL";
    assert_eq!(count(&app, recorded, &id).await, 0);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    drop(app);
    let restarted = world(dir).await;
    execute(&restarted, "DROP TRIGGER hold").await;
    (restarted, group, operation)
}

async fn change_alpha_model(app: &app::App) {
    app.core
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

/// The project's `MessageAdded` frames after `after`: exactly `count` of them, with no
/// further one arriving in the next 300 ms.
async fn message_frames(app: &app::App, after: i64, count: usize) -> Vec<Value> {
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
    let mut buffer = String::new();
    let mut frames = Vec::new();
    loop {
        while let Some(end) = buffer.find("\n\n") {
            let frame: String = buffer.drain(..end + 2).collect();
            if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                let value: Value = serde_json::from_str(data).unwrap();
                if value["kind"] == "MessageAdded" {
                    frames.push(value);
                }
            }
        }
        let wait = if frames.len() < count {
            Duration::from_secs(10)
        } else {
            Duration::from_millis(300)
        };
        match tokio::time::timeout(wait, stream.next()).await {
            Ok(Some(chunk)) => buffer.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap()),
            Ok(None) | Err(_) => break,
        }
    }
    assert_eq!(frames.len(), count, "{frames:?}");
    frames
}

#[tokio::test]
async fn a_message_reaches_the_project_stream() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let a = group(&app, "a").await;
    let path = format!("/api/groups/{}/messages", a["id"].as_str().unwrap());
    send(&app, &path, "send").await;
    let frames = message_frames(&app, 0, 1).await;
    assert_eq!(frames[0]["payload"], json!({"group_id":a["id"],"seq":1}));
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reconnect_resumes_after_messages_of_two_groups() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let a = group(&app, "a").await;
    let b = group(&app, "b").await;
    for (command, group) in [("a1", &a), ("b1", &b), ("a2", &a), ("b2", &b)] {
        let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
        say(&app, &path, command, "hello").await;
    }
    let all = message_frames(&app, 0, 4).await;
    let payloads: Vec<_> = all.iter().map(|frame| frame["payload"].clone()).collect();
    assert_eq!(
        payloads,
        [
            json!({"group_id":a["id"],"seq":1}),
            json!({"group_id":b["id"],"seq":1}),
            json!({"group_id":a["id"],"seq":2}),
            json!({"group_id":b["id"],"seq":2}),
        ]
    );
    let middle = all[1]["seq"].as_i64().unwrap();
    let rest = message_frames(&app, middle, 2).await;
    let seqs = |frames: &[Value]| -> Vec<(i64, Value)> {
        frames
            .iter()
            .map(|frame| (frame["seq"].as_i64().unwrap(), frame["payload"].clone()))
            .collect()
    };
    assert_eq!(seqs(&rest), seqs(&all[2..]));
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_turn_that_ends_before_its_link_is_still_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let (restarted, group, _) = complete_while_held(tmp.path(), HOLD_LINK, "starting").await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    change_alpha_model(&restarted).await;
    send(&restarted, &path, "later").await;
    restarted.core.start_messages_for_test();
    let answer = reply(&restarted, &path, 2).await;
    assert_eq!(answer["body"], "hello from fake_acp");
    let originals = "SELECT COUNT(*) FROM message WHERE group_id=? AND cause_seq=1
         AND operation_id IS NOT NULL";
    assert_eq!(count(&restarted, originals, id).await, 1);
    restarted
        .core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_redispatch_uses_the_stored_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let (restarted, group, operation) =
        complete_while_held(tmp.path(), HOLD_LINK, "starting").await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    change_alpha_model(&restarted).await;
    send(&restarted, &path, "later").await;
    restarted.core.start_messages_for_test();
    messages_until(&restarted, &path, |rows| rows.len() >= 4).await;
    let (model, linked): (String, String) = sqlx::query_as(
        "SELECT model, operation_id FROM message_delivery WHERE group_id=? AND trigger_seq=1",
    )
    .bind(id)
    .fetch_one(restarted.storage.reader())
    .await
    .unwrap();
    assert_eq!(linked, operation);
    assert_eq!(model, "fake-large");
    let notices = "SELECT COUNT(*) FROM message WHERE group_id=? AND kind='notice'";
    assert_eq!(count(&restarted, notices, id).await, 0);
    let rows = deliveries(&restarted, id).await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.2 == "answered"), "{rows:?}");
    restarted
        .core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reply_survives_a_restart_before_it_is_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let (restarted, group, operation) =
        complete_while_held(tmp.path(), HOLD_REPLY, "running").await;
    let path = format!("/api/groups/{}/messages", group["id"].as_str().unwrap());
    restarted.core.start_messages_for_test();
    let answer = reply(&restarted, &path, 1).await;
    assert_eq!(answer["speaker"]["slug"], "alpha");
    let recorded = "SELECT COUNT(*) FROM message WHERE operation_id=?";
    assert_eq!(count(&restarted, recorded, &operation).await, 1);
    restarted
        .core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reply_is_recorded_once() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = group(&app, "create").await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    reply(&app, &path, 1).await;
    // The journal is read again from its start: an ending already recorded records nothing.
    execute(&app, "UPDATE message_cursor SET seq = 0").await;
    say(&app, &path, "again", "@beta hello").await;
    let rows = messages_until(&app, &path, |rows| rows.len() >= 4).await;
    assert_eq!(rows[3]["speaker"]["slug"], "beta", "{rows:?}");
    let agents = rows
        .iter()
        .filter(|row| row["speaker"]["kind"] == "agent")
        .count();
    assert_eq!(agents, 2, "{rows:?}");
    let recorded = "SELECT COUNT(*) FROM message WHERE group_id=? AND operation_id IS NOT NULL";
    assert_eq!(count(&app, recorded, id).await, 2);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reply_without_member_mentions_wakes_no_one() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = group(&app, "create").await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    app.core.start_messages_for_test();
    send(&app, &path, "send").await;
    let answer = reply(&app, &path, 1).await;
    assert_eq!(answer["to"], json!([]));
    let rows = deliveries(&app, id).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].2, "answered");
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reply_mentioning_another_member_reaches_it() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Fixture loop", &["alpha", "beta"]).await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    say(&app, &path, "go", "@alpha go").await;
    app.core.start_messages_for_test();
    let rows = messages_until(&app, &path, |rows| {
        rows.iter().any(|row| row["kind"] == "notice")
    })
    .await;
    assert_eq!(rows[1]["speaker"]["slug"], "alpha", "{rows:?}");
    assert_eq!(rows[1]["to"], json!([{"kind":"agent","slug":"beta"}]));
    assert_eq!(rows[2]["speaker"]["slug"], "beta", "{rows:?}");
    assert_eq!(rows[2]["to"], json!([{"kind":"agent","slug":"alpha"}]));
    assert_eq!(
        rows[4]["body"],
        "The group stopped here: the limit for one message was reached."
    );
    let rounds: Vec<(String, i64)> = deliveries(&app, id)
        .await
        .into_iter()
        .map(|row| (row.0, row.1))
        .collect();
    assert_eq!(
        rounds,
        [("alpha".into(), 1), ("beta".into(), 2), ("alpha".into(), 3)]
    );
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_prompt_carries_only_what_is_new() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Group", &["alpha"]).await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    app.core.start_messages_for_test();
    say(&app, &path, "first", "@alpha first question").await;
    reply(&app, &path, 1).await;
    say(&app, &path, "second", "@alpha second question").await;
    reply(&app, &path, 3).await;
    let rows = deliveries(&app, id).await;
    assert!(rows[0].4.contains("Mohammed: @alpha first question"));
    assert!(
        rows[0]
            .4
            .starts_with("Group: Group\nMembers: Alpha (@alpha)\n")
    );
    assert!(rows[1].4.contains("Mohammed: @alpha second question"));
    assert!(!rows[1].4.contains("first question"), "{}", rows[1].4);
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_second_message_to_a_busy_member_gets_its_own_turn() {
    let tmp = tempfile::tempdir().unwrap();
    let app = world(tmp.path()).await;
    let group = create(&app, "create", "Group", &["alpha"]).await;
    let id = group["id"].as_str().unwrap();
    let path = format!("/api/groups/{id}/messages");
    say(&app, &path, "one", "@alpha one").await;
    say(&app, &path, "two", "@alpha two").await;
    app.core.start_messages_for_test();
    let rows = messages_until(&app, &path, |rows| rows.len() >= 4).await;
    assert_eq!(rows[2]["speaker"]["slug"], "alpha", "{rows:?}");
    assert_eq!(rows[3]["speaker"]["slug"], "alpha", "{rows:?}");
    let rows = deliveries(&app, id).await;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.2 == "answered"), "{rows:?}");
    assert_ne!(rows[0].3, rows[1].3);
    assert!(rows[1].4.contains("Mohammed: @alpha two"));
    assert!(!rows[1].4.contains("@alpha one"), "{}", rows[1].4);
    let thread = group["members"][0]["thread_id"].as_str().unwrap();
    let turns = "SELECT COUNT(*) FROM operation WHERE thread_id=?";
    assert_eq!(count(&app, turns, thread).await, 2);
    app.core
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
