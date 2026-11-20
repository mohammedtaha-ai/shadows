//! Shadows' MCP server at `/mcp` as a transport: a bearer grant
//! or 401, the daemon's guard before it, the tool list a grant's kind holds,
//! and both lifecycles Claude Code speaks — through `rmcp`'s own client, and
//! through raw HTTP where the transport itself is the subject. What each tool
//! does is `mcp_tools.rs`.

use serde_json::{Value, json};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use listening::{Listening, connect, listening_app, project_client, thread_client};
use plan::{draft, issue_grant, revoke_grant};

const INITIALIZE: &str = concat!(
    r#"{"jsonrpc":"2.0","id":0,"method":"initialize","#,
    r#""params":{"protocolVersion":"2025-11-25","capabilities":{},"#,
    r#""clientInfo":{"name":"raw","version":"1"}}}"#,
);

/// A raw `POST /mcp` of `body`, with `bearer` when given and `extra` headers.
async fn raw_post(
    l: &Listening,
    bearer: Option<&str>,
    extra: &[(&str, &str)],
    body: &str,
) -> reqwest::Response {
    let mut request = reqwest::Client::new()
        .post(format!("{}/mcp", l.base))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .body(body.to_string());
    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }
    for (name, value) in extra {
        request = request.header(*name, *value);
    }
    request.send().await.unwrap()
}

