use serde_json::json;
use shadows_agent::events::TaskUpdate;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;

async fn stop(app: &app::App, id: &str) -> (u16, serde_json::Value) {
    app::post(
        app,
        &format!("/api/threads/{}/session/tasks/{id}/stop", app.thread),
        json!({}),
    )
    .await
}

async fn open(app: &app::App) {
    app.core.harness().open_session(&app.thread).await.unwrap();
}

fn spawn(app: &app::App, id: &str) {
    app.core.harness().report_task_for_test(
        &app.thread,
        TaskUpdate::Spawned {
            async_task_id: id.into(),
            name: "job".into(),
            task_type: "shell".into(),
            description: "run".into(),
            show_in_transcript: false,
            can_stop: true,
            output_file_path: None,
            tool_call_id: None,
        },
    );
}

#[tokio::test]
async fn stop_reaches_the_adapter() {
    let app = app::test_app().await;
    open(&app).await;
    spawn(&app, "task-1");
    let (status, body) = stop(&app, "task-1").await;
    assert_eq!(status, 200);
    assert_eq!(body, json!({"stopped":true}));
    assert_eq!(app.core.harness().tasks_of(&app.thread)[0].state, "running");
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn unknown_task_is_404() {
    let app = app::test_app().await;
    assert_eq!(stop(&app, "missing").await.0, 404);
    open(&app).await;
    assert_eq!(stop(&app, "missing").await.0, 404);
    let unknown = shadows_core::ThreadId::generate();
    let path = format!("/api/threads/{unknown}/session/tasks/missing/stop");
    assert_eq!(app::post(&app, &path, json!({})).await.0, 404);
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn tasks_are_published_between_turns() {
    let app = app::test_app().await;
    open(&app).await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    app::next_frame_named(&mut sub, "commands").await;
    spawn(&app, "task-1");
    app.core.harness().report_task_for_test(
        &app.thread,
        TaskUpdate::State {
            async_task_id: "task-1".into(),
            state: "completed".into(),
            summary: None,
            output_file_path: None,
            tool_call_id: None,
        },
    );
    let frame = app::next_frame_named(&mut sub, "tasks").await;
    assert_eq!(frame["tasks"][0]["state"], "running");
    assert_eq!(frame["op"], serde_json::Value::Null);
    let frame = app::next_frame_named(&mut sub, "tasks").await;
    assert_eq!(frame["tasks"][0]["state"], "completed");
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn an_ended_task_is_not_stopped() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "async-tasks").await;
    app::wait_terminal(&app, &op).await;
    assert_eq!(app::last_agent_entry(&app).await.body, "after tasks");
    assert_eq!(stop(&app, "task-1").await, (200, json!({"stopped":false})));
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_subscriber_gets_the_tasks_after_caught_up() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "async-tasks").await;
    app::wait_terminal(&app, &op).await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    let frames = app::frames_until(&mut sub, |_, _| true).await;
    let (name, frame) = &frames[0];
    assert_eq!(name, "tasks");
    assert_eq!(frame["tasks"][0]["id"], "task-1");
    assert_eq!(frame["tasks"][0]["state"], "completed");
    assert_eq!(frame["op"], serde_json::Value::Null);
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn snapshot_does_not_regress_to_an_older_queued_change() {
    let app = app::test_app().await;
    open(&app).await;
    let mut sub = app::subscribe_from(&app, &app.thread, 0).await;
    spawn(&app, "first");
    app.core.harness().report_task_for_test(
        &app.thread,
        TaskUpdate::State {
            async_task_id: "first".into(),
            state: "completed".into(),
            summary: None,
            output_file_path: None,
            tool_call_id: None,
        },
    );
    app::next_frame_named(&mut sub, "caught-up").await;
    let snapshot = app::next_frame_named(&mut sub, "tasks").await;
    assert_eq!(snapshot["tasks"][0]["state"], "completed");
    spawn(&app, "second");
    let frame = app::next_frame_named(&mut sub, "tasks").await;
    assert_eq!(frame["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(frame["tasks"][0]["state"], "completed");
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
