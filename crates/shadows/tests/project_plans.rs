//! What §16 adds to a plan's versions (spec §16.2, §16.3): one Draft per
//! plan held by the database, a reason from v2 on, and the writer an
//! approval speaks to. Split from `plan_storage.rs`, which holds §13's
//! version rules.

use serde_json::json;
use shadows_core::StorageError;
use shadows_core::testing::Writer;
use shadows_core::testing::{Anchor, derived_id};
use shadows_core::{WorkflowState, WrittenBy};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, test_app};
use plan::{
    add, approved_v1, draft_on, edit_ctx, in_an_hour, insert_draft_ref, insert_grant, planner,
    writer_ctx,
};

/// §16.2: one Draft per plan, held by the database. The second start answers
/// the first's Draft.
#[tokio::test]
async fn two_starts_make_one_draft() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;
    let (a, b) = tokio::join!(
        draft_on(&app, &app.thread, "start-a"),
        draft_on(&app, &app.thread, "start-b"),
    );
    assert_eq!((a.workflow_id.clone(), a.version), (b.workflow_id, 2));
    assert_eq!(a.plan_id, app.storage.get_plan(&v1).await.unwrap().plan_id);
    let drafts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow WHERE plan_id = ? AND state = 'Draft'")
            .bind(a.plan_id.as_str())
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    assert_eq!(drafts, 1);
}

/// §16.3: v2 needs its reason; v1 has none.
#[tokio::test]
async fn a_new_version_needs_its_reason() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;
    let plan = app.storage.get_plan(&v1).await.unwrap().plan_id;
    let writer = planner(&app, &app.thread).await;
    let start = |command: &'static str, reason: Option<&'static str>| {
        let (app, writer, plan) = (&app, &writer, &plan);
        async move {
            app.storage
                .start_draft(
                    &writer_ctx(writer, command, "DraftStart", json!({ "c": command })),
                    writer,
                    &app.project,
                    Some(plan),
                    None,
                    None,
                    reason,
                    None,
                    None,
                )
                .await
        }
    };
    for (command, blank) in [("no-reason", None), ("blank-reason", Some("  "))] {
        match start(command, blank).await {
            Err(StorageError::PlanInvalid(problems)) => {
                assert_eq!(problems[0].message, "a new version needs its reason");
            }
            other => panic!("expected PlanInvalid, got {other:?}"),
        }
    }
    let v2 = start("with-reason", Some("the API changed")).await.unwrap();
    assert_eq!(v2.version, 2);
    let read = |id| app.storage.get_plan(id);
    assert_eq!(
        read(&v2.workflow_id)
            .await
            .unwrap()
            .change_reason
            .as_deref(),
        Some("the API changed")
    );
    assert_eq!(read(&v1).await.unwrap().change_reason, None);
}

/// §16.3: a person's approval of an external agent's version has no
/// conversation to speak in.
#[tokio::test]
async fn approving_an_external_version_writes_no_entry() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External {
        grant: grant.clone(),
    };
    let r = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let started = app
        .storage
        .start_draft(
            &writer_ctx(
                &external,
                &derived_id(Anchor::DraftRef(&r), ""),
                "DraftStart",
                json!({}),
            ),
            &external,
            &app.project,
            None,
            None,
            Some(("Search", "find things")),
            None,
            None,
            Some(&r),
        )
        .await
        .unwrap();
    let v1 = started.workflow_id;
    app.storage
        .edit_plan(
            &edit_ctx(&external, &v1, 0, &[add(1)]),
            &external,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await
        .unwrap();
    let approved = app
        .storage
        .approve_plan(&ctx("approve-ext", "PlanApprove"), &v1, 1)
        .await
        .unwrap();
    assert_eq!(
        (approved.workflow_id.clone(), approved.version),
        (v1.clone(), 1)
    );

    let plan = app.storage.get_plan(&v1).await.unwrap();
    assert_eq!(plan.state, WorkflowState::Frozen);
    assert_eq!(plan.written_by, WrittenBy::External { grant_id: grant });
    let threads: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM planning_thread")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(threads, 1, "only the app's own thread");
    let approvals: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM thread_entry WHERE kind = 'PlanApproved'")
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    assert_eq!(approvals, 0);
    let frozen: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT thread_id FROM durable_event WHERE kind = 'WorkflowFrozen' AND project_id = ?",
    )
    .bind(app.project.as_str())
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    assert_eq!(frozen, vec![None]);
}
