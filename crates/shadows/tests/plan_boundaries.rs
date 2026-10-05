//! Project plan write boundaries (§16): archive state is checked in the
//! transaction, and writer attribution does not limit a conversation's reach.

use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{OperationId, WorkflowState};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, http_start, test_app, wait_terminal};
use plan::{add, draft, edit, edit_ctx, planner, writer_ctx};

#[tokio::test]
async fn an_archive_blocks_every_new_version_write_in_storage() {
    let app = test_app().await;
    let v1 = draft(&app).await;
    edit(&app, &v1.workflow_id, 0, &[add(1)]).await;
    let writer = planner(&app, &app.thread).await;
    app.core
        .plans()
        .archive("archive".into(), &v1.plan_id)
        .await
        .unwrap();
    // These calls represent writes already scoped before the archive committed.
    let edited = app
        .storage
        .edit_plan(
            &edit_ctx(&writer, &v1.workflow_id, 1, &[add(2)]),
            &writer,
            None,
            &v1.workflow_id,
            1,
            &[add(2)],
        )
        .await;
    let started = app
        .storage
        .start_draft(
            &writer_ctx(&writer, "late-start", "DraftStart", json!({})),
            &writer,
            &app.project,
            Some(&v1.plan_id),
            None,
            None,
            Some("the API changed"),
            None,
            None,
        )
        .await;
    let approved = app
        .core
        .plans()
        .approve("late-approve".into(), &v1.workflow_id, 1)
        .await;
    assert!(
        edited
            .as_ref()
            .is_err_and(|e| e.to_string().contains("archived")),
        "{edited:?}"
    );
    assert!(
        started
            .as_ref()
            .is_err_and(|e| e.to_string().contains("archived")),
        "{started:?}"
    );
    assert!(
        matches!(
            approved,
            Err(shadows_core::CoreError::Refused {
                code: shadows_core::ErrorCode::InvalidCommand,
                ..
            })
        ),
        "{approved:?}"
    );
    let after = app.storage.get_plan(&v1.workflow_id).await.unwrap();
    assert_eq!(
        (after.revision, after.state, after.tasks.len()),
        (1, WorkflowState::Draft, 1)
    );
}

#[tokio::test]
async fn a_conversation_can_focus_another_conversations_project_plan() {
    let app = test_app().await;
    let v1 = draft(&app).await;
    edit(&app, &v1.workflow_id, 0, &[add(1)]).await;
    let plan = app.storage.get_plan(&v1.workflow_id).await.unwrap();
    let other = app
        .storage
        .create_planning_thread(
            &ctx("other", "thread.create"),
            &app.project,
            "Other",
            "claude-code",
        )
        .await
        .unwrap();
    let mut body = app::default_settings();
    body["command_id"] = json!("focus-shared");
    body["prompt"] = json!("report");
    body["focus"] = json!({ "workflow_id": plan.id, "task_id": plan.tasks[0].id, "revision": 1 });
    let (status, answer) = http_start(&app, other.id.as_str(), body).await;
    assert_eq!(status, 202, "{answer}");
    let op = OperationId::from_literal(answer["operation_id"].as_str().unwrap());
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Completed");
    let entries = app.storage.list_thread_entries(&other.id).await.unwrap();
    assert!(entries.iter().any(|e| {
        e.refs
            .contains(&shadows_core::EntryRef::Task(plan.tasks[0].id.clone()))
    }));
    app.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn deletion_waits_for_a_committed_turn_before_live_registration() {
    use shadows_core::testing::turn::{default_turn_settings, new_turn, turn_command};
    use shadows_core::testing::{PlannerTurn, PlannerTurnRequest};
    let app = test_app().await;
    let opened = app.sessions.open(&app.thread).await.unwrap();
    let events = app
        .sessions
        .lease_events(&app.thread, &opened)
        .await
        .unwrap();
    let settings = default_turn_settings();
    let command = turn_command("pending", &app.thread, "hang", &settings);
    let started = app
        .storage
        .start_turn(
            &command,
            new_turn(&app.thread, &app.runtime, "hang", &settings),
        )
        .await
        .unwrap();
    let op = started.operation_id;
    let mut commits = app.storage.watch_committed();
    let ((removed, at_answer), resumed) =
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            tokio::join!(
                async {
                    let removed = app
                        .core
                        .threads()
                        .remove("remove-pending".into(), &app.thread)
                        .await
                        .unwrap();
                    (removed, app.storage.get_operation(&op).await.unwrap())
                },
                async {
                    // Keep preparation suspended until Stop actually requests
                    // cancellation. Merely waiting for removed_at lets this
                    // test finish preparation before Stop looks for the turn.
                    loop {
                        if app
                            .storage
                            .get_operation(&op)
                            .await
                            .unwrap()
                            .cancel_requested_at
                            .is_some()
                        {
                            break;
                        }
                        commits.changed().await.unwrap();
                    }
                    PlannerTurn::start(
                        app.runtime.clone(),
                        app.handles.clone(),
                        app.sessions.clone(),
                        opened,
                        PlannerTurnRequest {
                            thread_id: app.thread.clone(),
                            harness: "claude-code".into(),
                            operation_id: op.clone(),
                            prompt: "hang".into(),
                            settings,
                            focus: None,
                            continue_plan: None,
                            client_tab: None,
                            events,
                            on_completed: None,
                        },
                        app.bus.clone(),
                    )
                    .await
                    .unwrap()
                }
            )
        })
        .await
        .expect("deletion must finish after the pending turn ends");
    assert!(removed.removed_at.is_some());
    assert_eq!(resumed, op);
    assert!(at_answer.cancel_requested_at.is_some(), "{at_answer:?}");
    assert!(
        at_answer.finished_at.is_some(),
        "delete answered while Pending: {at_answer:?}"
    );
    assert_eq!(app.sessions.live_count().await, 0);
}
