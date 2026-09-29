//! Starting a turn as one command (spec §12.7): one transaction for the entry,
//! the operation, its invocation and the command record; a replay answered
//! before anything else; every check before any write; the session set to the
//! turn's settings before the prompt.

use serde_json::{Value, json};
use shadows_agent::TurnSettings;
use shadows_core::storage::StorageError;
use shadows_core::testing::FailureStage;
use shadows_core::testing::StartedTurn;
use shadows_core::threads::ThreadEntryKind;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/turn.rs"]
mod turn;

use app::{App, ctx, entries, http_start, last_agent_entry, test_app, wait_terminal};
use turn::{new_turn, turn_command};

fn small_edits() -> TurnSettings {
    TurnSettings {
        model: "fake-small".into(),
        mode: "acceptEdits".into(),
        effort: Some("high".into()),
    }
}

async fn start(
    app: &App,
    id: &str,
    prompt: &str,
    settings: &TurnSettings,
) -> Result<StartedTurn, StorageError> {
    app.storage
        .start_turn(
            &turn_command(id, &app.thread, prompt, settings),
            new_turn(&app.thread, &app.runtime, prompt, settings),
        )
        .await
}

async fn patch_modes(app: &App, modes: &[&str]) {
    let modes = [(
        "claude-code".to_string(),
        modes.iter().map(|m| m.to_string()).collect(),
    )]
    .into();
    app.storage
        .set_project_modes(&ctx("modes", "project.modes"), &app.project, &modes)
        .await
        .unwrap();
}

fn thread(app: &App) -> String {
    app.thread.as_str().to_string()
}

#[tokio::test]
async fn one_command_writes_entry_operation_invocation_and_record_together() {
    let app = test_app().await;
    let started = start(&app, "t1", "hello", &small_edits()).await.unwrap();
    assert!(!started.replayed);
    let entries = entries(&app).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, started.entry_id);
    assert_eq!(
        entries[0].operation_id.as_ref(),
        Some(&started.operation_id)
    );
    let ops = app
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert_eq!(ops[0].status_kind, "Pending");
    let inv = ops[0]
        .invocation
        .as_ref()
        .expect("invocation written with Pending");
    assert_eq!(
        (
            inv.requested_model.as_str(),
            inv.requested_mode.as_str(),
            inv.requested_effort.as_deref()
        ),
        ("fake-small", "acceptEdits", Some("high"))
    );
    assert_eq!(
        (inv.harness_version.as_str(), inv.agent_version.as_str()),
        ("fake-adapter-1", "fake-claude-1")
    );
    assert!(inv.observed_model.is_none());
    assert_eq!(
        app.storage
            .remembered_settings("claude-code")
            .await
            .unwrap(),
        Some(("fake-small".into(), Some("high".into())))
    );
}

#[tokio::test]
async fn the_invocation_is_frozen_below_the_application() {
    let app = test_app().await;
    let started = start(&app, "t1", "hello", &small_edits()).await.unwrap();
    let raw =
        sqlx::query("UPDATE agent_invocation SET requested_model = 'x' WHERE operation_id = ?")
            .bind(started.operation_id.as_str())
            .execute(app.storage.reader())
            .await;
    assert!(raw.is_err(), "the trigger refuses a changed request");
}

#[tokio::test]
async fn a_replayed_turn_start_starts_nothing_and_returns_the_first_operation() {
    let app = test_app().await;
    let first = start(&app, "t1", "hello", &small_edits()).await.unwrap();
    app.storage
        .mark_operation_failed(&first.operation_id, FailureStage::Prepare, "test")
        .await
        .unwrap();
    let again = start(&app, "t1", "hello", &small_edits()).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.operation_id, first.operation_id);
    assert_eq!(again.entry_id, first.entry_id);
    assert_eq!(entries(&app).await.len(), 1);
    let ops = app
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert_eq!(ops.len(), 1);
}

