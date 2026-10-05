//! Spec §20: writing while a turn runs — the queue and Send now.

#[path = "fixtures/app.rs"]
mod app;

use app::{call, ctx, default_settings, fresh_command, post, test_app};
use serde_json::{Value, json};
use shadows_core::StorageError;
use shadows_core::testing::acp;
use shadows_core::testing::queue::{QueueAnswer, new_queued};
use shadows_core::testing::turn::{new_turn, turn_command};

#[tokio::test]
async fn waiting_messages_keep_their_order_and_a_removed_one_is_gone() {
    let app = test_app().await;
    // A queue needs a busy thread: hold one turn open at the store.
    let settings = app::settings();
    app.storage
        .start_turn(
            &turn_command("t0", &app.thread, "hang", &settings),
            new_turn(&app.thread, &app.runtime, "hang", &settings),
        )
        .await
        .unwrap();
    let first = match app
        .storage
        .queue_message(&ctx("q1", "turn.queue"), &app.thread, new_queued("one"))
        .await
        .unwrap()
    {
        QueueAnswer::Waiting(m) => m,
        QueueAnswer::Idle => panic!("a busy thread queues"),
    };
    app.storage
        .queue_message(&ctx("q2", "turn.queue"), &app.thread, new_queued("two"))
        .await
        .unwrap();
    let listed = app.storage.queued_messages(&app.thread).await.unwrap();
    let prompts: Vec<&str> = listed.iter().map(|m| m.prompt.as_str()).collect();
    assert_eq!(prompts, ["one", "two"]);

    app.storage
        .unqueue_message(&ctx("u1", "turn.unqueue"), &app.thread, &first.id)
        .await
        .unwrap();
    let again = app
        .storage
        .unqueue_message(&ctx("u2", "turn.unqueue"), &app.thread, &first.id)
        .await;
    assert!(matches!(again, Err(StorageError::QueuedMessageGone)));
}

fn queue_body(prompt: &str) -> Value {
    let mut body = default_settings();
    body["command_id"] = json!(fresh_command());
    body["prompt"] = json!(prompt);
    body
}

/// Lets the `wait-for-release` turn finish: fake-acp looks for this file.
async fn release(app: &app::App) {
    let context = app.storage.turn_context(&app.thread).await.unwrap();
    let dir = context.project_directory.expect("a project directory");
    std::fs::write(dir.join("release"), "").unwrap();
}

#[tokio::test]
async fn queueing_on_an_idle_thread_starts_a_turn_and_leaves_no_row() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (status, answer) = post(&app, &path, queue_body("hello")).await;
    assert_eq!(status, 202, "{answer}");
    assert_eq!(answer["status"], "started");
    assert!(answer["operation_id"].is_string());
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn a_replayed_queue_on_an_idle_thread_answers_its_turn() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let mut body = queue_body("wait-for-release");
    body["command_id"] = json!("same");
    let (_, first) = post(&app, &path, body.clone()).await;
    // The turn it started still runs: a replay must not queue a copy behind it.
    let (status, again) = post(&app, &path, body).await;
    assert_eq!(status, 202, "{again}");
    assert_eq!(again["status"], "started");
    assert_eq!(again["operation_id"], first["operation_id"]);
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
    release(&app).await;
}

#[tokio::test]
async fn a_busy_thread_queues_and_remove_answers_gone_the_second_time() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    // On the idle thread this starts the turn that holds it busy.
    let (_, started) = post(&app, &path, queue_body("wait-for-release")).await;
    assert_eq!(started["status"], "started");
    let (status, queued) = post(&app, &path, queue_body("next one")).await;
    assert_eq!(status, 202, "{queued}");
    assert_eq!(queued["status"], "waiting");
    assert_eq!(queued["message"]["prompt"], "next one");
    let qid = queued["message"]["id"].as_str().unwrap();
    let remove = format!("{path}/{qid}?command_id={}", fresh_command());
    assert_eq!(call(&app, "DELETE", &remove, None).await.0, 204);
    // The same command again answers as the first did, not as a gone row.
    assert_eq!(call(&app, "DELETE", &remove, None).await.0, 204);
    let again = format!("{path}/{qid}?command_id={}", fresh_command());
    let (status, gone) = call(&app, "DELETE", &again, None).await;
    assert_eq!(status, 404);
    assert_eq!(gone["code"], "QUEUED_MESSAGE_GONE");
    release(&app).await;
}

