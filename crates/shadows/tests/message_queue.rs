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
    let again = format!("{path}/{qid}?command_id={}", fresh_command());
    let (status, gone) = call(&app, "DELETE", &again, None).await;
    assert_eq!(status, 404);
    assert_eq!(gone["code"], "QUEUED_MESSAGE_GONE");
    release(&app).await;
}
