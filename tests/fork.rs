//! Forking a thread from its last completed entry (spec §12.9): a new thread
//! holding copies of the source's entries, whose first opening forks the
//! source's harness session; the source is unchanged.

use serde_json::{Value, json};
use shadows::events::Actor;
use shadows::operation::OperationId;
use shadows::planner::PlannerTurn;
use shadows::storage::StorageError;
use shadows::thread::ThreadId;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{
    App, default_settings, get_json, last_agent_entry_on, post, start_and_finish,
    start_and_finish_on, start_settled, test_app, wait_terminal,
};

async fn entries_json(app: &App, thread: &str) -> Vec<Value> {
    get_json(app, &format!("/api/threads/{thread}/entries")).await
}

async fn fork_last_raw_with(app: &App, command_id: &str) -> (u16, Value) {
    let src = entries_json(app, app.thread.as_str()).await;
    let last = src.last().unwrap()["id"].as_str().unwrap().to_string();
    post(
        app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({ "command_id": command_id, "at_entry_id": last }),
    )
    .await
}

async fn fork_last_raw(app: &App) -> (u16, Value) {
    fork_last_raw_with(app, "f1").await
}

async fn fork_last(app: &App) -> Value {
    let (s, fork) = fork_last_raw(app).await;
    assert_eq!(s, 201, "{fork}");
    fork
}

#[tokio::test]
async fn fork_copies_entries_keeps_their_operation_and_leaves_the_source_alone() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let src = entries_json(&app, app.thread.as_str()).await;
    let last = src.last().unwrap()["id"].as_str().unwrap().to_string();
    let (s, fork) = post(
        &app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({ "command_id": "f1", "at_entry_id": last }),
    )
    .await;
    assert_eq!(s, 201, "{fork}");
    assert_eq!(fork["forked_from_thread"], app.thread.as_str());
    assert_eq!(fork["title"], "T (fork)");
    assert_eq!(fork["harness"], "claude-code");
    let fork_id = fork["id"].as_str().unwrap();
    let copied = entries_json(&app, fork_id).await;
    assert_eq!(copied.len(), src.len());
    assert_ne!(copied[0]["id"], src[0]["id"]);
    for (c, s) in copied.iter().zip(&src) {
        assert_eq!(
            (&c["kind"], &c["body"], &c["ordinal"]),
            (&s["kind"], &s["body"], &s["ordinal"])
        );
        assert_eq!(c["thread_id"], fork_id);
    }
    assert_eq!(
        copied.last().unwrap()["operation_id"],
        src.last().unwrap()["operation_id"]
    );
    assert_eq!(
        entries_json(&app, app.thread.as_str()).await,
        src,
        "source unchanged"
    );
    let ops: Vec<Value> = get_json(&app, &format!("/api/threads/{fork_id}/operations")).await;
    assert!(ops.is_empty());
    let listed: Vec<Value> =
        get_json(&app, &format!("/api/projects/{}/threads", app.project)).await;
    assert_eq!(listed.len(), 2);
}

#[tokio::test]
async fn the_forks_first_opening_forks_the_source_session() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let src_session = app
        .storage
        .turn_context(&app.thread)
        .await
        .unwrap()
        .harness_session_id
        .unwrap();
    let fork = fork_last(&app).await;
    let fork_id = fork["id"].as_str().unwrap();
    let done = start_and_finish_on(&app, fork_id, "report", default_settings()).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let r: Value = serde_json::from_str(&last_agent_entry_on(&app, fork_id).await.body).unwrap();
    assert_eq!(r["how"], "fork");
    assert_eq!(r["session"], format!("fork-of-{src_session}"));
    let recorded = app
        .storage
        .turn_context(&ThreadId::from_literal(fork_id))
        .await
        .unwrap()
        .harness_session_id
        .unwrap();
    assert_eq!(recorded, format!("fork-of-{src_session}"));
    let again = start_and_finish(&app, "report", default_settings()).await;
    assert_eq!(again.status_kind, "Completed", "the source runs on");
}

#[tokio::test]
async fn only_the_last_completed_entry_is_a_fork_point() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let first = entries_json(&app, app.thread.as_str()).await[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (s, b) = post(
        &app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({ "command_id": "f1", "at_entry_id": first }),
    )
    .await;
    assert_eq!(
        (s, b["code"].as_str()),
        (422, Some("FORK_POINT_NOT_SUPPORTED"))
    );
}

#[tokio::test]
async fn an_entry_of_another_thread_is_not_found() {
    let app = test_app().await;
    let (s, b) = post(
        &app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({ "command_id": "f1", "at_entry_id": "00000000-0000-4000-8000-000000000000" }),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (404, Some("INVALID_COMMAND")));
}

#[tokio::test]
async fn a_stopped_last_turn_is_not_a_fork_point() {
    let app = test_app().await;
    let mut rx = app.bus.subscribe();
    let op = start_settled(&app, "hang").await;
    wait_for_delta(&mut rx).await;
    PlannerTurn::stop(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        &op,
        Actor::user("local"),
    )
    .await
    .unwrap();
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
    let (s, b) = fork_last_raw(&app).await;
    assert_eq!(
        (s, b["code"].as_str()),
        (422, Some("FORK_POINT_NOT_SUPPORTED"))
    );
}

#[tokio::test]
async fn fork_while_a_turn_runs_is_thread_busy_and_a_replay_returns_the_same_fork() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let a = fork_last(&app).await;
    let b = fork_last(&app).await; // same command_id "f1"
    assert_eq!(a["id"], b["id"]);
    let _running = start_settled(&app, "hang").await;
    let (s, body) = fork_last_raw_with(&app, "f2").await;
    assert_eq!((s, body["code"].as_str()), (409, Some("THREAD_BUSY")));
}

/// §12.6, §12.9: a fork's session is a fork of its source's, which no other
/// harness could continue, so its harness is locked from birth — before it
/// has an operation of its own — in the route, the storage call and the
/// trigger below both.
#[tokio::test]
async fn a_fork_is_locked_to_its_sources_harness_from_birth() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let fork = fork_last(&app).await;
    let fork_id = fork["id"].as_str().unwrap();
    let (s, b) = app::patch(
        &app,
        &format!("/api/threads/{fork_id}"),
        json!({ "command_id": "h1", "harness": "codex" }),
    )
    .await;
    assert_eq!(
        (s, b["code"].as_str()),
        (409, Some("HARNESS_LOCKED")),
        "{b}"
    );

    let thread = ThreadId::from_literal(fork_id);
    let direct = app
        .storage
        .set_thread_harness(&app::ctx("h2", "thread.harness"), &thread, "codex")
        .await;
    assert!(matches!(direct, Err(StorageError::HarnessLocked)));
    let raw = sqlx::query("UPDATE planning_thread SET harness_kind = 'codex' WHERE id = ?")
        .bind(fork_id)
        .execute(app.storage.reader())
        .await;
    assert!(raw.is_err(), "the trigger must refuse a raw update too");
    let ctx = app.storage.turn_context(&thread).await.unwrap();
    assert_eq!(ctx.harness, "claude-code");
}

async fn wait_for_delta(
    rx: &mut tokio::sync::broadcast::Receiver<(
        ThreadId,
        OperationId,
        shadows::agent::events::HarnessEvent,
    )>,
) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let (_, _, event) = rx.recv().await.unwrap();
            if matches!(event, shadows::agent::events::HarnessEvent::Chunk { .. }) {
                return;
            }
        }
    })
    .await
    .expect("the turn never streamed");
}