async fn until_ops(app: &app::App, n: usize) -> Vec<shadows_core::Operation> {
    for _ in 0..500 {
        let ops = app
            .storage
            .list_operations_for_thread(&app.thread)
            .await
            .unwrap();
        if ops.len() >= n && ops.iter().all(|o| o.finished_at.is_some()) {
            return ops;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the thread never reached {n} finished turns");
}

#[tokio::test]
async fn a_completed_turn_starts_the_next_waiting_message_exactly_once() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    post(&app, &path, queue_body("wait-for-release")).await;
    let (_, queued) = post(&app, &path, queue_body("after it")).await;
    assert_eq!(queued["status"], "waiting");
    release(&app).await;

    let ops = until_ops(&app, 2).await;
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|o| o.status_kind == "Completed"), "{ops:?}");
    let sent = app::entries(&app)
        .await
        .into_iter()
        .filter(|e| e.body == "after it")
        .count();
    assert_eq!(sent, 1);
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn send_now_steers_the_running_turn_after_its_streamed_text() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("steerable")).await;
    let (_, queued) = post(&app, &path, queue_body("turn left")).await;
    let qid = queued["message"]["id"].as_str().unwrap();
    // Let the turn stream its first message before the steer.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let now = format!("{path}/{qid}/send-now");
    let (status, answer) = post(&app, &now, json!({ "command_id": fresh_command() })).await;
    assert_eq!(status, 202, "{answer}");
    assert_eq!(answer["status"], "steered");

    let ops = until_ops(&app, 1).await;
    assert_eq!(ops.len(), 1, "a steer starts no turn of its own");
    assert_eq!(ops[0].id.as_str(), first["operation_id"].as_str().unwrap());
    let bodies: Vec<String> = app::entries(&app)
        .await
        .into_iter()
        .map(|e| e.body)
        .collect();
    assert_eq!(
        bodies,
        ["steerable", "waiting", "turn left", "steered: turn left"]
    );
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn send_now_after_stop_is_thread_busy_and_keeps_the_row() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("ignore-cancel")).await;
    let (_, queued) = post(&app, &path, queue_body("later")).await;
    let qid = queued["message"]["id"].as_str().unwrap();
    let op = first["operation_id"].as_str().unwrap();
    // `ignore-cancel` never confirms, so the Stop stays pending a while.
    let stop_path = format!("/api/operations/{op}/stop");
    let stop = post(&app, &stop_path, json!({}));
    let now = format!("{path}/{qid}/send-now");
    let send = async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        post(&app, &now, json!({ "command_id": fresh_command() })).await
    };
    let (_, (status, answer)) = tokio::join!(stop, send);
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["code"], "THREAD_BUSY");
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed[0]["prompt"], "later");
    assert!(listed[0]["last_error"].is_null());
}

#[tokio::test]
async fn the_watcher_and_send_now_never_send_one_message_twice() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    post(&app, &path, queue_body("wait-for-release")).await;
    let (_, queued) = post(&app, &path, queue_body("once")).await;
    let qid = queued["message"]["id"].as_str().unwrap().to_string();
    release(&app).await;
    // Send now as the turn completes. `wait-for-release` is not steerable, so
    // the adapter answers promptRequired: Send now starts it (202), finds the
    // turn still holding the thread (409), or finds it already sent (404, or
    // 202 replaying the watcher's start under the shared `queued:<id>`).
    let now = format!("{path}/{qid}/send-now");
    let (status, _) = post(&app, &now, json!({ "command_id": fresh_command() })).await;
    assert!([202, 404, 409].contains(&status), "{status}");
    let ops = until_ops(&app, 2).await;
    assert_eq!(ops.len(), 2);
    let sent = app::entries(&app)
        .await
        .into_iter()
        .filter(|e| e.body == "once")
        .count();
    assert_eq!(sent, 1);
}

#[tokio::test]
async fn stop_leaves_the_queue_and_starts_nothing() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("hang")).await;
    post(&app, &path, queue_body("waits")).await;
    let op = first["operation_id"].as_str().unwrap();
    post(&app, &format!("/api/operations/{op}/stop"), json!({})).await;

    let ops = until_ops(&app, 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let after = app
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert_eq!(after.len(), 1, "{ops:?}");
    assert_eq!(ops[0].status_kind, "Cancelled");
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed[0]["prompt"], "waits");
    assert!(listed[0]["last_error"].is_null());
}
