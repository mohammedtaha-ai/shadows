//! The plan in the conversation (spec §13.9): a turn can point at a task,
//! which the `UserMessage` keeps and the Planner is told in a context block;
//! the Planner's `plan_show` writes a `PlanView` card and, live only, signals
//! the tab that sent the turn with a `plan-show` frame.

use std::time::Duration;

use serde_json::{Value, json};
use shadows::operation::OperationId;
use shadows::thread::{EntryRef, ThreadEntryKind};
use shadows::workflow::{TaskId, WorkflowId};

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{
    App, default_settings, entries, frames_until, fresh_command, http_start, last_agent_entry,
    next_frame_named, subscribe, subscribe_from, wait_terminal,
};
use listening::{Listening, call, listening_app, project_client, refused, thread_client};
use plan::{add, approved_v1, draft, draft_on, edit, events_of};

/// v1 of the app's thread with T1..=`n`, at revision 1.
async fn plan_with(app: &App, n: u32) -> WorkflowId {
    let v1 = draft(app).await.workflow_id;
    let ops: Vec<_> = (1..=n).map(add).collect();
    edit(app, &v1, 0, &ops).await;
    v1
}

async fn task_id(app: &App, workflow: &WorkflowId, number: u32) -> TaskId {
    let plan = app.storage.get_plan(workflow).await.unwrap();
    let task = plan.tasks.iter().find(|t| t.content.number == number);
    task.expect("the task").id.clone()
}

fn focus(workflow: &WorkflowId, task: &TaskId, revision: i64) -> Value {
    json!({ "workflow_id": workflow, "task_id": task, "revision": revision })
}

/// A turn body with the fake's settings, `prompt`, and `extra`'s fields.
fn turn(command: &str, prompt: &str, extra: Value) -> Value {
    let mut body = default_settings();
    body["command_id"] = json!(command);
    body["prompt"] = json!(prompt);
    for (key, value) in extra.as_object().unwrap() {
        body[key] = value.clone();
    }
    body
}

/// Starts `body` on the app's thread, asserting it was accepted.
async fn start(app: &App, body: Value) -> OperationId {
    let (status, answer) = http_start(app, app.thread.as_str(), body).await;
    assert_eq!(status, 202, "{answer}");
    OperationId::from_literal(answer["operation_id"].as_str().unwrap())
}

async fn wait_running(app: &App, op: &OperationId) {
    for _ in 0..200 {
        if app.storage.get_operation(op).await.unwrap().status_kind == "Running" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the turn did not start running");
}

async fn release(l: &Listening, op: &OperationId) {
    let context = l.app.storage.turn_context(&l.app.thread).await.unwrap();
    let dir = context.project_directory.expect("a project directory");
    std::fs::write(dir.join("release"), "").unwrap();
    assert_eq!(wait_terminal(&l.app, op).await.status_kind, "Completed");
}

#[tokio::test]
async fn a_focused_turn_stores_the_task_with_the_message_and_tells_the_planner() {
    let l = listening_app().await;
    let v1 = plan_with(&l.app, 2).await;
    let t2 = task_id(&l.app, &v1, 2).await;

    let op = start(
        &l.app,
        turn("c1", "report", json!({ "focus": focus(&v1, &t2, 1) })),
    )
    .await;
    assert_eq!(wait_terminal(&l.app, &op).await.status_kind, "Completed");

    let report: Value = serde_json::from_str(&last_agent_entry(&l.app).await.body).unwrap();
    let blocks: Vec<&str> = (report["blocks"].as_array().unwrap().iter())
        .map(|b| b.as_str().unwrap())
        .collect();
    let told = format!(
        "[Shadows] The person is pointing at task T2 (\"task 2\") of plan {v1}, revision 1. \
         Read the plan with workflow_get before changing it."
    );
    assert_eq!(
        blocks,
        ["report", told.as_str()],
        "the person's text is first"
    );

    let message = entries(&l.app)
        .await
        .into_iter()
        .find(|e| e.kind == ThreadEntryKind::UserMessage);
    let message = message.expect("the person's message");
    assert_eq!(message.body, "report");
    assert_eq!(message.refs, [EntryRef::Workflow(v1), EntryRef::Task(t2)]);
}

#[tokio::test]
async fn the_same_command_with_another_focus_is_a_command_conflict() {
    let l = listening_app().await;
    let v1 = plan_with(&l.app, 4).await;
    let (t3, t4) = (task_id(&l.app, &v1, 3).await, task_id(&l.app, &v1, 4).await);

    let first = turn(
        "same",
        "change this",
        json!({ "focus": focus(&v1, &t3, 1) }),
    );
    let op = start(&l.app, first.clone()).await;
    wait_terminal(&l.app, &op).await;
    // The same focus is a replay, whichever tab sends it: the tab is transport.
    let mut again = first;
    again["client_tab"] = json!("tab-b");
    assert_eq!(start(&l.app, again).await, op);

    let other = turn(
        "same",
        "change this",
        json!({ "focus": focus(&v1, &t4, 1) }),
    );
    let (status, body) = http_start(&l.app, l.app.thread.as_str(), other).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "COMMAND_CONFLICT");
}

