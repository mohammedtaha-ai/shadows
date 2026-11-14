//! `task_question` over the real `/mcp`: an executor's tool only, whose input
//! keeps the selector, the native language-server question and the consumer
//! page, and which answers only for the executor's own running attempt.

use serde_json::json;

#[allow(unused_imports)]
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use listening::{listening_app, project_client, refused, thread_client};
use plan::approved_v1;

#[tokio::test]
async fn only_an_executor_lists_task_question() {
    let l = listening_app().await;
    let workflow = approved_v1(&l.app).await;
    let (project, thread) = (l.app.project.clone(), l.app.thread.clone());
    let scope = shadows_core::testing::TaskScope {
        workflow: workflow.as_str().to_string(),
        version: 1,
        task: 1,
    };
    let (_, token) = l
        .app
        .storage
        .write_txn(move |conn| {
            Box::pin(async move {
                shadows_core::Grants::issue_executor_grant_in(conn, &project, &thread, scope).await
            })
        })
        .await
        .unwrap();
    let executor = listening::mcp_client(&l.base, token.as_str()).await;
    let tools = executor.list_all_tools().await.unwrap();
    let question = tools
        .iter()
        .find(|tool| tool.name == "task_question")
        .expect("an executor asks task_question");
    let schema = serde_json::to_value(&question.input_schema).unwrap();
    for field in ["selector", "code", "after", "limit"] {
        assert!(
            schema["properties"].get(field).is_some(),
            "{field}: {schema}"
        );
    }
    let description = question.description.as_deref().unwrap();
    for text in ["planned", "code_error", "this_frozen_version"] {
        assert!(description.contains(text), "{text}: {description}");
    }
    let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    assert!(names.contains(&"code_lsp".to_string()), "{names:?}");
    // The grant holds no running attempt: nothing of its own to ask about.
    let own = json!({ "selector": { "kind": "own" } });
    let text = refused(&executor, "task_question", own.clone()).await;
    assert!(text.starts_with("GRANT_SCOPE"), "{text}");
    let with_code = json!({
        "selector": { "kind": "symbol", "symbol_id": "s1" },
        "code": { "operation": "hover", "file": "a.go", "line": 1, "character": 1 },
        "after": 0,
        "limit": 5,
    });
    refused(&executor, "task_question", with_code).await;
    // The Planner and an external agent have no task_question.
    let planner = thread_client(&l, &l.app.thread).await;
    let (_, external) = project_client(&l).await;
    for client in [&planner, &external] {
        let listed = client.list_all_tools().await.unwrap();
        assert!(listed.iter().all(|t| t.name != "task_question"));
        let params = rmcp::model::CallToolRequestParams::new("task_question")
            .with_arguments(own.as_object().unwrap().clone());
        let called = client.call_tool(params).await;
        assert!(
            format!("{called:?}").contains("tool not found"),
            "{called:?}"
        );
    }
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
