//! The plan's HTTP routes (spec §13.10): a project's plan list, one version's
//! read, and the person's approval with its refusals.

use serde_json::{Value, json};
use shadows_core::ThreadId;
use shadows_core::{PlanEdit, PlanOp, TaskContent};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{call, create_thread, get_json, post, test_app};
use plan::{add, approved_v1, draft, draft_on, edit, events_of, needs, project_events_of, task};

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
async fn the_project_lists_each_plans_latest_version() {
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
    assert!(listed[0]["title"].is_string() && listed[0]["plan_id"].is_string());
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
        message.starts_with("the resource changed; current revision is 1: "),
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

/// §16.2: an archived plan leaves the list, is read only, and comes back.
#[tokio::test]
async fn an_archived_plan_is_read_only_until_unarchived() {
    let app = test_app().await;
    let draft = draft(&app).await;

    // A plan with a Draft. POST archive → state Archived.
    let (status, archived) = post(
        &app,
        &format!("/api/plans/{}/archive", draft.plan_id),
        json!({ "command_id": "archive-1" }),
    )
    .await;
    assert_eq!(status, 200, "{archived}");
    assert_eq!(archived["state"], "Archived");
    assert!(archived["archived_at"].is_string());

    // Replay of archive answers the plan as it stands.
    let (status, replayed_archive) = post(
        &app,
        &format!("/api/plans/{}/archive", draft.plan_id),
        json!({ "command_id": "archive-1" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(replayed_archive, archived);

    // Archiving an already archived plan with a new command answers unchanged.
    let (status, again_archived) = post(
        &app,
        &format!("/api/plans/{}/archive", draft.plan_id),
        json!({ "command_id": "archive-2" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(again_archived["state"], "Archived");
    assert_eq!(
        project_events_of(&app, &app.project, "PlanArchived")
            .await
            .len(),
        1
    );

    // GET workflows → it is absent; GET workflows?archived=true → it is present.
    let listed: Value = get_json(&app, &format!("/api/projects/{}/workflows", app.project)).await;
    let listed_ids: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["plan_id"].as_str())
        .collect();
    assert!(!listed_ids.contains(&draft.plan_id.as_str()), "{listed}");

    let listed_archived: Value = get_json(
        &app,
        &format!("/api/projects/{}/workflows?archived=true", app.project),
    )
    .await;
    let archived_ids: Vec<&str> = listed_archived
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["plan_id"].as_str())
        .collect();
    assert!(
        archived_ids.contains(&draft.plan_id.as_str()),
        "{listed_archived}"
    );

    // A plan_edit through an external grant on its Draft → INVALID_COMMAND,
    // text contains "archived".
    let grant = app
        .core
        .grants()
        .issue("grant-external".into(), &app.project)
        .await
        .unwrap()
        .grant;
    let err = app
        .core
        .plans()
        .edit(
            &grant,
            PlanEdit {
                workflow_id: Some(draft.workflow_id.clone()),
                expected_revision: 0,
                ops: vec![add(1)],
                command_id: None,
            },
        )
        .await
        .unwrap_err();
    match err {
        shadows_core::CoreError::Refused { code, message } => {
            assert_eq!(code, shadows_core::ErrorCode::InvalidCommand);
            assert!(
                message.contains("archived"),
                "expected 'archived' in message: {message}"
            );
        }
        other => panic!("expected Refused with InvalidCommand, got {other:?}"),
    }

    // POST unarchive → Active, and the same edit now succeeds.
    let (status, unarchived) = post(
        &app,
        &format!("/api/plans/{}/unarchive", draft.plan_id),
        json!({ "command_id": "unarchive-1" }),
    )
    .await;
    assert_eq!(status, 200, "{unarchived}");
    assert_eq!(unarchived["state"], "Active");
    assert!(unarchived["archived_at"].is_null());

    // Replay of unarchive answers the plan as it stands.
    let (status, replayed_unarchive) = post(
        &app,
        &format!("/api/plans/{}/unarchive", draft.plan_id),
        json!({ "command_id": "unarchive-1" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(replayed_unarchive, unarchived);

    // Unarchiving an already active plan with a new command answers unchanged.
    let (status, again_unarchived) = post(
        &app,
        &format!("/api/plans/{}/unarchive", draft.plan_id),
        json!({ "command_id": "unarchive-2" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(again_unarchived["state"], "Active");
    assert_eq!(
        project_events_of(&app, &app.project, "PlanUnarchived")
            .await
            .len(),
        1
    );

    let outcome = app
        .core
        .plans()
        .edit(
            &grant,
            PlanEdit {
                workflow_id: Some(draft.workflow_id.clone()),
                expected_revision: 0,
                ops: vec![add(1)],
                command_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(outcome.revision, 1);

    // GET /api/plans/{id} lists v1 with its written_by.
    let plan: Value = get_json(&app, &format!("/api/plans/{}", draft.plan_id)).await;
    assert_eq!(plan["plan_id"], draft.plan_id.to_string());
    assert_eq!(plan["state"], "Active");
    assert!(plan["archived_at"].is_null());
    let versions = plan["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["workflow_id"], draft.workflow_id.to_string());
    assert_eq!(versions[0]["version"], 1);
    assert_eq!(versions[0]["title"], "Login");
    assert_eq!(versions[0]["written_by"]["kind"], "planner");
    assert_eq!(
        versions[0]["written_by"]["thread_id"],
        app.thread.to_string()
    );

    // A plan of a removed project is not found (§16.2).
    sqlx::query("UPDATE project SET removed_at = '2026-10-01T00:00:00Z' WHERE id = ?")
        .bind(app.project.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    let (status, _) = call(&app, "GET", &format!("/api/plans/{}", draft.plan_id), None).await;
    assert_eq!(status, 404);
}
