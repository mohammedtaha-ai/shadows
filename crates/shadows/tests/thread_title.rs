//! A conversation is named after its first message, then by its harness, and
//! never over a name a person gave it (spec §4.2, §12.3).

use std::time::Duration;

use serde_json::Value;

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{App, default_settings, start_and_finish, test_app};

async fn title(app: &App) -> String {
    let threads = app
        .storage
        .list_threads_for_project(&app.project)
        .await
        .unwrap();
    let thread = threads.iter().find(|t| t.id == app.thread).unwrap();
    thread.title.clone()
}

async fn source(app: &App) -> String {
    sqlx::query_scalar("SELECT title_source FROM planning_thread WHERE id = ?")
        .bind(app.thread.as_str())
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

/// The `ThreadRetitled` events of the app's thread, oldest first.
async fn retitled(app: &App) -> Vec<Value> {
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM durable_event
          WHERE thread_id = ? AND kind = 'ThreadRetitled' ORDER BY seq",
    )
    .bind(app.thread.as_str())
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    payloads
        .iter()
        .map(|p| serde_json::from_str(p).unwrap())
        .collect()
}

/// Waits for the thread's title to read `want`: the harness sends its title
/// after the turn has answered, so it lands after the turn has ended.
async fn wait_title(app: &App, want: &str) {
    for _ in 0..100 {
        if title(app).await == want {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the title stayed {:?}, not {want:?}", title(app).await);
}

#[tokio::test]
async fn the_first_message_titles_the_thread_once() {
    let app = test_app().await;
    let long = "Plan   the migration of every table from the old schema to the new one";
    let done = start_and_finish(
        &app,
        &format!("\n  {long}\nsecond line"),
        default_settings(),
    )
    .await;
    assert_eq!(done.status_kind, "Completed");
    let first = title(&app).await;
    assert_eq!(first.chars().count(), 60, "{first}");
    assert!(
        first.starts_with("Plan the migration of every table"),
        "{first}"
    );
    assert!(first.ends_with('…'), "{first}");
    assert_eq!(source(&app).await, "first_message");

    start_and_finish(&app, "something else entirely", default_settings()).await;
    assert_eq!(title(&app).await, first, "a second message changes nothing");
    let events = retitled(&app).await;
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0]["title"], first.as_str());
    assert_eq!(events[0]["source"], "first_message");
}

#[tokio::test]
async fn the_harness_title_replaces_the_first_message_sanitized_and_once() {
    let app = test_app().await;
    let prompt = "title   A plan for\n\t the   migration  ";
    start_and_finish(&app, prompt, default_settings()).await;
    wait_title(&app, "A plan for the migration").await;
    assert_eq!(source(&app).await, "harness");

    // The same title again, from a second turn: nothing is written.
    start_and_finish(&app, prompt, default_settings()).await;
    // A later title on the same connection is written after it, so once this
    // one lands the repeat has been judged.
    start_and_finish(&app, "title Migration plan", default_settings()).await;
    wait_title(&app, "Migration plan").await;
    let events = retitled(&app).await;
    let kept: Vec<(&str, &str)> = events
        .iter()
        .map(|e| (e["title"].as_str().unwrap(), e["source"].as_str().unwrap()))
        .collect();
    assert_eq!(
        kept,
        [
            ("title A plan for", "first_message"),
            ("A plan for the migration", "harness"),
            ("Migration plan", "harness"),
        ]
    );
}

#[tokio::test]
async fn a_title_a_person_gave_is_never_replaced() {
    let app = test_app().await;
    // No rename exists yet: the row is set as one would set it.
    sqlx::query("UPDATE planning_thread SET title = 'Mine', title_source = 'person' WHERE id = ?")
        .bind(app.thread.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    start_and_finish(&app, "the first message", default_settings()).await;
    let changed = app
        .storage
        .title_from_harness(&app.thread, "From the harness")
        .await
        .unwrap();
    assert!(!changed);
    assert_eq!(
        (title(&app).await, source(&app).await),
        ("Mine".into(), "person".into())
    );
    assert!(retitled(&app).await.is_empty());
}
