//! A project's Planner instructions (spec §13.8): each save a new numbered
//! version, nothing overwritten, the current one the highest number.

use serde_json::{Value, json};

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{call, ctx, get_json, other_project, test_app};

fn save_ctx(id: &str, body: &str) -> shadows::command::CommandContext {
    let mut c = ctx(id, "PlannerInstructionsSave");
    c.request_fingerprint = shadows::command::fingerprint("PlannerInstructionsSave", &json!(body));
    c
}

#[tokio::test]
async fn each_save_is_a_new_numbered_version() {
    let app = test_app().await;
    let first = app
        .storage
        .save_planner_instructions(&save_ctx("s1", "Plan small."), &app.project, "Plan small.")
        .await
        .unwrap();
    let second = app
        .storage
        .save_planner_instructions(&save_ctx("s2", "Plan big."), &app.project, "Plan big.")
        .await
        .unwrap();
    assert_eq!((first.number, first.body.as_str()), (1, "Plan small."));
    assert_eq!((second.number, second.body.as_str()), (2, "Plan big."));
    assert_ne!(first.id, second.id);

    // A replay answers the version it saved, and saves nothing.
    let replay = app
        .storage
        .save_planner_instructions(&save_ctx("s1", "Plan small."), &app.project, "Plan small.")
        .await
        .unwrap();
    assert_eq!(replay, first);
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM planner_instructions_version")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(rows, 2);
}

#[tokio::test]
async fn current_instructions_are_the_highest_number() {
    let app = test_app().await;
    let (other, _) = other_project(&app).await;
    assert_eq!(
        app.storage
            .current_planner_instructions(&app.project)
            .await
            .unwrap(),
        None
    );
    for (i, body) in ["one", "two", "three"].iter().enumerate() {
        app.storage
            .save_planner_instructions(&save_ctx(&format!("s{i}"), body), &app.project, body)
            .await
            .unwrap();
    }
    let current = app
        .storage
        .current_planner_instructions(&app.project)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((current.number, current.body.as_str()), (3, "three"));
    // Another project's instructions are its own, numbered from 1.
    assert_eq!(
        app.storage
            .current_planner_instructions(&other)
            .await
            .unwrap(),
        None
    );
    let theirs = app
        .storage
        .save_planner_instructions(&save_ctx("o1", "theirs"), &other, "theirs")
        .await
        .unwrap();
    assert_eq!(theirs.number, 1);
}

#[tokio::test]
async fn the_saved_event_does_not_carry_the_body() {
    let app = test_app().await;
    let body = "Keep every task under a day. لا تكتب رمزاً قبل الخطة.";
    app.storage
        .save_planner_instructions(&save_ctx("s1", body), &app.project, body)
        .await
        .unwrap();
    let events: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, payload_json FROM durable_event WHERE project_id = ? ORDER BY seq",
    )
    .bind(app.project.as_str())
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    let saved: Vec<Value> = events
        .iter()
        .filter(|(k, _)| k == "PlannerInstructionsSaved")
        .map(|(_, p)| serde_json::from_str(p).unwrap())
        .collect();
    assert_eq!(saved, [json!({ "number": 1 })]);
    assert!(events.iter().all(|(_, p)| !p.contains("Keep every task")));
}

#[tokio::test]
async fn the_routes_read_and_save_a_version() {
    let app = test_app().await;
    let path = format!("/api/projects/{}/planner-instructions", app.project);
    let none: Value = get_json(&app, &path).await;
    assert_eq!(none, Value::Null);

    let body = "خطط بمهام صغيرة.";
    let (status, saved) = call(
        &app,
        "PUT",
        &path,
        Some(json!({ "command_id": "s1", "body": body })),
    )
    .await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["number"], 1);
    assert_eq!(saved["body"], body);
    assert!(saved["created_at"].is_string());
    assert_eq!(
        saved.as_object().unwrap().len(),
        3,
        "number, body, created_at: {saved}"
    );
    let current: Value = get_json(&app, &path).await;
    assert_eq!(current, saved);

    // The same command id with another body is a conflict, not a save.
    let (status, conflict) = call(
        &app,
        "PUT",
        &path,
        Some(json!({ "command_id": "s1", "body": "other" })),
    )
    .await;
    assert_eq!(status, 409, "{conflict}");
    assert_eq!(conflict["code"], "COMMAND_CONFLICT");

    let unknown = "/api/projects/00000000-0000-4000-8000-000000000000/planner-instructions";
    let (status, missing) = call(
        &app,
        "PUT",
        unknown,
        Some(json!({ "command_id": "s2", "body": "x" })),
    )
    .await;
    assert_eq!(status, 404, "{missing}");
}