#[tokio::test]
async fn a_focus_on_a_task_of_another_plan_is_refused() {
    let l = listening_app().await;
    let v1 = plan_with(&l.app, 1).await;
    let (_, other_thread) = app::other_project(&l.app).await;
    let theirs = draft_on(&l.app, &other_thread, "start-other")
        .await
        .workflow_id;
    edit(&l.app, &theirs, 0, &[add(1)]).await;
    let their_t1 = task_id(&l.app, &theirs, 1).await;
    let before = entries(&l.app).await.len();

    for (n, (workflow, task)) in [(&v1, &their_t1), (&theirs, &their_t1)]
        .into_iter()
        .enumerate()
    {
        let body = turn(
            &fresh_command(),
            "change this",
            json!({ "focus": focus(workflow, task, 1) }),
        );
        let (status, answer) = http_start(&l.app, l.app.thread.as_str(), body).await;
        assert_eq!(status, 422, "case {n}: {answer}");
        assert_eq!(answer["code"], "INVALID_COMMAND", "case {n}");
    }
    let (_, answer) = http_start(
        &l.app,
        l.app.thread.as_str(),
        turn("c9", "x", json!({ "focus": focus(&v1, &their_t1, 1) })),
    )
    .await;
    assert_eq!(answer["message"], "the chosen task is not in that plan");
    assert_eq!(entries(&l.app).await.len(), before, "nothing was written");
    let turns = l
        .app
        .storage
        .list_operations_for_thread(&l.app.thread)
        .await;
    assert!(turns.unwrap().is_empty());
}

