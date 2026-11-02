//! Session opening configuration and option requests.
use super::{AcpError, Connection};
use agent_client_protocol::schema::v1::{
    EnvVariable, ForkSessionRequest, HttpHeader, McpServer, McpServerHttp, McpServerStdio, Meta,
    NewSessionRequest, ResumeSessionRequest, SessionConfigOptionValue,
    SetSessionConfigOptionRequest,
};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum SessionStart {
    New,
    Resume(String),
    Fork(String),
}

/// What a session opens with: sent alike on
/// `session/new`, `session/resume` and a fork's opening. On a resume Claude
/// Code keeps the `append` its session was created with, but reads
/// `mcp_servers` again, so a new adapter's grant reaches it.
#[derive(Debug, Clone, Default)]
pub struct SessionSetup {
    /// MCP servers supplied to the session.
    pub mcp: Vec<McpServerSpec>,
    /// Appended to Claude Code's prompt, which is kept (`_meta.systemPrompt`).
    pub append: Option<String>,
    /// Pre-approved tools (`_meta.claudeCode.options.allowedTools`).
    pub allowed_tools: Vec<String>,
    /// None keeps adapter defaults; Some(empty) explicitly removes built-in tools.
    pub tools: Option<Vec<String>>,
    /// Which settings files Claude Code loads (`settingSources`). None leaves
    /// the Planner rule (`[]` with `tools`); AYLUL sends `["user"]`.
    pub setting_sources: Option<Vec<String>>,
}

/// One MCP server configuration. Values are never included in Debug output.
#[derive(Clone)]
pub enum McpServerSpec {
    Http {
        name: String,
        url: String,
        headers: Vec<(String, String)>,
    },
    Stdio {
        name: String,
        command: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    },
}

impl std::fmt::Debug for McpServerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { name, url, .. } => f
                .debug_struct("Http")
                .field("name", name)
                .field("url", url)
                .finish_non_exhaustive(),
            Self::Stdio { name, command, .. } => f
                .debug_struct("Stdio")
                .field("name", name)
                .field("command", command)
                .finish_non_exhaustive(),
        }
    }
}

impl SessionSetup {
    pub(super) fn mcp_servers(&self) -> Vec<McpServer> {
        self.mcp
            .iter()
            .map(|m| match m {
                McpServerSpec::Http { name, url, headers } => McpServer::Http(
                    McpServerHttp::new(name, url).headers(
                        headers
                            .iter()
                            .map(|(name, value)| HttpHeader::new(name, value))
                            .collect(),
                    ),
                ),
                McpServerSpec::Stdio {
                    name,
                    command,
                    args,
                    env,
                } => McpServer::Stdio(
                    McpServerStdio::new(name, command).args(args.clone()).env(
                        env.iter()
                            .map(|(name, value)| EnvVariable::new(name, value))
                            .collect(),
                    ),
                ),
            })
            .collect()
    }

    pub(super) fn meta(&self) -> Option<Meta> {
        let mut meta = Meta::new();
        if let Some(append) = &self.append {
            meta.insert(
                "systemPrompt".into(),
                serde_json::json!({ "append": append }),
            );
        }
        let mut options = serde_json::Map::new();
        if !self.allowed_tools.is_empty() {
            options.insert("allowedTools".into(), serde_json::json!(self.allowed_tools));
        }
        if let Some(tools) = &self.tools {
            options.insert("tools".into(), serde_json::json!(tools));
            options.insert("settingSources".into(), serde_json::json!([]));
            options.insert("strictMcpConfig".into(), serde_json::json!(true));
        }
        if let Some(sources) = &self.setting_sources {
            options.insert("settingSources".into(), serde_json::json!(sources));
        }
        if !options.is_empty() {
            meta.insert(
                "claudeCode".into(),
                serde_json::json!({ "options": options }),
            );
        }
        (!meta.is_empty()).then_some(meta)
    }
}

#[derive(Debug)]
pub struct Opened {
    pub session_id: String,
    pub options: Value,
}

impl Connection {
    pub async fn start_session(
        &self,
        cwd: &Path,
        how: SessionStart,
        setup: &SessionSetup,
    ) -> Result<Opened, AcpError> {
        let fresh = matches!(&how, SessionStart::New);
        let (servers, meta) = (setup.mcp_servers(), setup.meta());
        let resume = |id: String| {
            ResumeSessionRequest::new(id, cwd)
                .mcp_servers(servers.clone())
                .meta(meta.clone())
        };
        let (session_id, options) = match how {
            SessionStart::New => {
                let request = NewSessionRequest::new(cwd)
                    .mcp_servers(servers.clone())
                    .meta(meta.clone());
                let r = self
                    .cx
                    .send_request(request)
                    .block_task()
                    .await
                    .map_err(AcpError::from_rpc)?;
                (r.session_id.to_string(), r.config_options)
            }
            SessionStart::Resume(id) => {
                let r = self
                    .cx
                    .send_request(resume(id.clone()))
                    .block_task()
                    .await
                    .map_err(AcpError::from_rpc)?;
                (id, r.config_options)
            }
            SessionStart::Fork(src) => {
                let request = ForkSessionRequest::new(src, cwd)
                    .mcp_servers(servers.clone())
                    .meta(meta.clone());
                let fork = self
                    .cx
                    .send_request(request)
                    .block_task()
                    .await
                    .map_err(AcpError::from_rpc)?;
                let id = fork.session_id.to_string();
                let r = self
                    .cx
                    .send_request(resume(id.clone()))
                    .block_task()
                    .await
                    .map_err(AcpError::from_rpc)?;
                (id, r.config_options)
            }
        };
        self.costs.open(&session_id, fresh);
        Ok(Opened {
            session_id,
            options: serde_json::to_value(options).unwrap_or(Value::Null),
        })
    }
    pub async fn set_option(
        &self,
        session: &str,
        config_id: &str,
        value: &str,
    ) -> Result<Value, AcpError> {
        let r = self
            .cx
            .send_request(SetSessionConfigOptionRequest::new(
                session.to_owned(),
                config_id.to_owned(),
                SessionConfigOptionValue::value_id(value.to_owned()),
            ))
            .block_task()
            .await
            .map_err(AcpError::from_rpc)?;
        Ok(serde_json::to_value(r.config_options).unwrap_or(Value::Null))
    }
}