#[tokio::test]
async fn a_request_without_a_token_is_401() {
    let l = listening_app().await;
    for bearer in [None, Some("shd_not-a-token")] {
        let response = raw_post(&l, bearer, &[], INITIALIZE).await;
        assert_eq!(response.status(), 401, "bearer {bearer:?}");
        assert!(response.headers().get("www-authenticate").is_none());
        assert_eq!(response.text().await.unwrap(), "");
    }

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_revoked_token_is_401() {
    let l = listening_app().await;
    let (grant, token) = issue_grant(&l.app, "thread", &l.app.project, Some(&l.app.thread)).await;
    assert_eq!(
        raw_post(&l, Some(&token), &[], INITIALIZE).await.status(),
        200
    );
    revoke_grant(&l.app, &grant).await;
    let response = raw_post(&l, Some(&token), &[], INITIALIZE).await;
    assert_eq!(response.status(), 401);
    assert_eq!(response.text().await.unwrap(), "");
    // A revoked external connection is refused the same way.
    let project = l.app.project.clone();
    let (connection, token) = connect(&l, std::slice::from_ref(&project)).await;
    assert_eq!(
        raw_post(&l, Some(&token), &[], INITIALIZE).await.status(),
        200
    );
    l.app
        .core
        .grants()
        .revoke_external("revoke-connection".into(), &connection)
        .await
        .unwrap();
    let response = raw_post(&l, Some(&token), &[], INITIALIZE).await;
    assert_eq!(response.status(), 401);
    assert_eq!(response.text().await.unwrap(), "");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_foreign_origin_is_403() {
    let l = listening_app().await;
    let (_, token) = connect(&l, std::slice::from_ref(&l.app.project)).await;
    let origin = [("origin", "http://evil.example")];
    let response = raw_post(&l, Some(&token), &origin, INITIALIZE).await;
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "ORIGIN_REFUSED");
    // The guard runs before the grant is looked at: no token, still 403.
    assert_eq!(raw_post(&l, None, &origin, INITIALIZE).await.status(), 403);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_tool_list_depends_on_the_grant_kind() {
    let l = listening_app().await;
    let names = |tools: Vec<rmcp::model::Tool>| {
        tools
            .into_iter()
            .map(|t| t.name.to_string())
            .collect::<Vec<_>>()
    };
    let planner = thread_client(&l, &l.app.thread).await;
    let mut thread_tools = names(planner.list_all_tools().await.unwrap());
    thread_tools.sort();
    assert_eq!(
        thread_tools,
        [
            "agreement_edit",
            "agreement_get",
            "agreement_list",
            "agreement_review",
            "agreement_start",
            "code_lsp",
            "decision_list",
            "decision_propose",
            "design_element_delete",
            "design_element_put",
            "design_elements",
            "draft_start",
            "executor_catalog",
            "map_approve",
            "map_propose",
            "message_agent",
            "person_ask",
            "plan_approve",
            "plan_edit",
            "plan_show",
            "project_change_note",
            "project_files",
            "project_git_init",
            "project_refresh",
            "project_state",
            "roadmap_approve",
            "roadmap_propose",
            "task_amend",
            "task_checks",
            "task_context",
            "task_diff",
            "task_done",
            "task_executor_set",
            "task_file",
            "task_get",
            "task_lessons",
            "task_ready",
            "task_runs",
            "task_start",
            "task_stop",
            "vision_approve",
            "vision_propose",
            "workflow_get",
            "workflow_list",
            "workspace_get"
        ]
    );
    let (_, external) = project_client(&l).await;
    let mut project_tools = names(external.list_all_tools().await.unwrap());
    project_tools.sort();
    assert_eq!(
        project_tools,
        [
            "agreement_edit",
            "agreement_get",
            "agreement_list",
            "agreement_review",
            "agreement_start",
            "code_lsp",
            "draft_prepare",
            "draft_start",
            "plan_edit",
            "project_list",
            "task_context",
            "task_get",
            "workflow_get",
            "workflow_list",
            "workspace_get"
        ]
    );
    // An external connection selects its project on every call but project_list.
    let tools = external.list_all_tools().await.unwrap();
    for tool in &tools {
        let required = tool
            .input_schema
            .get("required")
            .cloned()
            .unwrap_or_default();
        let selects = required
            .as_array()
            .is_some_and(|r| r.contains(&json!("project_id")));
        assert_eq!(selects, tool.name != "project_list", "{}", tool.name);
    }
    // A tool outside the grant's list is unknown to that client: rmcp's own
    // JSON-RPC "tool not found", not a tool result.
    let params = rmcp::model::CallToolRequestParams::new("draft_prepare")
        .with_arguments(serde_json::Map::new());
    let error = planner.call_tool(params).await.unwrap_err();
    assert!(error.to_string().contains("tool not found"), "{error}");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// Claude Code's fallback: `initialize` at 2025-11-25,
/// then every request carrying `MCP-Protocol-Version`, and never a session id.
#[tokio::test]
async fn the_legacy_initialize_lifecycle_works_without_a_session() {
    let l = listening_app().await;
    let (_, token) = issue_grant(&l.app, "thread", &l.app.project, Some(&l.app.thread)).await;

    let response = raw_post(&l, Some(&token), &[], INITIALIZE).await;
    assert_eq!(response.status(), 200);
    assert!(response.headers().get("mcp-session-id").is_none());
    let answer: Value = response.json().await.unwrap();
    assert_eq!(
        answer["result"]["protocolVersion"], "2025-11-25",
        "{answer}"
    );

    let version = [("mcp-protocol-version", "2025-11-25")];
    let initialized = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let response = raw_post(&l, Some(&token), &version, initialized).await;
    assert!(response.status().is_success(), "{}", response.status());

    let list = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let response = raw_post(&l, Some(&token), &version, list).await;
    assert_eq!(response.status(), 200);
    assert!(response.headers().get("mcp-session-id").is_none());
    let answer: Value = response.json().await.unwrap();
    let mut names: Vec<&str> = answer["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "agreement_edit",
            "agreement_get",
            "agreement_list",
            "agreement_review",
            "agreement_start",
            "code_lsp",
            "decision_list",
            "decision_propose",
            "design_element_delete",
            "design_element_put",
            "design_elements",
            "draft_start",
            "executor_catalog",
            "map_approve",
            "map_propose",
            "message_agent",
            "person_ask",
            "plan_approve",
            "plan_edit",
            "plan_show",
            "project_change_note",
            "project_files",
            "project_git_init",
            "project_refresh",
            "project_state",
            "roadmap_approve",
            "roadmap_propose",
            "task_amend",
            "task_checks",
            "task_context",
            "task_diff",
            "task_done",
            "task_executor_set",
            "task_file",
            "task_get",
            "task_lessons",
            "task_ready",
            "task_runs",
            "task_start",
            "task_stop",
            "vision_approve",
            "vision_propose",
            "workflow_get",
            "workflow_list",
            "workspace_get"
        ]
    );

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// Claude Code's first path: `server/discover` at
/// 2026-07-28, then requests that each carry the version, the client and its
/// capabilities in `_meta`. No `initialize`, and no session id.
#[tokio::test]
async fn the_discover_lifecycle_works_without_a_session() {
    let l = listening_app().await;
    let started = draft(&l.app).await;
    let (_, token) = issue_grant(&l.app, "thread", &l.app.project, Some(&l.app.thread)).await;
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "claude-code", "version": "2.1.281" },
        "io.modelcontextprotocol/clientCapabilities": { "roots": { "listChanged": true } }
    });
    let request = |id: u32, method: &str, params: Value| {
        let mut params = params;
        params["_meta"] = meta.clone();
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    };
    let headers = |method: &'static str| {
        [
            ("mcp-protocol-version", "2026-07-28"),
            ("mcp-method", method),
        ]
    };

    let discover = request(1, "server/discover", json!({}));
    let response = raw_post(&l, Some(&token), &headers("server/discover"), &discover).await;
    assert_eq!(response.status(), 200);
    assert!(response.headers().get("mcp-session-id").is_none());
    let answer: Value = response.json().await.unwrap();
    let versions = answer["result"]["supportedVersions"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"));
    assert!(versions.contains(&json!("2026-07-28")), "{answer}");

    let list = request(2, "tools/list", json!({}));
    let answer: Value = raw_post(&l, Some(&token), &headers("tools/list"), &list)
        .await
        .json()
        .await
        .unwrap();
    let tools = answer["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"));
    assert_eq!(tools.len(), 45, "{answer}");
    for tool in ["task_executor_set", "executor_catalog"] {
        assert!(tools.iter().any(|item| item["name"] == tool), "{answer}");
    }
    // The Planner proposes and lists decisions; an executor does neither.
    for name in ["decision_propose", "decision_list"] {
        assert!(tools.iter().any(|t| t["name"] == name), "{name}: {answer}");
        assert!(
            !shadows_core::Grants::tools_for(shadows_core::GrantKind::Executor).contains(&name),
            "{name}"
        );
    }
    assert_eq!(answer["result"]["cacheScope"], "private", "{answer}");
    // The Planner learns from the schema, not from a refusal, that an edit names no field.
    let propose = tools
        .iter()
        .find(|t| t["name"] == "vision_propose")
        .unwrap_or_else(|| panic!("{answer}"));
    let field = propose["inputSchema"]["properties"]["field"]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{propose}"));
    assert!(field.contains("add only"), "{propose}");
    assert!(
        propose["description"]
            .as_str()
            .unwrap()
            .contains("never field"),
        "{propose}"
    );

    let get = request(
        3,
        "tools/call",
        json!({ "name": "workflow_get", "arguments": { "workflow_id": started.workflow_id } }),
    );
    let mut call_headers = headers("tools/call").to_vec();
    call_headers.push(("mcp-name", "workflow_get"));
    let answer: Value = raw_post(&l, Some(&token), &call_headers, &get)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{answer}"));
    let plan: Value = serde_json::from_str(text).unwrap();
    assert_eq!(plan["title"], "Login");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn an_external_connection_only_reaches_its_chosen_projects() {
    let l = listening_app().await;
    let app = &l.app;
    let (other, _) = app::other_project(app).await;
    shadows_core::AppCore::design_for_tests(app.storage.clone())
        .seed_roadmap_for_tests(&other)
        .await
        .unwrap();
    let hidden_dir = tempfile::tempdir().unwrap();
    let hidden = app
        .core
        .projects()
        .create(
            "hidden-project".into(),
            "hidden",
            "Hidden",
            hidden_dir.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let (status, body) = app::call(
        app,
        "PUT",
        &format!("/api/projects/{}/code/links/{hidden}", app.project),
        Some(json!({ "command_id": "link-hidden" })),
    )
    .await;
    assert_eq!(status, 200, "{body}");

    let issued = app
        .core
        .grants()
        .issue_external("connect".into(), vec![app.project.clone(), other.clone()])
        .await
        .unwrap();
    let client = listening::mcp_client(&l.base, issued.token.as_deref().unwrap()).await;

    let tools = client.list_all_tools().await.unwrap();
    let workflow_list = tools
        .iter()
        .find(|tool| tool.name == "workflow_list")
        .unwrap();
    assert_eq!(
        workflow_list.input_schema["required"],
        json!(["project_id"])
    );
    assert!(
        !tools
            .iter()
            .any(|tool| tool.name == "plan_approve" || tool.name == "task_start")
    );
    let visible = listening::ok(&client, "project_list", json!({})).await;
    let visible: Vec<_> = visible
        .as_array()
        .unwrap()
        .iter()
        .map(|project| project["id"].as_str().unwrap())
        .collect();
    assert_eq!(visible.len(), 2);
    assert!(visible.contains(&app.project.as_str()) && visible.contains(&other.as_str()));
    for project in [&app.project, &other] {
        listening::ok(&client, "workflow_list", json!({ "project_id": project })).await;
    }
    for args in [
        json!({}),
        json!({ "project_id": hidden }),
        json!({ "project_id": app.project, "project": "hidden" }),
    ] {
        let text = listening::refused(&client, "workflow_list", args).await;
        assert!(text.starts_with("GRANT_SCOPE:"), "{text}");
    }
    let draft = listening::ok(
        &client,
        "draft_prepare",
        json!({ "project_id": app.project }),
    )
    .await;
    let draft_ref = draft["draft_ref"].as_str().unwrap();
    let args = |project: &shadows_core::ProjectId| {
        json!({
            "project_id": project, "draft_ref": draft_ref, "title": "Notes", "goal": "Specify notes"
        })
    };
    let text = listening::refused(&client, "draft_start", args(&other)).await;
    assert!(text.starts_with("GRANT_SCOPE:"), "{text}");
    let started = listening::ok(&client, "draft_start", args(&app.project)).await;
    let text = listening::refused(
        &client,
        "workflow_get",
        json!({
            "project_id": other, "workflow_id": started["workflow_id"]
        }),
    )
    .await;
    assert!(text.starts_with("GRANT_SCOPE:"), "{text}");

    let workflow: shadows_core::WorkflowId =
        serde_json::from_value(started["workflow_id"].clone()).unwrap();
    let hidden_thread = app
        .core
        .threads()
        .create("hidden-thread".into(), &hidden, "Hidden", None, None)
        .await
        .unwrap();
    let hidden_plan = plan::draft_on(app, &hidden_thread.id, "hidden-plan").await;
    let private_title = "Foreign plan internal title";
    plan::edit(
        app,
        &hidden_plan.workflow_id,
        0,
        &[
            shadows_core::PlanOp::PlanPut {
                title: private_title.into(),
                goal: "Private goal".into(),
            },
            plan::add(1),
        ],
    )
    .await;
    plan::edit(app, &workflow, 0, &[plan::add(1)]).await;
    let link = shadows_core::PlanOp::LinkPut {
        link: shadows_core::Link {
            task: 1,
            after: shadows_core::TaskParent::Plan {
                plan_id: hidden_plan.plan_id,
                task: 1,
            },
            kind: shadows_core::LinkKind::Needs,
            label: "external prerequisite".into(),
            waiting_items: vec![],
        },
    };
    let text = listening::refused(&client, "plan_edit", json!({
        "project_id": app.project, "workflow_id": workflow, "expected_revision": 1, "ops": [link],
    })).await;
    assert!(text.starts_with("GRANT_SCOPE:"), "{text}");
    plan::edit(app, &workflow, 1, &[link]).await;
    plan::edit(
        app,
        &hidden_plan.workflow_id,
        1,
        &[shadows_core::PlanOp::TaskRemove { number: 1 }],
    )
    .await;
    let view = listening::ok(
        &client,
        "workflow_get",
        json!({
            "project_id": app.project, "workflow_id": workflow
        }),
    )
    .await;
    assert!(
        view.get("linked_tasks")
            .is_none_or(|links| links == &json!([]))
    );
    assert!(!view["blockers"].to_string().contains(private_title));
    assert!(
        view["blockers"]
            .to_string()
            .contains("outside the selected project")
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
