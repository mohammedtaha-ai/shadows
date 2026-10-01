//! The effort a thread's session runs at (spec §12.4, §12.7): no `default`
//! asked of the adapter, an effort set as soon as it is picked, and the
//! effort remembered per model.

use std::path::Path;

use serde_json::{Value, json};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{App, call, get_json, names, post, start_and_finish, test_app, test_app_at};

fn session_path(app: &App) -> String {
    format!("/api/threads/{}/session", app.thread)
}

async fn put(app: &App, what: &str, value: &str) -> (u16, Value) {
    let path = format!("/api/threads/{}/session/{what}", app.thread);
    call(app, "PUT", &path, Some(json!({ what: value }))).await
}

fn current(c: &Value) -> (&str, Option<&str>) {
    (
        c["current"]["model"].as_str().unwrap(),
        c["current"]["effort"].as_str(),
    )
}

/// §12.4: `initialize` advertises the adapter's `recommendedValue`, under
/// which it offers no effort or model `default`. What the fake received is
/// what Shadows sent.
#[tokio::test]
async fn initialize_advertises_recommended_value() {
    let app = test_app().await;
    let done = start_and_finish(&app, "report", app::default_settings()).await;
    assert_eq!(done.status_kind, "Completed");
    let reply: Value = serde_json::from_str(&app::last_agent_entry(&app).await.body).unwrap();
    assert_eq!(
        reply["client_meta"],
        json!({ "jetbrains": { "air": { "version": 1, "capabilities": ["recommendedValue"] } } })
    );
}

/// §12.7: a picked effort is set at once and answered as `POST /session`
/// answers; twice is the same state, and nothing is remembered.
#[tokio::test]
async fn picking_an_effort_sets_it_at_once() {
    let app = test_app().await;
    let (s, c) = put(&app, "effort", "low").await;
    assert_eq!(s, 200, "{c}");
    assert_eq!(current(&c), ("fake-large", Some("low")));
    assert_eq!(names(&c["efforts"]), ["low", "high", "max"]);
    let (_, again) = put(&app, "effort", "low").await;
    assert_eq!(again, c, "the same effort twice is the same state");
    let (_, opened) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(opened, c, "the session answers the effort it now holds");
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert!(h[0]["remembered"].is_null(), "only a turn is remembered");
}

#[tokio::test]
async fn an_effort_the_model_does_not_offer_is_not_offered() {
    let app = test_app().await;
    let (s, b) = put(&app, "effort", "xhigh").await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    put(&app, "model", "fake-tiny").await;
    let (s, b) = put(&app, "effort", "low").await;
    assert_eq!(
        (s, b["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED")),
        "a model with no effort offers none"
    );
    let (_, c) = put(&app, "model", "fake-large").await;
    assert_eq!(current(&c), ("fake-large", Some("high")), "unchanged");
}

#[tokio::test]
async fn a_running_turns_effort_is_not_changed() {
    let app = test_app().await;
    let op = app::start_settled(&app, "hang").await;
    let (s, b) = put(&app, "effort", "low").await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    let (s, _) = post(&app, &format!("/api/operations/{op}/stop"), json!({})).await;
    assert_eq!(s, 200);
    app::wait_terminal(&app, &op).await;
    let (s, c) = put(&app, "effort", "low").await;
    assert_eq!((s, current(&c)), (200, ("fake-large", Some("low"))));
}

/// §12.4: a turn remembers its effort for its model. Choosing a model, or
/// opening a session, comes back at that model's effort; a model no turn ran
/// on starts at the harness's current value (the fake's `high`).
#[tokio::test]
async fn the_effort_is_remembered_per_model() {
    let app = test_app().await;
    let settings =
        |model, effort| json!({ "model": model, "mode": "acceptEdits", "effort": effort });
    let done = start_and_finish(&app, "hi", settings("fake-small", "low")).await;
    assert_eq!(done.status_kind, "Completed");
    let (_, c) = put(&app, "model", "fake-large").await;
    assert_eq!(current(&c), ("fake-large", Some("high")), "never used");

    let done = start_and_finish(&app, "hi", settings("fake-large", "max")).await;
    assert_eq!(done.status_kind, "Completed");
    let (_, c) = put(&app, "model", "fake-small").await;
    assert_eq!(current(&c), ("fake-small", Some("low")));
    let (_, c) = put(&app, "model", "fake-large").await;
    assert_eq!(current(&c), ("fake-large", Some("max")));

    // A new session opens at the last model and its effort.
    put(&app, "effort", "low").await;
    app.sessions.terminate(&app.thread).await.unwrap();
    let (_, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!(current(&c), ("fake-large", Some("max")));
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        h[0]["remembered"],
        json!({ "model": "fake-large", "effort": "max" })
    );
}

/// Migration 0011: the effort a harness remembered before moves to its
/// remembered model, and a new session still opens at both.
#[tokio::test]
async fn remembered_settings_survive_migration_0011() {
    let tmp = tempfile::tempdir().unwrap();
    seed_before_0011(tmp.path()).await;
    let app = test_app_at(tmp.path()).await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        h[0]["remembered"],
        json!({ "model": "fake-small", "effort": "low" })
    );
    let (s, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!((s, current(&c)), (200, ("fake-small", Some("low"))));
}

/// Migration 0011: an effort `default` was no level, so it is not carried; the
/// model's effort is then the adapter's.
#[tokio::test]
async fn a_remembered_default_effort_is_not_carried_by_migration_0011() {
    let tmp = tempfile::tempdir().unwrap();
    seed_before_0011_with(tmp.path(), "fake-small", "default").await;
    let app = test_app_at(tmp.path()).await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        h[0]["remembered"],
        json!({ "model": "fake-small", "effort": null })
    );
    let (s, c) = post(&app, &session_path(&app), json!({})).await;
    assert_eq!((s, current(&c)), (200, ("fake-small", Some("high"))));
}

async fn seed_before_0011(tmp: &Path) {
    seed_before_0011_with(tmp, "fake-small", "low").await;
}

/// Applies the migrations before 0011, from copies of the checked-in files so
/// their checksums match, and remembers `model` and `effort` as that schema did.
async fn seed_before_0011_with(tmp: &Path, model: &str, effort: &str) {
    let old = tmp.join("pre-0011");
    std::fs::create_dir(&old).unwrap();
    for entry in std::fs::read_dir(shadows_core::testing::migrations_dir()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if name.as_str() < "0011" {
            std::fs::copy(&path, old.join(&name)).unwrap();
        }
    }
    let db = tmp.join("s.sqlite3");
    let url = format!(
        "sqlite://{}?mode=rwc",
        db.display().to_string().replace('\\', "/")
    );
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::migrate::Migrator::new(old.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO harness_preference (harness_kind, model, effort, updated_at)
         VALUES ('claude-code', ?, ?, '2026-09-30T00:00:00Z')",
    )
    .bind(model)
    .bind(effort)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
}
