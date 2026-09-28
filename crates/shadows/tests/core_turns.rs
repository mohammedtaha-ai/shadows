//! Spec §14.9: starting and stopping a turn answer the same after moving into
//! `Turns`, and `send` keeps its order: the harness is checked, then the
//! thread's busyness, before any session is touched.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{
    App, create_thread, default_settings, http_start, post, start_settled, test_app, wait_terminal,
};
use plan::{add, draft, edit};
use serde_json::{Value, json};
use shadows_core::OperationId;
use shadows_core::command::fingerprint;

/// A turn body with the fake's own settings.
fn turn(command: &str, prompt: &str) -> Value {
    let mut body = default_settings();
    body["command_id"] = json!(command);
    body["prompt"] = json!(prompt);
    body
}

async fn stop(app: &App, op: &str) -> (u16, Value) {
    post(app, &format!("/api/operations/{op}/stop"), json!({})).await
}

#[tokio::test]
async fn turn_start_replays_after_the_move() {
    let app = test_app().await;
    let thread = app.thread.as_str().to_string();
    let (s1, first) = http_start(&app, &thread, turn("same", "hello")).await;
    let (s2, again) = http_start(&app, &thread, turn("same", "hello")).await;
    assert_eq!((s1, s2), (202, 202), "{first} {again}");
    assert_eq!(first["operation_id"], again["operation_id"]);
    let ops = app
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert_eq!(ops.len(), 1, "one command, one turn");
}

#[tokio::test]
async fn stop_after_the_move_records_cancelled() {
    let app = test_app().await;
    let op = start_settled(&app, "hang").await;
    let (status, answer) = stop(&app, op.as_str()).await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["id"], op.as_str());
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}

#[tokio::test]
async fn a_second_start_while_busy_is_thread_busy() {
    let app = test_app().await;
    let op = start_settled(&app, "hang").await;
    let (s, b) = http_start(&app, app.thread.as_str(), turn("second", "again")).await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    stop(&app, op.as_str()).await;
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}

/// The early busy check (§14.9) comes before the session: a thread whose turn
/// has not ended is refused without opening an adapter. Without it the start
/// would open the session and lease it, and only the transaction's own check
/// would refuse it.
#[tokio::test]
async fn a_busy_thread_is_refused_before_its_session_opens() {
    let app = test_app().await;
    app.storage
        .create_pending_operation(&app.thread, &app.runtime.instance_id)
        .await
        .unwrap();
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-small",
        "mode": "acceptEdits", "effort": "high" });
    let (s, b) = http_start(&app, app.thread.as_str(), body).await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")), "{b}");
    assert_eq!(app.sessions.live_count().await, 0, "no session was opened");
}

/// Harness availability (§14.9) is checked before any session work.
#[tokio::test]
async fn an_unavailable_harness_opens_no_session() {
    let app = test_app().await;
    let t = create_thread(
        &app,
        json!({ "command_id": "c9", "title": "x", "harness": "codex" }),
    )
    .await;
    let (s, b) = http_start(&app, t["id"].as_str().unwrap(), turn("t1", "hi")).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
    assert_eq!(app.sessions.live_count().await, 0, "no session was opened");
}

/// A start refused after the lease gives the session's events back, so the
/// next turn runs at once instead of waiting out the lease and being busy.
#[tokio::test]
async fn a_refused_start_gives_the_session_back() {
    let app = test_app().await;
    let mut body = turn("t1", "hi");
    body["model"] = json!("retired-model");
    let (s, b) = http_start(&app, app.thread.as_str(), body).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    assert_eq!(app.sessions.live_count().await, 1, "the session was opened");
    let op = start_settled(&app, "hi").await;
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Completed");
}

/// A replayed turn start matches a command an earlier daemon recorded only
/// when its principal, kind, schema version and fingerprint parameters are the
/// ones that daemon used. This reads the command log and compares it with the
/// parameters as the route before the move wrote them (§14.9): `thread_id`,
/// `prompt`, `model`, `mode`, `effort`, and `focus` only when one is given.
#[tokio::test]
async fn turn_command_fingerprints_do_not_move() {
    let app = test_app().await;
    let thread = app.thread.as_str().to_string();
    let (s, b) = http_start(&app, &thread, turn("pin-plain", "hi")).await;
    assert_eq!(s, 202, "{b}");
    let op = OperationId::from_literal(b["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;

    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add(1)]).await;
    let plan = app.storage.get_plan(&v1).await.unwrap();
    let focus = json!({ "workflow_id": v1, "task_id": plan.tasks[0].id, "revision": 1 });
    let mut body = turn("pin-focus", "look");
    body["focus"] = focus.clone();
    let (s, b) = http_start(&app, &thread, body).await;
    assert_eq!(s, 202, "{b}");
    let op = OperationId::from_literal(b["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;

    let mut recorded: Vec<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT principal_kind, principal_id, command_id, command_kind, command_schema_ver,
                request_fingerprint
           FROM command_record
          WHERE command_kind = 'turn.start'",
    )
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    recorded.sort();
    let pinned = |id: &str, params: Value| {
        let (user, local) = ("User".to_string(), "local".to_string());
        let kind = "turn.start".to_string();
        let fp = fingerprint(&kind, &params);
        (user, local, id.to_string(), kind, 1, fp)
    };
    let plain = json!({ "thread_id": thread, "prompt": "hi", "model": "fake-large",
        "mode": "acceptEdits", "effort": "high" });
    let mut focused = json!({ "thread_id": thread, "prompt": "look", "model": "fake-large",
        "mode": "acceptEdits", "effort": "high" });
    focused["focus"] = focus;
    let mut expected = vec![pinned("pin-plain", plain), pinned("pin-focus", focused)];
    expected.sort();
    assert_eq!(recorded, expected);
}