#[tokio::test]
async fn the_same_command_id_with_another_body_is_a_conflict() {
    let app = test_app().await;
    start(&app, "t1", "hello", &small_edits()).await.unwrap();
    let large = TurnSettings {
        model: "fake-large".into(),
        ..small_edits()
    };
    let other = start(&app, "t1", "hello", &large).await;
    assert!(matches!(other, Err(StorageError::CommandConflict)));
}

#[tokio::test]
async fn a_second_turn_while_one_is_running_is_thread_busy() {
    let app = test_app().await;
    start(&app, "t1", "hello", &small_edits()).await.unwrap();
    assert!(matches!(
        start(&app, "t2", "again", &small_edits()).await,
        Err(StorageError::ThreadBusy)
    ));
}

#[tokio::test]
async fn a_second_turn_over_http_while_one_runs_is_thread_busy_and_writes_nothing() {
    let app = test_app().await;
    let _running = app::start_settled(&app, "hang").await;
    let (s, b) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t2", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "high" }),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (409, Some("THREAD_BUSY")));
    assert_eq!(
        entries(&app).await.len(),
        1,
        "only the running turn's prompt"
    );
}

#[tokio::test]
async fn a_mode_the_project_no_longer_allows_is_refused() {
    let app = test_app().await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (status, body) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "auto", "effort": "high" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (403, Some("MODE_NOT_ALLOWED"))
    );
    assert!(entries(&app).await.is_empty(), "nothing durable");
}

#[tokio::test]
async fn a_mode_outside_the_policy_is_not_offered() {
    let app = test_app().await;
    let (status, body) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "bypassPermissions", "effort": "high" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED"))
    );
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn an_effort_the_model_does_not_offer_is_refused() {
    let app = test_app().await;
    let (status, body) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "max" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED"))
    );
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn a_model_the_account_cannot_use_is_refused_with_the_harness_message() {
    let app = test_app().await;
    let (status, body) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "hi", "model": "fake-locked", "mode": "acceptEdits", "effort": null }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED"))
    );
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Usage credits are required"),
        "{body}"
    );
    assert!(entries(&app).await.is_empty());
}

#[tokio::test]
async fn a_model_without_efforts_runs_with_none() {
    let app = test_app().await;
    let (s, b) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "report", "model": "fake-tiny", "mode": "acceptEdits", "effort": null }),
    )
    .await;
    assert_eq!(s, 202, "{b}");
    let op = shadows_core::OperationId::from_literal(b["operation_id"].as_str().unwrap());
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Completed");
    let r: Value = serde_json::from_str(&last_agent_entry(&app).await.body).unwrap();
    assert_eq!(
        (r["model"].as_str(), r["effort"].is_null()),
        (Some("fake-tiny"), true)
    );
    let ops = app
        .storage
        .list_operations_for_thread(&app.thread)
        .await
        .unwrap();
    assert!(
        ops[0]
            .invocation
            .clone()
            .unwrap()
            .requested_effort
            .is_none()
    );
}

#[tokio::test]
async fn auto_on_a_model_without_it_fails_at_prepare_with_the_harness_message() {
    let app = test_app().await;
    let (s, b) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "auto", "effort": "high" }),
    )
    .await;
    assert_eq!(s, 202, "{b}");
    let op = shadows_core::OperationId::from_literal(b["operation_id"].as_str().unwrap());
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.status_kind, "Failed");
    assert_eq!(done.failure_stage.as_deref(), Some("Prepare"));
    assert!(
        done.failure_reason
            .unwrap()
            .contains("auto mode is not available")
    );
}

