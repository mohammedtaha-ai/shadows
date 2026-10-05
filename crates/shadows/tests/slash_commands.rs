//! The `/` menu's list (spec §21): the harness's `available_commands_update`
//! reaches the thread stream as a transient `commands` frame, live and again
//! after `caught-up` for a stream opened later.

use serde_json::{Value, json};
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;

use app::{post, test_app};

fn names(frame: &Value) -> Vec<String> {
    frame["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn opening_a_session_sends_its_commands_to_a_subscriber() {
    let app = test_app().await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    post(
        &app,
        &format!("/api/threads/{}/session", app.thread),
        json!({}),
    )
    .await;
    let frame = app::next_frame_named(&mut sub, "commands").await;
    assert_eq!(frame["thread_id"], app.thread.as_str());
    assert_eq!(names(&frame), ["compact", "superpowers:brainstorming"]);
    assert_eq!(frame["commands"][0]["hint"], Value::Null);
    assert_eq!(frame["commands"][1]["hint"], "[topic]");
}

#[tokio::test]
async fn a_stream_opened_after_the_list_gets_it_after_caught_up() {
    let app = test_app().await;
    let mut first = app::subscribe(&app, &app.thread).await;
    post(
        &app,
        &format!("/api/threads/{}/session", app.thread),
        json!({}),
    )
    .await;
    app::next_frame_named(&mut first, "commands").await;
    let mut later = app::subscribe(&app, &app.thread).await;
    let frame = app::next_frame_named(&mut later, "commands").await;
    assert_eq!(names(&frame), ["compact", "superpowers:brainstorming"]);
}