#[tokio::test]
async fn plan_show_writes_a_card_and_signals_only_the_sending_tab() {
    let l = listening_app().await;
    let v1 = plan_with(&l.app, 1).await;
    let t1 = task_id(&l.app, &v1, 1).await;
    let mut tab_a = subscribe(&l.app, &l.app.thread).await;
    let mut tab_b = subscribe(&l.app, &l.app.thread).await;

    let extra = json!({ "focus": focus(&v1, &t1, 1), "client_tab": "tab-a" });
    let op = start(
        &l.app,
        turn("c1", r#"mcp plan_show {"place":"page"}"#, extra),
    )
    .await;
    let done = wait_terminal(&l.app, &op).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let shown: Value = serde_json::from_str(&last_agent_entry(&l.app).await.body).unwrap();
    assert_eq!(shown["version"], 1, "{shown}");

    for sub in [&mut tab_a, &mut tab_b] {
        let frames = frames_until(sub, |event, data| {
            event == "durable" && data["kind"] == "OperationCompleted"
        })
        .await;
        let card = frames
            .iter()
            .position(|(e, d)| e == "durable" && d["kind"] == "PlanShown")
            .expect("the PlanShown event");
        let signals: Vec<usize> = (frames.iter().enumerate())
            .filter(|(_, (e, _))| e == "plan-show")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(signals.len(), 1, "one plan-show frame: {frames:?}");
        assert!(signals[0] > card, "the card is journaled before the signal");
        let signal = &frames[signals[0]].1;
        assert_eq!(signal["target_tab"], "tab-a");
        assert_eq!(signal["thread_id"], l.app.thread.as_str());
        assert_eq!(signal["workflow_id"], v1.as_str());
        assert_eq!(signal["version"], 1);
        assert_eq!(signal["task_number"], Value::Null);
        assert_eq!(signal["place"], "page");
    }

    let card = entries(&l.app)
        .await
        .into_iter()
        .find(|e| e.kind == ThreadEntryKind::PlanView);
    let card = card.expect("a PlanView entry");
    assert_eq!(card.body, "Plan v1");
    assert_eq!(card.refs, [EntryRef::Workflow(v1.clone())]);
    assert_eq!(card.operation_id.as_ref(), Some(&op));
    let recorded = events_of(&l.app, &l.app.thread, "PlanShown").await;
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].get("target_tab").is_none(), "{}", recorded[0]);
    assert!(recorded[0].get("client_tab").is_none(), "{}", recorded[0]);
}

#[tokio::test]
async fn a_replayed_subscription_gets_the_card_but_no_plan_show_frame() {
    let l = listening_app().await;
    plan_with(&l.app, 1).await;
    let body = turn(
        "c1",
        r#"mcp plan_show {"place":"side","task_number":1}"#,
        json!({ "client_tab": "tab-a" }),
    );
    let op = start(&l.app, body).await;
    assert_eq!(wait_terminal(&l.app, &op).await.status_kind, "Completed");

    let mut later = subscribe_from(&l.app, &l.app.thread, 0).await;
    let frames = frames_until(&mut later, |event, _| event == "caught-up").await;
    assert!(
        frames
            .iter()
            .any(|(e, d)| e == "durable" && d["kind"] == "PlanShown"),
        "{frames:?}"
    );
    assert!(frames.iter().all(|(e, _)| e != "plan-show"), "{frames:?}");
    // Nothing live follows: the signal went out before this client came.
    let quiet = tokio::time::timeout(
        Duration::from_millis(300),
        next_frame_named(&mut later, "plan-show"),
    )
    .await;
    assert!(quiet.is_err(), "a replayed card never signals: {quiet:?}");
}

#[tokio::test]
async fn plan_show_twice_in_one_turn_for_different_tasks_writes_two_cards() {
    let l = listening_app().await;
    let v1 = plan_with(&l.app, 2).await;
    let planner = thread_client(&l, &l.app.thread).await;
    let idle = refused(&planner, "plan_show", json!({ "place": "inline" })).await;
    assert!(idle.starts_with("INVALID_COMMAND: "), "{idle}");
    let mut signals = l.app.ui.subscribe();

    let op = start(&l.app, turn("c1", "wait-for-release", json!({}))).await;
    wait_running(&l.app, &op).await;
    let first = json!({ "place": "inline", "task_number": 1 });
    let (is_error, one) = call(&planner, "plan_show", first.clone()).await;
    assert!(!is_error, "{one}");
    let (_, two) = call(
        &planner,
        "plan_show",
        json!({ "place": "inline", "task_number": 2 }),
    )
    .await;
    let (_, again) = call(&planner, "plan_show", first).await;
    assert_eq!(again, one, "a retry answers the first call");
    let missing = refused(
        &planner,
        "plan_show",
        json!({ "place": "inline", "task_number": 9 }),
    )
    .await;
    assert!(
        missing.contains("T9 does not exist in this plan"),
        "{missing}"
    );
    release(&l, &op).await;

    let cards: Vec<_> = (entries(&l.app).await.into_iter())
        .filter(|e| e.kind == ThreadEntryKind::PlanView)
        .collect();
    let bodies: Vec<&str> = cards.iter().map(|c| c.body.as_str()).collect();
    assert_eq!(bodies, ["T1 · task 1", "T2 · task 2"], "{two}");
    let (t1, t2) = (task_id(&l.app, &v1, 1).await, task_id(&l.app, &v1, 2).await);
    assert_eq!(
        cards[0].refs,
        [EntryRef::Workflow(v1.clone()), EntryRef::Task(t1)]
    );
    assert_eq!(
        cards[1].refs,
        [EntryRef::Workflow(v1.clone()), EntryRef::Task(t2)]
    );
    assert!(cards.iter().all(|c| c.operation_id.as_ref() == Some(&op)));

    let mut sent = Vec::new();
    while let Ok(signal) = signals.try_recv() {
        sent.push((signal.task_number, signal.target_tab));
    }
    assert_eq!(
        sent,
        [(Some(1), None), (Some(2), None)],
        "a replay sends nothing"
    );
}

