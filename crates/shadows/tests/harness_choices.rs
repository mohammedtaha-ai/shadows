//! The choices a thread's session offers (spec §12.4): read from the harness,
//! filtered by Shadows' policy and the project's modes, started at the
//! remembered model and effort when the harness still offers them; and the
//! harness list (§12.10).

use serde_json::{Value, json};
use shadows::agent::policy;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{ctx, get_json, names, post, test_app};

fn session_path(app: &app::App) -> String {
    format!("/api/threads/{}/session", app.thread)
}

#[tokio::test]
async fn opening_a_session_answers_the_harness_choices_after_the_policy() {
    let app = test_app().await;
    let (s, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(s, 200, "{c}");
    assert_eq!(
        names(&c["models"]),
        ["fake-large", "fake-small", "fake-tiny", "fake-locked"]
    );
    assert_eq!(c["models"][0]["label"], "Fake Large");
    assert_eq!(names(&c["efforts"]), ["low", "high", "max"]);
    assert_eq!(names(&c["modes"]), ["acceptEdits", "auto"]);
    assert_eq!(
        c["current"]["mode"], "acceptEdits",
        "the fake starts in auto, as the real adapter does; opening sets the default"
    );
    let (s2, again) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(
        (s2, &again),
        (200, &c),
        "an open session answers what it holds"
    );
    assert_eq!(app.sessions.live_count().await, 1);
}

#[tokio::test]
async fn a_mode_the_project_does_not_allow_is_listed_disabled_with_the_reason() {
    let app = test_app().await;
    let only = [("claude-code".to_string(), vec!["acceptEdits".to_string()])].into();
    app.storage
        .set_project_modes(&ctx("m1", "project.modes"), &app.project, &only)
        .await
        .unwrap();
    let (_, c) = post(&app, &session_path(&app), json!({})).await;
    let auto = &c["modes"][1];
    assert_eq!(
        (&auto["id"], &auto["enabled"], &auto["reason"]),
        (
            &json!("auto"),
            &json!(false),
            &json!("Not allowed in this project")
        )
    );
}

#[tokio::test]
async fn a_remembered_model_the_account_cannot_use_is_dropped() {
    let app = test_app().await;
    app.storage
        .remember_for_test("claude-code", "fake-locked", None)
        .await;
    let (s, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(
        (s, c["current"]["model"].as_str()),
        (200, Some("fake-large"))
    );
}

#[tokio::test]
async fn harnesses_lists_claude_runnable_and_codex_not() {
    let app = test_app().await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        (h[0]["kind"].as_str(), h[0]["available"].as_bool()),
        (Some("claude-code"), Some(true))
    );
    assert!(h[0]["remembered"].is_null());
    assert!(h[0]["limits"].is_null());
    assert_eq!(
        (h[1]["kind"].as_str(), h[1]["available"].as_bool()),
        (Some("codex"), Some(false))
    );
    assert!(h[1]["reason"].is_string());
}

#[tokio::test]
async fn a_new_session_starts_at_the_remembered_model_and_effort() {
    let app = test_app().await;
    app.storage
        .remember_for_test("claude-code", "fake-small", Some("low"))
        .await;
    let (_, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(
        (
            c["current"]["model"].as_str(),
            c["current"]["effort"].as_str(),
            c["current"]["mode"].as_str()
        ),
        (Some("fake-small"), Some("low"), Some("acceptEdits"))
    );
    assert_eq!(
        names(&c["efforts"]),
        ["low", "high"],
        "the remembered model's efforts"
    );
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        h[0]["remembered"],
        json!({ "model": "fake-small", "effort": "low" })
    );
}

#[tokio::test]
async fn a_remembered_model_the_harness_no_longer_offers_is_dropped() {
    let app = test_app().await;
    app.storage
        .remember_for_test("claude-code", "retired-model", Some("high"))
        .await;
    let (s, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(s, 200);
    assert_eq!(c["current"]["model"], "fake-large"); // the fake's own default
}

#[tokio::test]
async fn a_codex_thread_cannot_open_a_session() {
    let app = test_app().await;
    let t = app
        .storage
        .create_planning_thread(
            &ctx("c9", "thread.create"),
            &app.project,
            "x",
            policy::CODEX,
        )
        .await
        .unwrap();
    let (s, b) = post(&app, &format!("/api/threads/{}/session", t.id), json!({})).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
    assert_eq!(app.sessions.live_count().await, 0, "nothing was started");
}

#[tokio::test]
async fn a_change_to_the_offer_reaches_a_subscriber_as_an_options_frame() {
    let app = test_app().await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    post(&app, &session_path(&app), json!({})).await;
    let frame = app::next_frame_named(&mut sub, "options").await;
    assert_eq!(frame["thread_id"], app.thread.as_str());
    assert_eq!(names(&frame["choices"]["modes"]), ["acceptEdits", "auto"]);
}

fn model_path(app: &app::App) -> String {
    format!("/api/threads/{}/session/model", app.thread)
}

async fn put_model(app: &app::App, model: &str) -> (u16, Value) {
    app::call(
        app,
        "PUT",
        &model_path(app),
        Some(json!({ "model": model })),
    )
    .await
}

/// §12.7: a picked model is set at once, so the efforts answered are its own
/// before any turn; the session need not be open, and nothing is remembered.
#[tokio::test]
async fn picking_a_model_answers_that_models_efforts() {
    let app = test_app().await;
    let (s, c) = put_model(&app, "fake-small").await;
    assert_eq!(s, 200, "{c}");
    assert_eq!(c["current"]["model"], "fake-small");
    assert_eq!(names(&c["efforts"]), ["low", "high"]);
    assert_eq!(names(&c["modes"]), ["acceptEdits", "auto"]);

    let (s, c) = put_model(&app, "fake-tiny").await;
    assert_eq!((s, names(&c["efforts"])), (200, Vec::<String>::new()));
    let (_, again) = put_model(&app, "fake-tiny").await;
    assert_eq!(again, c, "the same model twice is the same state");
    let (_, opened) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(opened, c, "the session answers the model it now holds");

    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert!(h[0]["remembered"].is_null(), "only a turn is remembered");
    assert_eq!(app.sessions.live_count().await, 1);
}

#[tokio::test]
async fn a_model_the_session_does_not_list_is_not_offered() {
    let app = test_app().await;
    let (s, b) = put_model(&app, "retired-model").await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    let (_, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(
        c["current"]["model"], "fake-large",
        "the session is unchanged"
    );
}

#[tokio::test]
async fn a_model_the_harness_refuses_is_not_offered_in_its_words() {
    let app = test_app().await;
    let (s, b) = put_model(&app, "fake-locked").await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    let message = b["message"].as_str().unwrap();
    assert!(
        message.contains("Usage credits are required for this model"),
        "{message}"
    );
    let (_, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(c["current"]["model"], "fake-large");
}

#[tokio::test]
async fn a_running_turns_model_is_not_changed() {
    let app = test_app().await;
    let op = app::start_settled(&app, "hang").await;
    let (s, b) = put_model(&app, "fake-small").await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    let (s, _) = post(&app, &format!("/api/operations/{op}/stop"), json!({})).await;
    assert_eq!(s, 200);
    app::wait_terminal(&app, &op).await;
    let (s, c) = put_model(&app, "fake-small").await;
    assert_eq!(
        (s, c["current"]["model"].as_str()),
        (200, Some("fake-small"))
    );
}

#[tokio::test]
async fn an_unknown_thread_is_not_found() {
    let app = test_app().await;
    let (s, b) = post(
        &app,
        "/api/threads/00000000-0000-4000-8000-000000000000/session",
        json!({}),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (404, Some("INVALID_COMMAND")));
}
