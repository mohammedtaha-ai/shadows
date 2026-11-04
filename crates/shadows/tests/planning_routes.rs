use serde_json::json;
use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn an_anchor_is_set_over_http_and_read_back() {
    let app = app::test_app().await;
    let path = format!("/api/threads/{}/anchor", app.thread);
    let anchor = json!({"kind":"part", "id":app.part.as_ref().unwrap()});
    let (status, answer) = app::call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id":"anchor", "anchor":anchor})),
    )
    .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer, json!({"anchor":anchor}));
    assert_eq!(app::call(&app, "GET", &path, None).await.1, answer);
    let (status, answer) = app::call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id":"clear", "anchor":null})),
    )
    .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer, json!({"anchor":null}));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_turns_carried_context_is_read_over_http() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "hello").await;
    app::wait_terminal(&app, &op).await;
    let (status, answer) =
        app::call(&app, "GET", &format!("/api/operations/{op}/context"), None).await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["heard"], true);
    assert!(
        answer["carried"]["text"]
            .as_str()
            .unwrap()
            .contains("Seeded part")
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_turn_that_carried_nothing_answers_not_carried() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "/compact").await;
    app::wait_terminal(&app, &op).await;
    let (status, answer) =
        app::call(&app, "GET", &format!("/api/operations/{op}/context"), None).await;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["code"], "CONTEXT_NOT_CARRIED");
    assert!(
        !answer["message"]
            .as_str()
            .unwrap()
            .contains("already heard")
    );
    app::shut_down_app(app).await;
}
