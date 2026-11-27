//! Design element writes and reads through the Planner's MCP tools.

use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{ElementSeed, OperationId};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use listening::{Client, Listening, call, listening_app, ok, thread_client};

async fn open_planner(l: &Listening) -> (ElementSeed, OperationId, Client) {
    let seed = l
        .app
        .core
        .design()
        .seed_elements_for_tests(&l.app.project)
        .await
        .unwrap();
    let turn = app::start_settled(&l.app, "hang").await;
    let planner = thread_client(l, &l.app.thread).await;
    (seed, turn, planner)
}

#[tokio::test]
async fn the_planner_writes_a_table_and_reads_it_back() {
    let l = listening_app().await;
    let (seed, turn, planner) = open_planner(&l).await;
    let table = ok(
        &planner,
        "design_element_put",
        json!({
            "command_id": "users",
            "part_id": seed.database,
            "outcomes": [seed.outcome],
            "content": {
                "kind": "table", "name": "users", "purpose": "Store users",
                "columns": [{"name": "id", "type_name": "TEXT", "primary_key": true}]
            }
        }),
    )
    .await;
    assert_eq!(table["id"], "E1");
    assert_eq!(table["revision"], 1);
    assert_eq!(table["content"]["columns"][0]["name"], "id");
    assert_eq!(
        ok(&planner, "design_elements", json!({})).await,
        json!([table])
    );

    let deleted = ok(
        &planner,
        "design_element_delete",
        json!({"command_id": "remove-users", "id": "E1", "expected_revision": 1}),
    )
    .await;
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["revision"], 2);
    assert_eq!(
        ok(&planner, "design_elements", json!({"id": "E1"})).await,
        json!([table, deleted])
    );

    l.app.core.turns().stop(&turn).await.unwrap();
    planner.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn a_refused_element_keeps_its_code() {
    let l = listening_app().await;
    let (seed, turn, planner) = open_planner(&l).await;
    let (error, text) = call(
        &planner,
        "design_element_put",
        json!({
            "command_id": "login",
            "part_id": seed.database,
            "outcomes": [seed.outcome],
            "content": {"kind": "screen", "name": "Login", "purpose": "Sign in"}
        }),
    )
    .await;
    assert!(error);
    assert_eq!(
        text,
        format!(
            "INVALID_ELEMENT: a screen belongs to a part of kind frontend; {} is database",
            seed.database
        )
    );
    l.app.core.turns().stop(&turn).await.unwrap();
    planner.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn code_where_tool_answers_the_method() {
    let l = listening_app().await;
    let (seed, turn, planner) = open_planner(&l).await;
    ok(
        &planner,
        "design_element_put",
        json!({
            "command_id": "users",
            "part_id": seed.database,
            "outcomes": [seed.outcome],
            "content": {
                "kind": "table", "name": "users", "purpose": "Store users",
                "columns": [{"name": "id", "type_name": "TEXT", "primary_key": true}]
            }
        }),
    )
    .await;
    let tool = ok(&planner, "code_where", json!({"topic": "users"})).await;
    let expected = l
        .app
        .core
        .code_record()
        .code_where(&l.app.project, "users")
        .await
        .unwrap();
    assert_eq!(tool, serde_json::to_value(&expected).unwrap());
    assert_eq!(tool["entries"][0]["element"]["id"], "E1");
    let (error, text) = call(&planner, "code_where", json!({"topic": " "})).await;
    assert!(error);
    assert_eq!(text, "INVALID_COMMAND: a topic needs 1 to 200 characters");
    l.app.core.turns().stop(&turn).await.unwrap();
    planner.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn project_state_tool_answers_the_method() {
    let l = listening_app().await;
    let (seed, turn, planner) = open_planner(&l).await;
    ok(
        &planner,
        "design_element_put",
        json!({
            "command_id": "users",
            "part_id": seed.database,
            "outcomes": [seed.outcome],
            "content": {
                "kind": "table", "name": "users", "purpose": "Store users",
                "columns": [{"name": "id", "type_name": "TEXT", "primary_key": true}]
            }
        }),
    )
    .await;
    let state = l.app.core.project_state();
    let whole = ok(&planner, "project_state", json!({})).await;
    let expected = state.state(&l.app.project, None).await.unwrap();
    assert_eq!(whole, serde_json::to_value(&expected).unwrap());
    let part = ok(&planner, "project_state", json!({"part_id": seed.database})).await;
    let expected = state
        .state(&l.app.project, Some(&seed.database))
        .await
        .unwrap();
    assert_eq!(part, serde_json::to_value(&expected).unwrap());
    assert_eq!(part["part"]["elements"][0]["element"]["id"], "E1");
    let served = ok(
        &planner,
        "project_state",
        json!({"outcome_id": seed.outcome}),
    )
    .await;
    let expected = state
        .state_for(&l.app.project, None, Some(&seed.outcome))
        .await
        .unwrap();
    assert_eq!(served, serde_json::to_value(&expected).unwrap());
    let args = json!({"part_id": seed.database, "outcome_id": seed.outcome});
    let (error, text) = call(&planner, "project_state", args).await;
    assert!(error);
    assert!(text.starts_with("INVALID_COMMAND:"), "{text}");
    let (error, text) = call(&planner, "project_state", json!({"part_id": "nope"})).await;
    assert!(error);
    assert_eq!(
        text,
        "INVALID_COMMAND: part nope is not a part of this project"
    );
    l.app.core.turns().stop(&turn).await.unwrap();
    planner.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}