#[tokio::test]
async fn an_external_grant_has_no_plan_show() {
    let l = listening_app().await;
    plan_with(&l.app, 1).await;
    let (_, external) = project_client(&l).await;
    let params = rmcp::model::CallToolRequestParams::new("plan_show")
        .with_arguments(json!({ "place": "page" }).as_object().unwrap().clone());
    let error = external.call_tool(params).await.unwrap_err();
    assert!(error.to_string().contains("tool not found"), "{error}");
    let listed = external.list_all_tools().await.unwrap();
    assert!(listed.iter().all(|t| t.name != "plan_show"));
    assert!(
        events_of(&l.app, &l.app.thread, "PlanShown")
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn a_thread_grant_can_read_and_show_its_older_version_but_not_another_thread() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let v2 = draft_on(&l.app, &l.app.thread, "start-2").await.workflow_id;
    let other = l
        .app
        .storage
        .create_planning_thread(
            &app::ctx("other-thread-in-project", "thread.create"),
            &l.app.project,
            "Other",
            shadows_agent::policy::CLAUDE_CODE,
        )
        .await
        .unwrap();
    let other_plan = draft_on(&l.app, &other.id, "other-plan").await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;

    let old = listening::ok(&planner, "workflow_get", json!({ "workflow_id": v1 })).await;
    assert_eq!(old["version"], 1);
    let old_task = listening::ok(
        &planner,
        "task_get",
        json!({ "workflow_id": v1, "number": 1 }),
    )
    .await;
    assert_eq!(old_task["title"], "task 1");
    let latest = listening::ok(&planner, "workflow_get", json!({})).await;
    assert_eq!(latest["id"], v2.as_str());
    let stale_edit = refused(
        &planner,
        "plan_edit",
        json!({ "workflow_id": v1, "expected_revision": 1, "ops": [] }),
    )
    .await;
    assert!(stale_edit.starts_with("GRANT_SCOPE: "), "{stale_edit}");
    let outside = refused(
        &planner,
        "workflow_get",
        json!({ "workflow_id": other_plan }),
    )
    .await;
    assert!(outside.starts_with("GRANT_SCOPE: "), "{outside}");

    let op = start(&l.app, turn("show-old", "wait-for-release", json!({}))).await;
    wait_running(&l.app, &op).await;
    let shown = listening::ok(
        &planner,
        "plan_show",
        json!({ "workflow_id": v1, "place": "inline" }),
    )
    .await;
    assert_eq!(shown["workflow_id"], v1.as_str());
    assert_eq!(shown["version"], 1);
    let outside = refused(
        &planner,
        "plan_show",
        json!({ "workflow_id": other_plan, "place": "inline" }),
    )
    .await;
    assert!(outside.starts_with("GRANT_SCOPE: "), "{outside}");
    release(&l, &op).await;

    let cards: Vec<_> = entries(&l.app)
        .await
        .into_iter()
        .filter(|entry| entry.kind == ThreadEntryKind::PlanView)
        .collect();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].body, "Plan v1");
    assert_eq!(cards[0].refs, [EntryRef::Workflow(v1)]);
}
