//! Spec §14.9: the harness routes answer the same after moving into `Harness`,
//! and a model change keeps its order: the harness is checked, then the
//! thread's busyness, before any session is touched.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{App, call, create_thread, post, start_settled, test_app, wait_terminal};
use serde_json::{Value, json};

async fn put_model(app: &App, thread: &str, model: &str) -> (u16, Value) {
    let path = format!("/api/threads/{thread}/session/model");
    call(app, "PUT", &path, Some(json!({ "model": model }))).await
}

/// Busy, both checks (§14.9): a model change while a turn runs is THREAD_BUSY,
/// and the running turn's session keeps the model it holds. Stopping the turn
/// then records `Cancelled`.
#[tokio::test]
async fn change_model_is_refused_while_a_turn_runs() {
    let app = test_app().await;
    let op = start_settled(&app, "hang").await;
    let before = app.sessions.offered(&app.thread).await.unwrap();
    let (s, b) = put_model(&app, app.thread.as_str(), "fake-small").await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    let after = app.sessions.offered(&app.thread).await.unwrap();
    assert_eq!(
        after.current.model, before.current.model,
        "session untouched"
    );
    assert_eq!(after.current.model, "fake-large");
    let (s, b) = post(&app, &format!("/api/operations/{op}/stop"), json!({})).await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}

/// The early busy check comes before the session: a thread with an operation
/// not yet ended is refused without opening an adapter. Without it, nothing
/// holds the lease here, so the session would open and its model change.
#[tokio::test]
async fn change_model_on_a_busy_thread_opens_no_session() {
    let app = test_app().await;
    app.storage
        .create_pending_operation(&app.thread, &app.runtime.instance_id)
        .await
        .unwrap();
    let (s, b) = put_model(&app, app.thread.as_str(), "fake-small").await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    assert_eq!(app.sessions.live_count().await, 0, "no session was opened");
}

/// Harness availability (§14.9) is checked before any session work.
#[tokio::test]
async fn change_model_on_an_unavailable_harness_opens_no_session() {
    let app = test_app().await;
    let t = create_thread(
        &app,
        json!({ "command_id": "c9", "title": "x", "harness": "codex" }),
    )
    .await;
    let (s, b) = put_model(&app, t["id"].as_str().unwrap(), "fake-small").await;
    assert_eq!(
        (s, b["code"].as_str()),
        (422, Some("HARNESS_UNAVAILABLE")),
        "{b}"
    );
    assert_eq!(app.sessions.live_count().await, 0, "no session was opened");
}

/// Harness carries no command: opening a session, changing its model and
/// reading its context record nothing in the command log, so there is no
/// fingerprint for the move to shift.
#[tokio::test]
async fn harness_calls_record_no_command() {
    let app = test_app().await;
    let thread = app.thread.as_str().to_string();
    let count = || async {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_record")
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
        n
    };
    let before = count().await;
    let (s, b) = post(&app, &format!("/api/threads/{thread}/session"), json!({})).await;
    assert_eq!(s, 200, "{b}");
    let (s, b) = put_model(&app, &thread, "fake-small").await;
    assert_eq!(s, 200, "{b}");
    let (s, b) = call(&app, "GET", &format!("/api/threads/{thread}/context"), None).await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(count().await, before, "no command was recorded");
}
