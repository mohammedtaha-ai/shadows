//! Shared apparatus: the in-process daemon of `app.rs`, served on a real
//! loopback port, for clients that speak HTTP themselves — an MCP client
//! above all — and the tool calls such a client makes. Include beside
//! `acp.rs`, `app.rs` and `plan.rs`:
//!
//! ```ignore
//! #[path = "fixtures/listening.rs"] mod listening;
//! ```

#![allow(dead_code)]

use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;
use shadows_core::GrantId;
use shadows_core::ThreadId;

use std::path::Path;

use shadows_core::testing::SessionsConfig;

use super::acp;
use super::app::{App, test_app_with};
use super::plan::issue_grant;

/// A connected MCP client, as `mcp_client` answers it.
pub type Client = RunningService<RoleClient, ()>;

pub struct Listening {
    pub app: App,
    /// `http://127.0.0.1:<port>`, without a trailing slash.
    pub base: String,
}

/// `test_app()`, listening on `127.0.0.1` at a port the system chose, its
/// sessions opening with this `/mcp` (§13.8). The server runs until the
/// test's runtime ends.
pub async fn listening_app() -> Listening {
    listening_with(acp::test_config()).await
}

/// As `listening_app`, with the sessions' own configuration.
pub async fn listening_with(config: SessionsConfig) -> Listening {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = listening_at(tmp.path(), config).await;
    l.app = l.app.owning(tmp);
    l
}

/// As `listening_with`, on the database in `dir` (see `test_app_at`).
pub async fn listening_at(dir: &Path, config: SessionsConfig) -> Listening {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let mcp_url = format!("{base}/mcp");
    let config = SessionsConfig {
        mcp_url: Some(mcp_url.clone()),
        ..config
    };
    let app = test_app_with(dir, config, &mcp_url).await;
    let router = app.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Listening { app, base }
}

/// `rmcp`'s own client on `{base}/mcp`, sending `token` as its bearer, after
/// whichever lifecycle it negotiates with the server.
pub async fn mcp_client(base: &str, token: &str) -> Client {
    let config =
        StreamableHttpClientTransportConfig::with_uri(format!("{base}/mcp")).auth_header(token);
    ().serve(StreamableHttpClientTransport::from_config(config))
        .await
        .expect("the MCP client connects")
}

/// `(is_error, text)` of a tool call that the server answered with a result.
pub async fn call(client: &Client, tool: &str, args: Value) -> (bool, String) {
    let params = rmcp::model::CallToolRequestParams::new(tool.to_string())
        .with_arguments(args.as_object().expect("arguments are an object").clone());
    let result = client.call_tool(params).await.expect("a tool result");
    let text = result.content[0]
        .as_text()
        .expect("text content")
        .text
        .clone();
    (result.is_error == Some(true), text)
}

/// The JSON a tool call answered, asserting it succeeded.
pub async fn ok(client: &Client, tool: &str, args: Value) -> Value {
    let (is_error, text) = call(client, tool, args).await;
    assert!(!is_error, "{tool} failed: {text}");
    serde_json::from_str(&text).unwrap()
}

/// The text of a refused tool call, asserting it was refused.
pub async fn refused(client: &Client, tool: &str, args: Value) -> String {
    let (is_error, text) = call(client, tool, args).await;
    assert!(is_error, "{tool} was not refused: {text}");
    text
}

/// A client holding a new grant on `thread`: the Planner's.
pub async fn thread_client(l: &Listening, thread: &ThreadId) -> Client {
    let (_, token) = issue_grant(&l.app, "thread", &l.app.project, Some(thread)).await;
    mcp_client(&l.base, &token).await
}

/// A client holding a new grant on the app's project: an external agent's.
pub async fn project_client(l: &Listening) -> (GrantId, Client) {
    let (grant, token) = issue_grant(&l.app, "project", &l.app.project, None).await;
    (grant, mcp_client(&l.base, &token).await)
}