#[tokio::test]
async fn a_replay_is_answered_even_after_the_mode_was_disallowed() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "auto", "effort": "high" });
    let (s1, b1) = http_start(&app, &thread(&app), body.clone()).await;
    assert_eq!(s1, 202, "{b1}");
    let op = shadows_core::OperationId::from_literal(b1["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (s2, b2) = http_start(&app, &thread(&app), body).await;
    assert_eq!((s2, &b2["operation_id"]), (202, &b1["operation_id"]));
    assert_eq!(
        entries(&app)
            .await
            .iter()
            .filter(|e| e.kind == ThreadEntryKind::UserMessage)
            .count(),
        1
    );
}

#[tokio::test]
async fn a_replay_with_another_body_over_http_is_a_command_conflict() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "acceptEdits", "effort": "high" });
    let (_, b1) = http_start(&app, &thread(&app), body).await;
    let op = shadows_core::OperationId::from_literal(b1["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;
    let (s, b) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "other", "model": "fake-large", "mode": "acceptEdits", "effort": "high" }),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (409, Some("COMMAND_CONFLICT")));
}

#[tokio::test]
async fn a_replay_is_answered_while_the_daemon_is_stopping() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "high" });
    let (_, first) = http_start(&app, &thread(&app), body.clone()).await;
    let op = shadows_core::OperationId::from_literal(first["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;
    app.handles.close_for_test().await; // what shutdown does first
    let (s, again) = http_start(&app, &thread(&app), body).await;
    assert_eq!((s, &again["operation_id"]), (202, &first["operation_id"]));
    let (s2, b2) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t2", "prompt": "new", "model": "fake-small", "mode": "acceptEdits", "effort": "high" }),
    )
    .await;
    assert_eq!((s2, b2["code"].as_str()), (503, Some("RUNTIME_STOPPING")));
}

#[tokio::test]
async fn the_harness_runs_with_the_chosen_model_mode_and_effort() {
    let app = test_app().await;
    let (_, b) = http_start(
        &app,
        &thread(&app),
        json!({ "command_id": "t1", "prompt": "report", "model": "fake-large", "mode": "auto", "effort": "max" }),
    )
    .await;
    let op = shadows_core::OperationId::from_literal(b["operation_id"].as_str().unwrap());
    wait_terminal(&app, &op).await;
    let r: Value = serde_json::from_str(&last_agent_entry(&app).await.body).unwrap();
    assert_eq!(
        (
            r["model"].as_str(),
            r["mode"].as_str(),
            r["effort"].as_str()
        ),
        (Some("fake-large"), Some("auto"), Some("max"))
    );
    let every = entries(&app).await;
    assert!(
        every.iter().all(|e| e.operation_id.as_ref() == Some(&op)),
        "the prompt and every entry the turn wrote name the turn"
    );
}

/// A Stop that lands while the committed turn is still `Pending` finds nothing
/// registered to cancel (`NotLive`), but its request is durable: the turn
/// sees it once registered and ends `Cancelled` without prompting the model.
#[tokio::test]
async fn a_stop_while_pending_cancels_the_turn_before_its_prompt() {
    use shadows_core::events::Actor;
    use shadows_core::testing::{PlannerTurn, PlannerTurnRequest, StopOutcome};
    let app = test_app().await;
    let opened = app.sessions.open(&app.thread).await.unwrap();
    let events = app
        .sessions
        .lease_events(&app.thread, &opened)
        .await
        .unwrap();
    let settings = turn::default_turn_settings();
    let started = start(&app, "t1", "hello", &settings).await.unwrap();
    let op = started.operation_id;
    let stop = PlannerTurn::stop(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        &op,
        Actor::user("local"),
    )
    .await
    .unwrap();
    assert_eq!(stop, StopOutcome::NotLive);
    PlannerTurn::start(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        opened,
        PlannerTurnRequest {
            thread_id: app.thread.clone(),
            harness: "claude-code".into(),
            operation_id: op.clone(),
            prompt: "hello".into(),
            settings,
            focus: None,
            client_tab: None,
            events,
        },
        app.bus.clone(),
    )
    .await
    .unwrap();
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
    let kinds: Vec<_> = entries(&app).await.into_iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [ThreadEntryKind::UserMessage],
        "nothing reached the model"
    );
    let next = wait_terminal(&app, &app::start_settled(&app, "hi").await).await;
    assert_eq!(next.status_kind, "Completed", "the session was given back");
}
