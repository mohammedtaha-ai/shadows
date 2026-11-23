//! Where a topic is, over HTTP: the route answers CodeRecord's method.

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn the_route_answers_the_method() {
    let app = app::test_app().await;
    let path = format!("/api/projects/{}/code-where?topic=users", app.project);
    let (status, answer) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 200, "{answer}");
    let expected = app
        .core
        .code_record()
        .code_where(&app.project, "users")
        .await
        .unwrap();
    assert_eq!(answer, serde_json::to_value(&expected).unwrap());
    let unknown = uuid::Uuid::new_v4();
    let path = format!("/api/projects/{unknown}/code-where?topic=users");
    let (status, _) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 404);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_blank_topic_is_422() {
    let app = app::test_app().await;
    let path = format!("/api/projects/{}/code-where?topic=%20", app.project);
    let (status, refusal) = app::call(&app, "GET", &path, None).await;
    assert_eq!(status, 422, "{refusal}");
    assert_eq!(refusal["code"], "INVALID_COMMAND");
    assert_eq!(refusal["message"], "a topic needs 1 to 200 characters");
    app::shut_down_app(app).await;
}
