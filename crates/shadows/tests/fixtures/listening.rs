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
use serde_json::{Value, json};
use shadows_core::ThreadId;
use shadows_core::{ExternalConnectionId, GrantId, ProjectId};

use std::path::Path;

use shadows_core::testing::SessionsConfig;

use super::acp;
use super::app::{App, test_app_unseeded_with, test_app_with};
use super::plan::issue_grant;

/// A connected MCP client. On an external connection it also holds the
/// connection and the project its tool calls select, as an agent would after
/// `project_list`.
pub struct Client {
    service: RunningService<RoleClient, ()>,
    connection: Option<ExternalConnectionId>,
    project: Option<ProjectId>,
}

impl std::ops::Deref for Client {
    type Target = RunningService<RoleClient, ()>;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

impl Client {
    pub async fn cancel(self) -> Result<rmcp::service::QuitReason, tokio::task::JoinError> {
        self.service.cancel().await
    }

    /// The connection this client authenticates with, when it is external.
    pub fn connection(&self) -> &ExternalConnectionId {
        self.connection.as_ref().expect("an external connection")
    }
}

pub struct Listening {
    pub app: App,
    /// `http://127.0.0.1:<port>`, without a trailing slash.
    pub base: String,
}

/// `test_app()`, listening on `127.0.0.1` at a port the system chose, its
/// sessions opening with this `/mcp`. The server runs until the
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

/// As `listening_app`, with the project at its start (see `test_app_unseeded`).
pub async fn listening_unseeded() -> Listening {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = listen(tmp.path(), acp::test_config(), false).await;
    l.app = l.app.owning(tmp);
    l
}

/// As `listening_app`, its sessions on `adapter`; the project folder is `dir`.
pub async fn listening_on(
    dir: &Path,
    adapter: std::sync::Arc<shadows_agent::claude::ClaudeAdapter>,
) -> Listening {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let mcp_url = format!("{base}/mcp");
    let config = SessionsConfig {
        mcp_url: Some(mcp_url.clone()),
        ..acp::test_config()
    };
    let app = super::app::test_app_on(dir, config, &mcp_url, adapter).await;
    let router = app.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Listening { app, base }
}

/// As `listening_with`, on the database in `dir` (see `test_app_at`).
pub async fn listening_at(dir: &Path, config: SessionsConfig) -> Listening {
    listen(dir, config, true).await
}

async fn listen(dir: &Path, config: SessionsConfig, roadmap: bool) -> Listening {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let mcp_url = format!("{base}/mcp");
    let config = SessionsConfig {
        mcp_url: Some(mcp_url.clone()),
        ..config
    };
    let app = match roadmap {
        true => test_app_with(dir, config, &mcp_url).await,
        false => test_app_unseeded_with(dir, config, &mcp_url).await,
    };
    let router = app.router.clone();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Listening { app, base }
}

/// `rmcp`'s own client on `{base}/mcp`, sending `token` as its bearer, after
/// whichever lifecycle it negotiates with the server.
pub async fn mcp_client(base: &str, token: &str) -> Client {
    let config =
        StreamableHttpClientTransportConfig::with_uri(format!("{base}/mcp")).auth_header(token);
    let service =
        ().serve(StreamableHttpClientTransport::from_config(config))
            .await
            .expect("the MCP client connects");
    Client {
        service,
        connection: None,
        project: None,
    }
}

/// `(is_error, text)` of a tool call that the server answered with a result.
/// An external client's call names its selected project unless `args` sets
/// `project_id` itself (`null` to send none).
pub async fn call(client: &Client, tool: &str, args: Value) -> (bool, String) {
    let mut args = args.as_object().expect("arguments are an object").clone();
    if let Some(project) = &client.project
        && tool != "project_list"
    {
        args.entry("project_id").or_insert_with(|| json!(project));
    }
    let params = rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(args);
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

/// A new external connection to `projects`, as the person issues one in
/// Settings: its id and its bearer token.
pub async fn connect(l: &Listening, projects: &[ProjectId]) -> (ExternalConnectionId, String) {
    let issued = l
        .app
        .core
        .grants()
        .issue_external(
            format!("connect-{}", uuid::Uuid::new_v4()),
            None,
            projects.to_vec(),
        )
        .await
        .unwrap();
    let token = issued.token.expect("a new connection shows its token");
    (issued.connection.id, token)
}

/// An external agent's client: a new connection to `projects`, its calls
/// selecting `project`. Answers the connection's grant on `project`, which
/// writes as the agent.
pub async fn connection_client(
    l: &Listening,
    projects: &[ProjectId],
    project: &ProjectId,
) -> (GrantId, Client) {
    let (connection, token) = connect(l, projects).await;
    let grant = l
        .app
        .core
        .grants()
        .external_project(&connection, project)
        .await
        .unwrap()
        .id;
    let mut client = mcp_client(&l.base, &token).await;
    client.connection = Some(connection);
    client.project = Some(project.clone());
    (grant, client)
}

/// An external agent's client, connected to the app's project only.
pub async fn project_client(l: &Listening) -> (GrantId, Client) {
    let project = l.app.project.clone();
    connection_client(l, std::slice::from_ref(&project), &project).await
}

/// The person revokes `client`'s connection: its access stops.
pub async fn revoke_connection(l: &Listening, client: &Client) {
    l.app
        .core
        .grants()
        .revoke_external(
            format!("revoke-{}", uuid::Uuid::new_v4()),
            client.connection(),
        )
        .await
        .unwrap();
}
