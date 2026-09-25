//! The plan's HTTP routes (spec §13.10): a project's plan list, one version's
//! read, and the person's approval with its refusals.

use serde_json::{Value, json};
use shadows::thread::ThreadId;
use shadows::workflow::{PlanOp, TaskContent};

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{create_thread, get_json, post, test_app};
use plan::{add, approved_v1, draft, draft_on, edit, events_of, needs, task};

fn approve_body(command_id: &str, expected_revision: i64) -> Value {
    json!({ "command_id": command_id, "expected_revision": expected_revision })
}

/// T1 with nothing anyone could check it by: an edit accepts it, approval
/// does not.
fn add_unchecked(n: u32) -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            acceptance: vec![],
            ..task(n, &format!("task {n}"))
        },
    }
}

#[tokio::test]
async fn the_project_lists_each_threads_latest_version() {
    let app = test_app().await;
    approved_v1(&app).await;
    let v2 = draft_on(&app, &app.thread, "start-v2").await;
    let other = create_thread(&app, json!({ "command_id": "t-2", "title": "Other" })).await;
    let other = ThreadId::from_literal(other["id"].as_str().unwrap());
    let other_v1 = draft_on(&app, &other, "start-other").await;

    let listed: Value = get_json(&app, &format!("/api/projects/{}/workflows", app.project)).await;
    let mut rows: Vec<(String, i64, String)> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["id"].as_str().unwrap().to_string(),
                p["version"].as_i64().unwrap(),
                p["state"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    rows.sort();
    let mut expected = vec![
        (v2.workflow_id.to_string(), 2, "Draft".to_string()),
        (other_v1.workflow_id.to_string(), 1, "Draft".to_string()),
    ];
    expected.sort();
    assert_eq!(rows, expected, "{listed}");
    assert!(listed[0]["title"].is_string() && listed[0]["thread_id"].is_string());
}

#[tokio::test]
async fn a_plan_read_lists_its_blockers_and_its_last_edit() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add(2), add_unchecked(1), needs(2, 1)]).await;

    let plan: Value = get_json(&app, &format!("/api/workflows/{v1}")).await;
    assert_eq!(plan["revision"], 1);
    assert_eq!(plan["state"], "Draft");
    assert_eq!(plan["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(plan["links"].as_array().unwrap().len(), 1);
    assert_eq!(
        plan["blockers"],
        json!([{ "message": "T1 has no acceptance item" }])
    );
    assert_eq!(plan["last_edit"]["revision"], 1);
    assert_eq!(plan["last_edit"]["changed_tasks"], json!([1, 2]));
    assert_eq!(
        plan["last_edit"]["summary"],
        "3 changes: added T2, added T1, linked T2 → T1"
    );
}

#[tokio::test]
async fn approving_a_stale_revision_is_refused_with_the_current_one() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add(1)]).await;

    let (status, body) = post(
        &app,
        &format!("/api/workflows/{v1}/approve"),
        approve_body("approve-stale", 0),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "REVISION_CONFLICT");
    assert_eq!(body["current_revision"], 1);
    let message = body["message"].as_str().unwrap();
    assert!(
        message.starts_with("the plan changed; current revision is 1: "),
        "{message}"
    );
    assert!(body.get("problems").is_none(), "{body}");
    assert!(
        events_of(&app, &app.thread, "WorkflowFrozen")
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn approving_with_blockers_is_422_with_the_list() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add_unchecked(1), add_unchecked(2)]).await;

    let (status, body) = post(
        &app,
        &format!("/api/workflows/{v1}/approve"),
        approve_body("approve-blocked", 1),
    )
    .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["code"], "WORKFLOW_VALIDATION_FAILED");
    assert_eq!(
        body["problems"],
        json!(["T1 has no acceptance item", "T2 has no acceptance item"])
    );
    assert_eq!(
        body["message"],
        "T1 has no acceptance item; T2 has no acceptance item"
    );
    assert!(body.get("current_revision").is_none(), "{body}");
}

#[tokio::test]
async fn a_replayed_approval_answers_the_first_result() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add(1), add(2), needs(2, 1)]).await;
    let path = format!("/api/workflows/{v1}/approve");

    let (first_status, first) = post(&app, &path, approve_body("approve-once", 1)).await;
    let (again_status, again) = post(&app, &path, approve_body("approve-once", 1)).await;
    assert_eq!((first_status, again_status), (200, 200), "{first} {again}");
    assert_eq!(first, again);
    assert_eq!(first["workflow_id"], v1.to_string());
    assert_eq!(
        (first["version"].as_i64(), first["revision"].as_i64()),
        (Some(1), Some(1))
    );
    let (conflict_status, conflict) = post(&app, &path, approve_body("approve-once", 0)).await;
    assert_eq!(conflict_status, 409, "{conflict}");
    assert_eq!(conflict["code"], "COMMAND_CONFLICT");
    assert_eq!(
        events_of(&app, &app.thread, "WorkflowFrozen").await.len(),
        1
    );
    let plan: Value = get_json(&app, &format!("/api/workflows/{v1}")).await;
    assert_eq!(plan["state"], "Frozen");
}

#[tokio::test]
async fn approving_a_frozen_version_again_with_a_new_command_is_409_frozen() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;

    let (status, body) = post(
        &app,
        &format!("/api/workflows/{v1}/approve"),
        approve_body("approve-again", 1),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "WORKFLOW_FROZEN_IMMUTABLE");
    assert_eq!(
        events_of(&app, &app.thread, "WorkflowFrozen").await.len(),
        1
    );
}
