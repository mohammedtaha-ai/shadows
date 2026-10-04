//! Cross-project plan reads use the grant's one-way project links (§16.10).

use serde_json::json;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::other_project;
use listening::{listening_app, ok, project_client, refused};
use plan::{add, draft_on};

#[tokio::test]
async fn a_foreign_task_link_names_the_missing_project_link_as_invalid_command() {
    let l = listening_app().await;
    let (_, target_thread) = other_project(&l.app).await;
    let target = draft_on(&l.app, &target_thread, "target").await;
    plan::edit(&l.app, &target.workflow_id, 0, &[add(3)]).await;
    let source = plan::draft(&l.app).await;
    let (_, client) = project_client(&l).await;
    let text = refused(
        &client,
        "plan_edit",
        json!({
            "workflow_id": source.workflow_id, "expected_revision": 0,
            "ops": [add(4), {"op": "link_put", "link": {
                "task": 4, "after": {"plan_id": target.plan_id, "task": 3},
                "kind": "needs", "label": "Login API", "waiting_items": []
            }}]
        }),
    )
    .await;
    assert!(text.starts_with("INVALID_COMMAND:"), "{text}");
    assert!(text.contains("project link"), "{text}");
    assert_eq!(plan::revision(&l.app, &source.workflow_id).await, 0);
}

#[tokio::test]
async fn a_linked_project_can_be_read_but_never_written_by_the_source_grant() {
    let l = listening_app().await;
    let (target, thread) = other_project(&l.app).await;
    let theirs = draft_on(&l.app, &thread, "target-draft").await.workflow_id;
    let (_, client) = project_client(&l).await;
    let args = json!({ "workflow_id": theirs, "project": "other" });
    assert!(
        refused(&client, "workflow_get", args.clone())
            .await
            .starts_with("GRANT_SCOPE:")
    );
    l.app
        .core
        .code()
        .link("link-target".into(), &l.app.project, &target)
        .await
        .unwrap();
    let listed = ok(&client, "workflow_list", json!({ "project": "other" })).await;
    assert_eq!(listed[0]["id"], json!(theirs));
    assert_eq!(
        ok(&client, "workflow_get", args.clone()).await["id"],
        json!(theirs)
    );
    assert!(
        refused(
            &client,
            "plan_edit",
            json!({
                "workflow_id": theirs, "project": "other", "expected_revision": 0, "ops": [add(1)]
            })
        )
        .await
        .starts_with("GRANT_SCOPE:")
    );
    assert_eq!(plan::revision(&l.app, &theirs).await, 0);
    l.app
        .core
        .code()
        .unlink("unlink-target".into(), &l.app.project, &target)
        .await
        .unwrap();
    assert!(
        refused(&client, "workflow_get", args)
            .await
            .starts_with("GRANT_SCOPE:")
    );
    assert!(
        refused(&client, "workflow_list", json!({ "project": "other" }))
            .await
            .starts_with("GRANT_SCOPE:")
    );
}
