//! Spec §14.9: plan operations answer the same after moving into `Plans`.

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{other_project, post, start_settled};
use listening::{listening_app, ok, project_client, refused, thread_client};
use plan::{add, draft_on};
use serde_json::json;
use shadows_core::testing::fingerprint;

/// A replay only matches a command recorded by an earlier daemon when its
/// kind, schema version and fingerprint parameters are the ones that daemon
/// used. `plan_commands_replay_after_the_move` replays within one daemon, so
/// it cannot see a parameter set that changed on both calls; this reads what
/// the command log holds and compares it with each command's parameters as
/// the code before the move wrote them (§14.9).
#[tokio::test]
async fn plan_command_fingerprints_do_not_move() {
    let l = listening_app().await;
    let thread = l.app.thread.clone();

    // The Planner, inside a turn: DraftStart, PlanEdit, PlanShow.
    let planner = thread_client(&l, &thread).await;
    start_settled(&l.app, "hang").await;
    for _ in 0..200 {
        if l.app.handles.running_turn(&thread).await.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let started = ok(
        &planner,
        "draft_start",
        json!({ "title": "T", "goal": "G" }),
    )
    .await;
    let v1 = started["workflow_id"].clone();
    let edit = json!({ "expected_revision": 0, "ops": [add(1)], "command_id": "pin-edit" });
    ok(&planner, "plan_edit", edit).await;
    ok(
        &planner,
        "plan_show",
        json!({ "task_number": 1, "place": "side" }),
    )
    .await;

    // An external agent's DraftStart, from scratch.
    let (_grant, external) = project_client(&l).await;
    let r = ok(&external, "draft_prepare", json!({})).await;
    let scratch = json!({ "title": "X", "goal": "Y", "draft_ref": r["draft_ref"] });
    ok(&external, "draft_start", scratch).await;

    // A person's PlanApprove.
    let approve = json!({ "command_id": "pin-approve", "expected_revision": 1 });
    let (status, body) = post(
        &l.app,
        &format!("/api/workflows/{}/approve", v1.as_str().unwrap()),
        approve,
    )
    .await;
    assert_eq!(status, 200, "{body}");

    let mut recorded: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT command_kind, command_schema_ver, request_fingerprint FROM command_record
          WHERE command_kind IN ('DraftStart', 'PlanEdit', 'PlanShow', 'PlanApprove')",
    )
    .fetch_all(l.app.storage.reader())
    .await
    .unwrap();
    recorded.sort();
    let pinned =
        |kind: &str, params: serde_json::Value| (kind.to_string(), 1, fingerprint(kind, &params));
    let mut expected = vec![
        pinned(
            "DraftStart",
            json!({ "thread": thread, "title": "T", "goal": "G" }),
        ),
        pinned(
            "DraftStart",
            json!({ "title": "X", "goal": "Y", "from_workflow_id": null }),
        ),
        pinned(
            "PlanEdit",
            json!({ "workflow": v1, "expected_revision": 0, "ops": [add(1)] }),
        ),
        pinned(
            "PlanShow",
            json!({ "workflow": v1, "task_number": 1, "place": "side" }),
        ),
        pinned(
            "PlanApprove",
            json!({ "workflow": v1, "expected_revision": 1 }),
        ),
    ];
    expected.sort();
    assert_eq!(recorded, expected);
}

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
