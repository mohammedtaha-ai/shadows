//! Session opening configuration and option requests.
use super::{AcpError, Connection};
use agent_client_protocol::schema::v1::{
    ForkSessionRequest, HttpHeader, McpServer, McpServerHttp, Meta, NewSessionRequest,
    ResumeSessionRequest, SessionConfigOptionValue, SetSessionConfigOptionRequest,
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
    /// Shadows' own MCP server.
    pub mcp: Option<McpServerSpec>,
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

/// One HTTP MCP server, reached with `Authorization: Bearer <bearer>`.
#[derive(Clone)]
pub struct McpServerSpec {
    pub name: String,
    pub url: String,
    pub bearer: String,
}

/// The bearer is a live grant's token: never in a log line.
impl std::fmt::Debug for McpServerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServerSpec")
            .field("name", &self.name)
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

impl SessionSetup {
    pub(super) fn mcp_servers(&self) -> Vec<McpServer> {
        self.mcp
            .iter()
            .map(|m| {
                let auth = HttpHeader::new("Authorization", format!("Bearer {}", m.bearer));
                McpServer::Http(McpServerHttp::new(&m.name, &m.url).headers(vec![auth]))
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
