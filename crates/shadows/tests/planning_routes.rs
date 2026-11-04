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

async fn reported_blocks(app: &app::App) -> Vec<String> {
    let op = app::start_settled(app, "report").await;
    app::wait_terminal(app, &op).await;
    let reply: serde_json::Value =
        serde_json::from_str(&app::last_agent_entry(app).await.body).unwrap();
    reply["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block.as_str().unwrap().to_owned())
        .collect()
}

fn carries_context(blocks: &[String]) -> bool {
    blocks
        .iter()
        .any(|block| block.contains("## Planning context index"))
}

#[tokio::test]
async fn a_resend_makes_the_next_turn_carry_the_context() {
    let app = app::test_app().await;
    assert!(carries_context(&reported_blocks(&app).await));
    assert!(!carries_context(&reported_blocks(&app).await));
    let path = format!("/api/threads/{}/context/resend", app.thread);
    let (status, answer) = app::post(&app, &path, json!({"command_id":"resend"})).await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer, json!({"owed":true}));
    assert!(carries_context(&reported_blocks(&app).await));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_resend_on_an_aylul_thread_is_refused() {
    let app = app::test_app().await;
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        &app.storage,
        &app::ctx("aylul", "thread.create"),
        &app.project,
        "Assistant",
        "aylul",
        None,
    )
    .await
    .unwrap();
    let path = format!("/api/threads/{}/context/resend", thread.id);
    let (status, answer) = app::post(&app, &path, json!({"command_id":"resend"})).await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(answer["code"], "INVALID_COMMAND");
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_resend_replay_answers_the_first_result() {
    let app = app::test_app().await;
    let path = format!("/api/threads/{}/context/resend", app.thread);
    let first = app::post(&app, &path, json!({"command_id":"same"})).await;
    let replay = app::post(&app, &path, json!({"command_id":"same"})).await;
    assert_eq!(first, (200, json!({"owed":true})));
    assert_eq!(replay, first);
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM context_forget")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(rows, 1);
    let thread = app.thread.as_str().to_owned();
    app.storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let other = app::ctx("taken", "thread.other");
                shadows_core::testing::CommandContext::record_command(
                    conn, &other, "Thread", &thread, "X", "1", "t",
                )
                .await
            })
        })
        .await
        .unwrap();
    let (status, answer) = app::post(&app, &path, json!({"command_id":"taken"})).await;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["code"], "COMMAND_CONFLICT");
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_resend_on_a_missing_thread_is_not_found() {
    let app = app::test_app().await;
    let path = "/api/threads/no-such-thread/context/resend";
    let (status, answer) = app::post(&app, path, json!({"command_id":"resend"})).await;
    assert_eq!(status, 404, "{answer}");
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM context_forget")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(rows, 0);
    app::shut_down_app(app).await;
}
