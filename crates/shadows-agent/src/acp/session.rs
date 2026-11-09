//! Session opening configuration and option requests.
use super::{AcpError, Connection, McpServerSpec, Opened, SessionSetup, SessionStart};
use crate::choices::Offered;
use agent_client_protocol::v1::schema::{
    EnvVariable, ForkSessionRequest, HttpHeader, McpServer, McpServerHttp, McpServerStdio, Meta,
    NewSessionRequest, ResumeSessionRequest, SessionConfigOptionValue,
    SetSessionConfigOptionRequest,
};
use serde_json::Value;
use std::path::Path;

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
        if !self.permissions.is_empty() {
            options.insert(
                "settings".into(),
                serde_json::json!({ "permissions": { "allow": self.permissions } }),
            );
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

impl Connection {
    /// Asks the adapter to stop one background task; the turn and the other
    /// tasks go on. Answers the adapter's `stopped`.
    pub async fn stop_task(&self, session: &str, task: &str) -> Result<bool, AcpError> {
        let request = agent_client_protocol::UntypedMessage::new(
            "_session/async_task/stop",
            serde_json::json!({"sessionId": session, "asyncTaskId": task}),
        )
        .map_err(AcpError::from_rpc)?;
        let answer = self
            .cx
            .send_request(request)
            .block_task()
            .await
            .map_err(AcpError::from_rpc)?;
        answer
            .get("stopped")
            .and_then(Value::as_bool)
            .ok_or_else(|| AcpError::Rpc("invalid task stop response".into()))
    }
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
            options: Offered::parse(&serde_json::to_value(options).unwrap_or(Value::Null))
                .map_err(AcpError::Rpc)?,
        })
    }
    pub async fn set_option(
        &self,
        session: &str,
        config_id: &str,
        value: &str,
    ) -> Result<Offered, AcpError> {
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
        Offered::parse(&serde_json::to_value(r.config_options).unwrap_or(Value::Null))
            .map_err(AcpError::Rpc)
    }
}
