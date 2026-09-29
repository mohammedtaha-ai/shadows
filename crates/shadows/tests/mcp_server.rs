//! Shadows' MCP server at `/mcp` as a transport (spec §13.6): a bearer grant
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

use listening::{Listening, listening_app, project_client, thread_client};
use plan::{draft, issue_grant, revoke_grant};

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"raw","version":"1"}}}"#;

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
}

#[tokio::test]
async fn a_revoked_token_is_401() {
    let l = listening_app().await;
    let (grant, token) = issue_grant(&l.app, "project", &l.app.project, None).await;
    assert_eq!(
        raw_post(&l, Some(&token), &[], INITIALIZE).await.status(),
        200
    );
    revoke_grant(&l.app, &grant).await;
    let response = raw_post(&l, Some(&token), &[], INITIALIZE).await;
    assert_eq!(response.status(), 401);
    assert_eq!(response.text().await.unwrap(), "");
}

#[tokio::test]
async fn a_foreign_origin_is_403() {
    let l = listening_app().await;
    let (_, token) = issue_grant(&l.app, "project", &l.app.project, None).await;
    let origin = [("origin", "http://evil.example")];
    let response = raw_post(&l, Some(&token), &origin, INITIALIZE).await;
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "ORIGIN_REFUSED");
    // The guard runs before the grant is looked at: no token, still 403.
    assert_eq!(raw_post(&l, None, &origin, INITIALIZE).await.status(), 403);
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
            "draft_start",
            "plan_edit",
            "plan_show",
            "task_get",
            "workflow_get"
        ]
    );
    let (_, external) = project_client(&l).await;
    let mut project_tools = names(external.list_all_tools().await.unwrap());
    project_tools.sort();
    assert_eq!(
        project_tools,
        [
            "draft_prepare",
            "draft_start",
            "plan_edit",
            "task_get",
            "workflow_get",
            "workflow_list"
        ]
    );
    // A tool outside the grant's list is unknown to that client: rmcp's own
    // JSON-RPC "tool not found", not a tool result.
    let params = rmcp::model::CallToolRequestParams::new("workflow_list")
        .with_arguments(serde_json::Map::new());
    let error = planner.call_tool(params).await.unwrap_err();
    assert!(error.to_string().contains("tool not found"), "{error}");
}

/// Claude Code's fallback (MCP_PROBE.md §4): `initialize` at 2025-11-25,
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
            "draft_start",
            "plan_edit",
            "plan_show",
            "task_get",
            "workflow_get"
        ]
    );
}

/// Claude Code's first path (MCP_PROBE.md §4): `server/discover` at
/// 2026-07-28, then requests that each carry the version, the client and its
/// capabilities in `_meta`. No `initialize`, and no session id.
#[tokio::test]
async fn the_discover_lifecycle_works_without_a_session() {
    let l = listening_app().await;
    draft(&l.app).await;
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
    assert_eq!(tools.len(), 5, "{answer}");
    assert_eq!(answer["result"]["cacheScope"], "private", "{answer}");

    let get = request(
        3,
        "tools/call",
        json!({ "name": "workflow_get", "arguments": {} }),
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
}
