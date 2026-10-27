//! Conversation permission choices survive session and daemon lifetimes.
use serde_json::{Value, json};
use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
use app::{App, call, default_settings, get_json, post, test_app};

async fn pick(app: &App, command: &str, mode: &str) -> (u16, Value) {
    call(
        app,
        "PUT",
        &format!("/api/threads/{}/session/mode", app.thread),
        Some(json!({"command_id": command, "mode": mode})),
    )
    .await
}

async fn open(app: &App, thread: &str) -> Value {
    let (status, choices) = post(app, &format!("/api/threads/{thread}/session"), json!({})).await;
    assert_eq!(status, 200, "{choices}");
    choices
}

#[tokio::test]
async fn picking_auto_without_sending_survives_reopen_and_core_restart() {
    let dir = tempfile::tempdir().unwrap();
    let app = app::test_app_at(dir.path()).await;
    let (status, choices) = pick(&app, "pick", "auto").await;
    assert_eq!(status, 200, "{choices}");
    assert_eq!(choices["current"]["mode"], "auto");
    app.sessions.terminate(&app.thread).await.unwrap();
    assert_eq!(
        open(&app, app.thread.as_str()).await["current"]["mode"],
        "auto"
    );
    let entries: Vec<Value> = get_json(&app, &format!("/api/threads/{}/entries", app.thread)).await;
    assert!(entries.is_empty(), "a pick sends no turn");
    app::shut_down_app(app).await;
    let app = app::test_app_at(dir.path()).await;
    assert_eq!(
        open(&app, app.thread.as_str()).await["current"]["mode"],
        "auto"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

async fn forbid_auto(app: &App) {
    shadows_core::testing::set_project_modes(
        &app.storage,
        &app::ctx("restrict", "project.modes"),
        &app.project,
        &[("claude-code".into(), vec!["acceptEdits".into()])].into(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn disallowed_mode_is_refused_before_any_write() {
    let app = test_app().await;
    forbid_auto(&app).await;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_record")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    let (status, answer) = pick(&app, "refused", "auto").await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (403, Some("MODE_NOT_ALLOWED"))
    );
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_record")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        app.sessions.live_count().await,
        0,
        "not even an opening grant is written"
    );
    assert_eq!(mode_rows(&app).await, 0);

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

async fn mode_rows(app: &App) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM harness_thread_mode")
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn disallowed_remembered_mode_falls_back_on_open() {
    let app = test_app().await;
    assert_eq!(pick(&app, "auto", "auto").await.0, 200);
    forbid_auto(&app).await;
    app.sessions.terminate(&app.thread).await.unwrap();
    assert_eq!(
        open(&app, app.thread.as_str()).await["current"]["mode"],
        "acceptEdits"
    );
    assert_eq!(
        app.sessions
            .offered(&app.thread)
            .await
            .unwrap()
            .current
            .mode,
        "acceptEdits"
    );
    assert_eq!(
        mode_rows(&app).await,
        0,
        "the disallowed preference is dropped"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_new_conversation_including_a_draft_uses_its_own_default() {
    let app = test_app().await;
    assert_eq!(pick(&app, "auto", "auto").await.0, 200);
    let other = app::create_thread(&app, json!({"command_id":"draft", "title":"Draft"})).await;
    assert_eq!(
        open(&app, other["id"].as_str().unwrap()).await["current"]["mode"],
        "acceptEdits"
    );
    assert_eq!(
        open(&app, app.thread.as_str()).await["current"]["mode"],
        "auto"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_replay_with_another_mode_conflicts_and_equal_replay_keeps_later_choice() {
    let app = test_app().await;
    assert_eq!(pick(&app, "same", "auto").await.0, 200);
    assert_eq!(pick(&app, "same", "auto").await.0, 200);
    let (status, answer) = pick(&app, "same", "acceptEdits").await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    assert_eq!(pick(&app, "later", "acceptEdits").await.0, 200);
    let (status, answer) = pick(&app, "same", "auto").await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["current"]["mode"], "acceptEdits");
    assert_eq!(mode_rows(&app).await, 1);

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn running_turn_keeps_its_mode_and_next_turn_uses_the_pick() {
    let app = test_app().await;
    let running = app::start_settled(&app, "hang").await;
    let (status, answer) = pick(&app, "during", "auto").await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["current"]["mode"], "auto");
    assert_eq!(
        app.sessions
            .offered(&app.thread)
            .await
            .unwrap()
            .current
            .mode,
        "acceptEdits"
    );
    let (status, _) = post(&app, &format!("/api/operations/{running}/stop"), json!({})).await;
    assert_eq!(status, 200);
    app::wait_terminal(&app, &running).await;
    let next = app::start_and_finish(&app, "hello", answer["current"].clone()).await;
    assert_eq!(next.status_kind, "Completed");
    assert_eq!(
        app.sessions
            .offered(&app.thread)
            .await
            .unwrap()
            .current
            .mode,
        "auto"
    );
    let modes: Vec<String> =
        sqlx::query_scalar("SELECT requested_mode FROM agent_invocation ORDER BY created_at, id")
            .fetch_all(app.storage.reader())
            .await
            .unwrap();
    assert_eq!(modes, ["acceptEdits", "auto"]);

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn fork_copies_the_mode_and_removal_deletes_only_its_row() {
    let app = test_app().await;
    app::start_and_finish(&app, "hello", default_settings()).await;
    assert_eq!(pick(&app, "auto", "auto").await.0, 200);
    let entries: Vec<Value> = get_json(&app, &format!("/api/threads/{}/entries", app.thread)).await;
    let (status, fork) = post(
        &app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({"command_id":"fork", "at_entry_id":entries.last().unwrap()["id"]}),
    )
    .await;
    assert_eq!(status, 201, "{fork}");
    assert_eq!(
        open(&app, fork["id"].as_str().unwrap()).await["current"]["mode"],
        "auto"
    );
    assert_eq!(mode_rows(&app).await, 2);
    app.core
        .turns()
        .remove_thread("remove".into(), &app.thread)
        .await
        .unwrap();
    assert_eq!(mode_rows(&app).await, 1);
    assert_eq!(
        open(&app, fork["id"].as_str().unwrap()).await["current"]["mode"],
        "auto"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_turn_start_remembers_its_explicit_mode_in_its_transaction() {
    let app = test_app().await;
    let mut settings = default_settings();
    settings["mode"] = json!("auto");
    assert_eq!(
        app::start_and_finish(&app, "hello", settings)
            .await
            .status_kind,
        "Completed"
    );
    app.sessions.terminate(&app.thread).await.unwrap();
    assert_eq!(
        open(&app, app.thread.as_str()).await["current"]["mode"],
        "auto"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn idle_choices_report_the_adapters_mode() {
    let app = test_app().await;
    assert_eq!(pick(&app, "auto", "auto").await.0, 200);
    let opened = app.sessions.open(&app.thread).await.unwrap();
    // Model changes can move the adapter's mode without changing the saved pick.
    app.sessions
        .set_option(&app.thread, &opened, "mode", "acceptEdits")
        .await
        .unwrap();
    let (status, choices) = call(
        &app,
        "PUT",
        &format!("/api/threads/{}/session/model", app.thread),
        Some(json!({"model": "fake-large"})),
    )
    .await;
    assert_eq!(status, 200, "{choices}");
    assert_eq!(choices["current"]["mode"], "acceptEdits");
    let saved: String =
        sqlx::query_scalar("SELECT mode FROM harness_thread_mode WHERE thread_id = ?")
            .bind(app.thread.as_str())
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    assert_eq!(
        saved, "auto",
        "the adapter's mode does not overwrite the pick"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
