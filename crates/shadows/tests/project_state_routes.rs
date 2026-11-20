//! The project's state over HTTP: the route answers ProjectState's method.

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

#[tokio::test]
async fn the_state_route_answers_the_method() {
    let app = app::test_app().await;
    plan::approved_v1(&app).await;
    let part = shadows_core::AppCore::design_for_tests(app.storage.clone())
        .seed_roadmap_for_tests(&app.project)
        .await
        .unwrap();
    let path = format!("/api/projects/{}/state", app.project);
    let state = app.core.project_state();
    let (status, whole) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 200, "{whole}");
    let expected = state.state(&app.project, None).await.unwrap();
    assert_eq!(whole, serde_json::to_value(&expected).unwrap());
    assert!(whole.get("part").is_none());
    let (status, detail) = app::call(&app, "GET", &format!("{path}?part_id={part}"), None).await;
    assert_eq!(status, 200, "{detail}");
    let expected = state.state(&app.project, Some(&part)).await.unwrap();
    assert_eq!(detail, serde_json::to_value(&expected).unwrap());
    assert_eq!(detail["part"]["plans"][0]["tasks"][0]["number"], 1);
    let unknown = uuid::Uuid::new_v4();
    let (status, _) = app::call(&app, "GET", &format!("/api/projects/{unknown}/state"), None).await;
    assert_eq!(status, 404);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn an_unknown_part_is_422() {
    let app = app::test_app().await;
    let path = format!("/api/projects/{}/state?part_id=nope", app.project);
    let (status, refusal) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 422, "{refusal}");
    assert_eq!(refusal["code"], "INVALID_COMMAND");
    assert_eq!(
        refusal["message"],
        "part nope is not a part of this project"
    );
    app::shut_down_app(app).await;
}
