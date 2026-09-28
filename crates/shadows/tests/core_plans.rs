//! Spec §14.9: plan operations answer the same after moving into `Plans`.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::other_project;
use listening::{listening_app, ok, project_client, refused};
use plan::{add, draft_on};
use serde_json::json;

#[tokio::test]
async fn plan_commands_replay_after_the_move() {
    let l = listening_app().await;
    let (_grant, client) = project_client(&l).await;
    let r = ok(&client, "draft_prepare", json!({})).await;
    let args = json!({ "title": "T", "goal": "G", "draft_ref": r["draft_ref"] });
    let first = ok(&client, "draft_start", args.clone()).await;
    let again = ok(&client, "draft_start", args).await;
    assert_eq!(first["workflow_id"], again["workflow_id"]);
    let edit = json!({ "workflow_id": first["workflow_id"], "expected_revision": 0,
        "ops": [add(1)], "command_id": "e1" });
    let e1 = ok(&client, "plan_edit", edit.clone()).await;
    let e2 = ok(&client, "plan_edit", edit).await;
    assert_eq!(e1, e2);
    assert_eq!(
        e1["revision"], 1,
        "the replay answered the first edit, not a second one"
    );
}

#[tokio::test]
async fn a_project_grant_is_refused_another_projects_plan_after_the_move() {
    let l = listening_app().await;
    let (_g, client) = project_client(&l).await;
    let (_, their_thread) = other_project(&l.app).await;
    let other = draft_on(&l.app, &their_thread, "their-start")
        .await
        .workflow_id;
    let text = refused(&client, "workflow_get", json!({ "workflow_id": other })).await;
    assert!(
        text.starts_with("GRANT_SCOPE: that plan is not in this grant's project"),
        "{text}"
    );
}
