//! Contract conflicts over the real adapters: the person's read of an
//! attempt's conflict (`TaskConflict` or null), the executor's
//! `task_conflict` and the Planner's `task_stop`, each only for its own grant
//! kind.

use serde_json::{Value, json};

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

/// Records an ended attempt `id` at T1 of `workflow`, with the conflict
/// columns `conflict` sets (or none) and released at `released`.
async fn attempt_row(
    app: &app::App,
    workflow: &str,
    id: &str,
    number: i64,
    conflict: Option<&str>,
    released: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO run_attempts
           (id, project_id, workflow_id, version, task, number, thread_id, model, state,
            writes_json, started_at, ended_at, released_at, command_id, fingerprint,
            conflict_json, conflict_at, conflict_delivery)
         VALUES (?, ?, ?, 1, 1, ?, 'gone', 'fake-small', 'stopped', '[\"src/**\"]',
                 '2026-11-14T00:00:00Z', '2026-11-14T00:02:00Z', ?, ?, 'f', ?,
                 CASE WHEN ? IS NULL THEN NULL ELSE '2026-11-14T00:01:00Z' END,
                 CASE WHEN ? IS NULL THEN NULL ELSE 'no_planner' END)",
    )
    .bind(id)
    .bind(app.project.as_str())
    .bind(workflow)
    .bind(number)
    .bind(released)
    .bind(format!("cmd-{id}"))
    .bind(conflict)
    .bind(conflict)
    .bind(conflict)
    .execute(app.storage.reader())
    .await
    .unwrap();
}

#[tokio::test]
async fn an_attempts_conflict_is_read_or_null() {
    let l = listening_app().await;
    let workflow = approved_v1(&l.app).await;
    let conflict = json!({
        "expected": "find answers by id",
        "observed": "no id column",
        "evidence": [{ "path": "src/find.rs", "line": 3, "summary": "no argument" }],
    })
    .to_string();
    attempt_row(
        &l.app,
        workflow.as_str(),
        "plain",
        1,
        None,
        Some("2026-11-14T00:03:00Z"),
    )
    .await;
    attempt_row(&l.app, workflow.as_str(), "held", 2, Some(&conflict), None).await;
    let (status, body) = app::call(&l.app, "GET", "/api/attempts/nope/conflict", None).await;
    assert_eq!(status, 404, "{body}");
    let (status, body) = app::call(&l.app, "GET", "/api/attempts/plain/conflict", None).await;
    assert_eq!((status, body), (200, Value::Null));
    let (status, body) = app::call(&l.app, "GET", "/api/attempts/held/conflict", None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["attempt_id"], "held");
    assert_eq!(body["workflow_id"], workflow.as_str());
    assert_eq!(body["project_id"], l.app.project.as_str());
    assert_eq!(
        (body["version"].clone(), body["task"].clone()),
        (json!(1), json!(1))
    );
    assert_eq!(body["attempt_state"], "stopped");
    assert_eq!(body["scope_held"], true);
    assert_eq!(body["delivery"], "no_planner");
    assert_eq!(body["evidence"][0]["line"], 3);
    assert_eq!(body["created_at"], "2026-11-14T00:01:00Z");
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn only_an_executor_reports_and_only_a_planner_stops() {
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
    let conflict = tools
        .iter()
        .find(|tool| tool.name == "task_conflict")
        .expect("an executor reports task_conflict");
    let schema = serde_json::to_value(&conflict.input_schema).unwrap();
    for field in ["command_id", "expected", "observed", "evidence"] {
        assert!(
            schema["properties"].get(field).is_some(),
            "{field}: {schema}"
        );
    }
    let description = conflict.description.as_deref().unwrap();
    for text in ["end your turn", "task_report"] {
        assert!(description.contains(text), "{text}: {description}");
    }
    assert!(tools.iter().all(|t| t.name != "task_stop"));
    // The grant holds no attempt: nothing of its own to report on.
    let args = json!({
        "command_id": "c1",
        "expected": "find answers by id",
        "observed": "no id column",
        "evidence": [{ "path": "src/find.rs", "line": 3, "summary": "no argument" }],
    });
    let text = refused(&executor, "task_conflict", args.clone()).await;
    assert!(text.starts_with("GRANT_SCOPE"), "{text}");

    let planner = thread_client(&l, &l.app.thread).await;
    let listed = planner.list_all_tools().await.unwrap();
    let stop = listed
        .iter()
        .find(|tool| tool.name == "task_stop")
        .expect("the Planner stops with task_stop");
    let schema = serde_json::to_value(&stop.input_schema).unwrap();
    for field in ["command_id", "attempt_id"] {
        assert!(
            schema["properties"].get(field).is_some(),
            "{field}: {schema}"
        );
    }
    assert!(listed.iter().all(|t| t.name != "task_conflict"));
    let text = refused(
        &planner,
        "task_stop",
        json!({ "command_id": "s1", "attempt_id": "nope" }),
    )
    .await;
    assert!(text.starts_with("GRANT_SCOPE"), "{text}");

    let (_, external) = project_client(&l).await;
    let listed = external.list_all_tools().await.unwrap();
    assert!(
        listed
            .iter()
            .all(|t| t.name != "task_stop" && t.name != "task_conflict")
    );
    for (tool, args) in [
        ("task_conflict", args),
        (
            "task_stop",
            json!({ "command_id": "s1", "attempt_id": "nope" }),
        ),
    ] {
        let params = rmcp::model::CallToolRequestParams::new(tool)
            .with_arguments(args.as_object().unwrap().clone());
        let called = external.call_tool(params).await;
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
