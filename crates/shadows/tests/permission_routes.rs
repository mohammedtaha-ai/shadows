//! Permission routes through the real AppCore and synthetic ACP adapter.
use serde_json::{Value, json};
use shadows_core::testing::{SessionsConfig, acp};
use shadows_core::{AgentInput, AylulConfig, ThreadId};
use std::time::Duration;

#[path = "fixtures/app.rs"]
mod app;
use app::{App, call};

async fn world() -> (App, ThreadId) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let app = app::test_app_with(
        tmp.path(),
        SessionsConfig {
            aylul: Some(AylulConfig {
                home_root: home.clone(),
                project_slug: "aylul".into(),
            }),
            ..acp::test_config()
        },
        acp::MCP_URL,
    )
    .await
    .owning(tmp);
    let project = app
        .core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &home)
        .await
        .unwrap();
    let provider = app
        .core
        .providers()
        .create_provider(
            "provider".into(),
            serde_json::from_value(json!({"name":"Local","base_url":"https://example.com/api",
            "models":[{"id":"fake-large","context_window":1000000}]}))
            .unwrap(),
        )
        .await
        .unwrap();
    app.core
        .providers()
        .set_key("key".into(), &provider.slug, "synthetic-provider-key")
        .await
        .unwrap();
    let agent = app
        .core
        .agents()
        .create_agent(
            "agent".into(),
            serde_json::from_value::<AgentInput>(json!({"name":"AYLUL","provider":provider.slug,
            "default_model":"fake-large"}))
            .unwrap(),
        )
        .await
        .unwrap();
    let thread = app
        .core
        .threads()
        .create(
            "aylul-thread".into(),
            &project.id,
            "Chat",
            Some("aylul"),
            Some(&agent.slug),
        )
        .await
        .unwrap()
        .id;
    (app, thread)
}

async fn start(app: &App, thread: &ThreadId) -> shadows_core::OperationId {
    app::start_on(
        app,
        thread.as_str(),
        "ask-permission",
        json!({"model":"fake-large","effort":"high","mode":"acceptEdits"}),
    )
    .await
}

async fn waiting(app: &App, count: usize) -> Vec<Value> {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let (status, list) = call(app, "GET", "/api/aylul/permissions", None).await;
            assert_eq!(status, 200, "{list}");
            let list = list.as_array().unwrap();
            if list.len() == count {
                return list.clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

fn answer_path(thread: &ThreadId, request: &str) -> String {
    format!("/api/threads/{thread}/session/permissions/{request}")
}

async fn close(app: &App) {
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn no_aylul_lists_nothing() {
    let app = app::test_app().await;
    let before = shadows_core::Events::current_cursor_for_test(&app.storage)
        .await
        .unwrap();
    let (status, list) = call(&app, "GET", "/api/aylul/permissions", None).await;
    assert_eq!(status, 200);
    assert_eq!(list, json!([]));
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&app.storage)
            .await
            .unwrap(),
        before
    );
    close(&app).await;
}

#[tokio::test]
async fn answering_over_http_answers_the_adapter() {
    let (app, thread) = world().await;
    let op = start(&app, &thread).await;
    let list = waiting(&app, 1).await;
    let id = list[0]["request_id"].as_str().unwrap();
    assert_eq!(list[0]["thread_id"], thread.as_str());
    assert_eq!(list[0]["agent"], "aylul");
    let (status, answer) = call(
        &app,
        "POST",
        &answer_path(&thread, id),
        Some(json!({"command_id":"answer","option_id":"allow_once"})),
    )
    .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["option_id"], "allow_once");
    assert_eq!(answer["kind"], "allow_once");
    app::wait_terminal(&app, &op).await;
    let entries = app.core.threads().entries(&thread).await.unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.body.contains("permission: allow_once"))
    );
    assert!(waiting(&app, 0).await.is_empty());
    close(&app).await;
}

#[tokio::test]
async fn unknown_settled_and_not_offered_answer_their_codes() {
    let (app, thread) = world().await;
    let op = start(&app, &thread).await;
    let list = waiting(&app, 1).await;
    let id = list[0]["request_id"].as_str().unwrap();
    for (path, command, option, status, code) in [
        (
            answer_path(&thread, "unknown"),
            "unknown",
            "allow_once",
            404,
            "PERMISSION_NOT_FOUND",
        ),
        (
            answer_path(&app.thread, id),
            "wrong-thread",
            "allow_once",
            404,
            "PERMISSION_NOT_FOUND",
        ),
        (
            answer_path(&thread, id),
            "bad-option",
            "invented",
            422,
            "PERMISSION_OPTION_NOT_OFFERED",
        ),
    ] {
        let (actual, body) = call(
            &app,
            "POST",
            &path,
            Some(json!({"command_id":command,"option_id":option})),
        )
        .await;
        assert_eq!(actual, status, "{body}");
        assert_eq!(body["code"], code);
    }
    let (status, body) = call(
        &app,
        "POST",
        &answer_path(&thread, id),
        Some(json!({"command_id":"first","option_id":"allow_once"})),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = call(
        &app,
        "POST",
        &answer_path(&thread, id),
        Some(json!({"command_id":"second","option_id":"allow_once"})),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "PERMISSION_SETTLED");
    app::wait_terminal(&app, &op).await;
    close(&app).await;
}

#[tokio::test]
async fn a_replayed_answer_answers_the_first_result() {
    let (app, thread) = world().await;
    let op = start(&app, &thread).await;
    let list = waiting(&app, 1).await;
    let path = answer_path(&thread, list[0]["request_id"].as_str().unwrap());
    let body = json!({"command_id":"replay","option_id":"reject_once"});
    let first = call(&app, "POST", &path, Some(body.clone())).await;
    assert_eq!(first.0, 200, "{}", first.1);
    app::wait_terminal(&app, &op).await;
    app.sessions.terminate(&thread).await.unwrap();
    assert_eq!(call(&app, "POST", &path, Some(body)).await, first);
    let (status, error) = call(
        &app,
        "POST",
        &path,
        Some(json!({"command_id":"replay","option_id":"allow_once"})),
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(error["code"], "COMMAND_CONFLICT");
    close(&app).await;
}

#[tokio::test]
async fn waiting_requests_are_listed_oldest_first() {
    let (app, thread) = world().await;
    let provider = app.core.agents().agent("aylul").await.unwrap().provider;
    let agent = app
        .core
        .agents()
        .create_agent(
            "second-agent".into(),
            serde_json::from_value::<AgentInput>(json!({"name":"Work","provider":provider,
            "default_model":"fake-large"}))
            .unwrap(),
        )
        .await
        .unwrap();
    let project = app.core.agents().assistant_project().await.unwrap();
    let other = app
        .core
        .threads()
        .create(
            "second-thread".into(),
            &project,
            "Second",
            Some("aylul"),
            Some(&agent.slug),
        )
        .await
        .unwrap()
        .id;
    let first = start(&app, &thread).await;
    let old = waiting(&app, 1).await[0]["request_id"].clone();
    let second = start(&app, &other).await;
    let list = waiting(&app, 2).await;
    assert_eq!(list[0]["request_id"], old);
    assert_eq!(list[1]["agent"], "work");
    app.core.turns().stop(&first).await.unwrap();
    app.core.turns().stop(&second).await.unwrap();
    close(&app).await;
}
