use serde_json::Value;
use shadows_core::testing::acp;
use shadows_core::{DesignAnchor, OperationId};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

async fn heard(app: &app::App, op: &OperationId) -> Option<(Value, bool)> {
    app.core
        .harness()
        .context_record(op)
        .await
        .unwrap()
        .map(|(context, heard)| (serde_json::to_value(context).unwrap(), heard))
}

async fn report(app: &app::App) -> (OperationId, Vec<String>) {
    let op = app::start_settled(app, "report").await;
    assert_eq!(app::wait_terminal(app, &op).await.status_kind, "Completed");
    let reply: Value = serde_json::from_str(&app::last_agent_entry(app).await.body).unwrap();
    let blocks = reply["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block.as_str().unwrap().to_owned())
        .collect();
    (op, blocks)
}

#[tokio::test]
async fn the_first_turn_carries_the_context_and_marks_it_heard() {
    let app = app::test_app().await;
    let (op, blocks) = report(&app).await;
    let (record, received) = heard(&app, &op).await.unwrap();
    assert!(received);
    assert!(
        blocks
            .iter()
            .any(|block| block == record["text"].as_str().unwrap())
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn an_unchanged_context_is_not_sent_again() {
    let app = app::test_app().await;
    report(&app).await;
    let (op, blocks) = report(&app).await;
    assert!(heard(&app, &op).await.is_none());
    assert!(
        !blocks
            .iter()
            .any(|block| block.contains("## Planning context index"))
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_changed_anchor_sends_it_again() {
    let app = app::test_app().await;
    report(&app).await;
    app.core
        .design()
        .set_thread_anchor(
            "anchor".into(),
            &app.thread,
            Some(DesignAnchor::Part(app.part.clone().unwrap())),
        )
        .await
        .unwrap();
    let (op, blocks) = report(&app).await;
    assert!(heard(&app, &op).await.unwrap().1);
    assert!(
        blocks
            .iter()
            .any(|block| block.contains("## Planning anchor detail"))
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_command_alone_carries_and_records_nothing_and_the_next_turn_does() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "/compact").await;
    app::wait_terminal(&app, &op).await;
    assert!(heard(&app, &op).await.is_none());
    assert!(heard(&app, &report(&app).await.0).await.unwrap().1);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_failed_turn_leaves_it_owed() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "refuse").await;
    assert_eq!(app::wait_terminal(&app, &op).await.status_kind, "Failed");
    assert!(!heard(&app, &op).await.unwrap().1);
    assert!(heard(&app, &report(&app).await.0).await.unwrap().1);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_stopped_turn_leaves_it_owed() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "hang").await;
    app.core.turns().stop(&op).await.unwrap();
    app::wait_terminal(&app, &op).await;
    assert!(!heard(&app, &op).await.unwrap().1);
    assert!(heard(&app, &report(&app).await.0).await.unwrap().1);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_new_session_on_an_old_thread_gets_it_again() {
    let app = app::test_app().await;
    report(&app).await;
    app.sessions.close_session(&app.thread).await.unwrap();
    // Simulates a lost native session while retaining the thread and its history.
    sqlx::query("UPDATE planning_thread SET harness_session_id=NULL WHERE id=?")
        .bind(app.thread.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    assert!(heard(&app, &report(&app).await.0).await.unwrap().1);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_planning_block_comes_after_changes_and_before_focus() {
    let app = app::test_app().await;
    let workflow = plan::draft(&app).await.workflow_id;
    plan::edit(&app, &workflow, 0, &[plan::add(1)]).await;
    let stored = app.core.plans().get(&workflow).await.unwrap();
    app.sessions.open_session(&app.thread).await.unwrap();
    app.core
        .instructions()
        .save(
            "instructions".into(),
            &app.project,
            "Use the revised project instructions.",
        )
        .await
        .unwrap();
    let mut settings = app::default_settings();
    settings["focus"] = serde_json::json!({"workflow_id": workflow,
        "task_id": stored.tasks[0].id, "revision": stored.revision});
    let op = app::start_on(&app, app.thread.as_str(), "report", settings).await;
    app::wait_terminal(&app, &op).await;
    let reply: Value = serde_json::from_str(&app::last_agent_entry(&app).await.body).unwrap();
    let blocks: Vec<String> = reply["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block.as_str().unwrap().to_owned())
        .collect();
    let planning = blocks
        .iter()
        .position(|block| block.contains("## Planning context index"))
        .unwrap();
    let changes = blocks
        .iter()
        .position(|block| block.contains("Use the revised"))
        .unwrap();
    assert!(changes < planning);
    let focus = blocks
        .iter()
        .position(|block| block.contains("T1"))
        .unwrap();
    assert!(planning < focus);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_bound_agreement_is_read_at_its_pinned_version() {
    use shadows_core::{
        AgreementBinding, AgreementContent, AgreementParty, AgreementRole, DesignOp, PartContent,
        PartId, PlanOp,
    };
    let app = app::test_app().await;
    let provider = app.part.clone().unwrap();
    let consumer = PartId::generate();
    app.core
        .plans()
        .edit_design(
            "consumer".into(),
            &app.project,
            0,
            vec![DesignOp::PartCreate {
                id: consumer.clone(),
                parent: None,
                before: None,
                content: PartContent {
                    title: "Consumer".into(),
                    ..Default::default()
                },
            }],
        )
        .await
        .unwrap();
    let operation = "c0000000-0000-4000-8000-000000000001";
    let first = app.core.design().start_agreement("agreement".into(), &app.project, None,
        Some(AgreementContent {
            capability: "Login".into(), purpose: "Login".into(), behavior: "Version one behavior".into(),
            acceptance: vec!["Works".into()],
            parties: vec![AgreementParty { part_id: provider.clone(), role: AgreementRole::Provides },
                AgreementParty { part_id: consumer, role: AgreementRole::Uses }],
            openapi: serde_json::json!({"openapi":"3.1.0","info":{"title":"Login","version":"1"},
                "paths":{"/login":{"post":{"x-shadows-operation-id":operation,
                    "responses":{"200":{"description":"OK"}}}}}}),
        }), None).await.unwrap();
    let review = app
        .core
        .plans()
        .review_agreement(&app.project, &first.agreement_id)
        .await
        .unwrap();
    app.core
        .plans()
        .agree_agreement(
            "agree".into(),
            &app.project,
            &first.agreement_id,
            0,
            review.review_id,
        )
        .await
        .unwrap();
    let workflow = plan::draft(&app).await.workflow_id;
    plan::edit(
        &app,
        &workflow,
        0,
        &[
            plan::add(1),
            PlanOp::BindingPut {
                binding: AgreementBinding {
                    task: 1,
                    agreement_id: first.agreement_id.clone(),
                    version: 1,
                    part_id: provider.clone(),
                    role: AgreementRole::Provides,
                    operations: vec![operation.into()],
                },
            },
        ],
    )
    .await;
    let second = app
        .core
        .design()
        .start_agreement(
            "v2".into(),
            &app.project,
            Some(&first.agreement_id),
            None,
            Some("New behavior".into()),
        )
        .await
        .unwrap();
    let mut content = second.content;
    content.behavior = "Version two behavior".into();
    app.core
        .design()
        .edit_agreement(
            "edit-v2".into(),
            &app.project,
            &first.agreement_id,
            2,
            0,
            content,
        )
        .await
        .unwrap();
    app.core
        .design()
        .set_thread_anchor(
            "anchor".into(),
            &app.thread,
            Some(DesignAnchor::Part(provider)),
        )
        .await
        .unwrap();
    let context = app
        .core
        .turns()
        .planning_context(&app.thread, &app.project)
        .await
        .unwrap()
        .unwrap();
    assert!(context.text.contains("Version one behavior"));
    assert!(!context.text.contains("Version two behavior"));
    assert!(
        context
            .sources
            .iter()
            .any(|source| source.kind == "agreement"
                && source.id == first.agreement_id.as_str()
                && source.version == 1)
    );
    assert!(context.text.contains("T1: task 1"));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_removed_anchor_says_so() {
    let app = app::test_app().await;
    let part = app.part.clone().unwrap();
    app.core
        .design()
        .set_thread_anchor(
            "anchor".into(),
            &app.thread,
            Some(DesignAnchor::Part(part.clone())),
        )
        .await
        .unwrap();
    // Represents an anchor retained after its design item was removed.
    sqlx::query("DELETE FROM design_part WHERE id=?")
        .bind(part.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    let context = app
        .core
        .turns()
        .planning_context(&app.thread, &app.project)
        .await
        .unwrap()
        .unwrap();
    assert!(context.text.contains("[anchor removed:"));
    assert_eq!(context.detail_tokens, 0);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn an_aylul_thread_gets_nothing() {
    let app = app::test_app().await;
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        &app.storage,
        &app::ctx("aylul", "thread.create"),
        &app.project,
        "Assistant",
        "aylul",
        None,
    )
    .await
    .unwrap();
    assert!(
        app.core
            .turns()
            .planning_context(&thread.id, &app.project)
            .await
            .unwrap()
            .is_none()
    );
    app::shut_down_app(app).await;
}
