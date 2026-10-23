//! One ACP client connection to one managed adapter process.
use super::events::{TurnUsage, UsageCost};
use super::usage::{SessionCost, cost_delta};
use base64::Engine;
use std::path::Path;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use agent_client_protocol::schema::{
    MaybeUndefined, ProtocolVersion,
    v1::{
        AvailableCommandInput, CancelNotification, ClientCapabilities, ContentBlock,
        ForkSessionRequest, HttpHeader, InitializeRequest, McpServer, McpServerHttp, Meta,
        NewSessionRequest, PermissionOptionKind, PromptRequest, RequestPermissionOutcome,
        RequestPermissionRequest, RequestPermissionResponse, ResumeSessionRequest,
        SelectedPermissionOutcome, SessionConfigOptionValue, SessionNotification, SessionUpdate,
        SetSessionConfigOptionRequest, StopReason, TextContent,
    },
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use serde_json::Value;
use shadows_process::{ChildErr, ProcessHandle};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::events::{HarnessEvent, SlashCommand, tool_meta};

#[derive(Debug, Clone)]
pub enum SessionStart {
    New,
    Resume(String),
    Fork(String),
}

/// What a session opens with (spec §13.8's table): sent alike on
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
    fn mcp_servers(&self) -> Vec<McpServer> {
        self.mcp
            .iter()
            .map(|m| {
                let auth = HttpHeader::new("Authorization", format!("Bearer {}", m.bearer));
                McpServer::Http(McpServerHttp::new(&m.name, &m.url).headers(vec![auth]))
            })
            .collect()
    }

    fn meta(&self) -> Option<Meta> {
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

#[derive(Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Ended,
    Cancelled,
    Refused(String),
}

/// What `_session/steering` answered (spec §20.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steer {
    /// The message went into the running prompt; that prompt answers for it.
    Injected,
    /// No prompt was running; nothing was started.
    PromptRequired,
}

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("ACP connection closed")]
    Closed,
    #[error("ACP request failed: {0}")]
    Rpc(String),
}

/// Backend-neutral image input; protocol encoding belongs to Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlock {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

/// Ordered, backend-neutral prompt input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptBlock {
    Text(String),
    Image(ImageBlock),
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct PromptCapabilities {
    pub image: bool,
    pub image_reason: Option<String>,
}

#[derive(Clone)]
pub struct Connection {
    cx: ConnectionTo<Agent>,
    capabilities: PromptCapabilities,
    costs: Arc<Mutex<HashMap<String, SessionCost>>>,
}

#[derive(Debug, PartialEq)]
pub struct PromptResult {
    pub end: TurnEnd,
    pub usage: Option<TurnUsage>,
}

impl Connection {
    pub fn is_closed(&self) -> bool {
        self.cx.is_incoming_closed()
    }

    /// `events` receives what the adapter reports, called inside the
    /// connection's dispatch loop in the order the adapter sent it: an update
    /// sent before the answer to a request has been handed over before that
    /// answer is. A task between the two would break that, and a turn could
    /// end before its last update arrived.
    pub async fn open(
        handle: &mut ProcessHandle,
        events: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
    ) -> Result<Self, AcpError> {
        let (stdin, stdout, stderr) = handle.take_stdio().ok_or(AcpError::Closed)?;
        forward_stderr(stderr);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let notify = events.clone();
        let costs: Arc<Mutex<HashMap<String, SessionCost>>> = Arc::default();
        let dispatch_costs = costs.clone();
        tokio::spawn(async move {
            let _ = Client
                .builder()
                .on_receive_notification(
                    async move |n: SessionNotification, _cx| {
                        if let SessionUpdate::UsageUpdate(u) = &n.update {
                            let value = serde_json::to_value(&u.cost).unwrap_or(Value::Null);
                            let mut costs = dispatch_costs.lock().expect("cost ledger");
                            if let Some(state) = costs.get_mut(&n.session_id.to_string())
                                && state.active
                            {
                                state.current = serde_json::from_value(value).ok();
                            }
                        }
                        forward(&notify, n.update);
                        Ok(())
                    },
                    agent_client_protocol::on_receive_notification!(),
                )
                .on_receive_request(
                    async move |r: RequestPermissionRequest, responder, _cx| {
                        events(HarnessEvent::PermissionRefused {
                            title: r.tool_call.fields.title.clone().unwrap_or_default(),
                        });
                        let choice = r
                            .options
                            .iter()
                            .find(|o| matches!(o.kind, PermissionOptionKind::RejectOnce))
                            .or_else(|| {
                                r.options
                                    .iter()
                                    .find(|o| matches!(o.kind, PermissionOptionKind::RejectAlways))
                            });
                        let outcome = match choice {
                            Some(o) => RequestPermissionOutcome::Selected(
                                SelectedPermissionOutcome::new(o.option_id.clone()),
                            ),
                            None => RequestPermissionOutcome::Cancelled,
                        };
                        responder.respond(RequestPermissionResponse::new(outcome))
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .connect_with(
                    ByteStreams::new(stdin.compat_write(), stdout.compat()),
                    |cx: ConnectionTo<Agent>| async move {
                        let _ = ready_tx.send(cx);
                        std::future::pending::<()>().await;
                        #[allow(unreachable_code)]
                        Ok(())
                    },
                )
                .await;
        });
        let cx = ready_rx.await.map_err(|_| AcpError::Closed)?;
        let request =
            InitializeRequest::new(ProtocolVersion::V1).client_capabilities(capabilities());
        let answer = cx.send_request(request).block_task().await.map_err(rpc)?;
        let value = serde_json::to_value(answer).map_err(|e| AcpError::Rpc(e.to_string()))?;
        let image = value
            .pointer("/agentCapabilities/promptCapabilities/image")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let capabilities = PromptCapabilities {
            image,
            image_reason: (!image).then(|| "The harness does not support image prompts".into()),
        };
        Ok(Self {
            cx,
            costs,
            capabilities,
        })
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
                    .map_err(rpc)?;
                (r.session_id.to_string(), r.config_options)
            }
            SessionStart::Resume(id) => {
                let r = self
                    .cx
                    .send_request(resume(id.clone()))
                    .block_task()
                    .await
                    .map_err(rpc)?;
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
                    .map_err(rpc)?;
                let id = fork.session_id.to_string();
                let r = self
                    .cx
                    .send_request(resume(id.clone()))
                    .block_task()
                    .await
                    .map_err(rpc)?;
                (id, r.config_options)
            }
        };
        self.costs.lock().expect("cost ledger").insert(
            session_id.clone(),
            SessionCost {
                previous: fresh.then(|| UsageCost {
                    amount: 0.0,
                    currency: "USD".into(),
                }),
                ..Default::default()
            },
        );
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
            .map_err(rpc)?;
        Ok(serde_json::to_value(r.config_options).unwrap_or(Value::Null))
    }

    /// The person's text is the first content block; each `context` entry
    /// is one more text block after it (§13.8).
    pub async fn prompt(
        &self,
        session: &str,
        text: &str,
        context: &[String],
    ) -> Result<PromptResult, AcpError> {
        let blocks = std::iter::once(text)
            .chain(context.iter().map(String::as_str))
            .map(|text| PromptBlock::Text(text.into()))
            .collect::<Vec<_>>();
        self.prompt_blocks(session, &blocks).await
    }

    pub fn prompt_capabilities(&self) -> PromptCapabilities {
        self.capabilities.clone()
    }

    pub async fn prompt_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<PromptResult, AcpError> {
        self.require_images(blocks)?;
        {
            let mut costs = self.costs.lock().expect("cost ledger");
            let state = costs.entry(session.into()).or_default();
            if state.active {
                return Err(AcpError::Rpc("session prompt already active".into()));
            }
            state.current = None;
            state.active = true;
        }
        let blocks = Self::acp_blocks(blocks);
        let response = self
            .cx
            .send_request(PromptRequest::new(session.to_owned(), blocks))
            .block_task()
            .await;
        let delta = {
            let mut costs = self.costs.lock().expect("cost ledger");
            let state = costs.entry(session.into()).or_default();
            let delta = cost_delta(state.previous.as_ref(), state.current.as_ref());
            state.previous = state.current.take();
            state.active = false;
            delta
        };
        let r = response.map_err(rpc)?;
        let usage = r
            .usage
            .and_then(|u| serde_json::to_value(u).ok())
            .and_then(|v| serde_json::from_value::<TurnUsage>(v).ok())
            .map(|mut u| {
                u.cost = delta;
                u
            });
        let end = match r.stop_reason {
            StopReason::EndTurn => TurnEnd::Ended,
            StopReason::Cancelled => TurnEnd::Cancelled,
            StopReason::MaxTokens => TurnEnd::Refused("max_tokens".into()),
            StopReason::MaxTurnRequests => TurnEnd::Refused("max_turn_requests".into()),
            StopReason::Refusal => TurnEnd::Refused("refusal".into()),
            other => TurnEnd::Refused(format!("{other:?}")),
        };
        Ok(PromptResult { end, usage })
    }

    /// Sends `text` into the session's running prompt (§20.4), asking the
    /// adapter to start nothing when none runs.
    pub async fn steer(&self, session: &str, text: &str) -> Result<Steer, AcpError> {
        self.steer_blocks(session, &[PromptBlock::Text(text.into())])
            .await
    }

    fn require_images(&self, blocks: &[PromptBlock]) -> Result<(), AcpError> {
        if blocks.iter().any(|b| matches!(b, PromptBlock::Image(_))) && !self.capabilities.image {
            return Err(AcpError::Rpc(
                self.capabilities.image_reason.clone().unwrap_or_default(),
            ));
        }
        Ok(())
    }

    fn acp_blocks(blocks: &[PromptBlock]) -> Vec<ContentBlock> {
        blocks
            .iter()
            .map(|block| match block {
                PromptBlock::Text(text) => ContentBlock::Text(TextContent::new(text)),
                PromptBlock::Image(image) => {
                    let data = base64::engine::general_purpose::STANDARD.encode(&image.bytes);
                    ContentBlock::Image(agent_client_protocol::schema::v1::ImageContent::new(
                        data,
                        &image.media_type,
                    ))
                }
            })
            .collect()
    }

    pub async fn steer_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<Steer, AcpError> {
        self.require_images(blocks)?;
        let params = serde_json::json!({
            "sessionId": session,
            "prompt": Self::acp_blocks(blocks),
            "_meta": { "steering": { "idleBehavior": "promptRequired" } },
        });
        let request =
            agent_client_protocol::UntypedMessage::new("_session/steering", params).map_err(rpc)?;
        let answer = self
            .cx
            .send_request(request)
            .block_task()
            .await
            .map_err(rpc)?;
        match answer.get("outcome").and_then(Value::as_str) {
            Some("injected") => Ok(Steer::Injected),
            Some("promptRequired") => Ok(Steer::PromptRequired),
            other => Err(AcpError::Rpc(format!(
                "unexpected steering answer {other:?}"
            ))),
        }
    }

    pub fn cancel(&self, session: &str) {
        let cx = self.cx.clone();
        let session = session.to_string();
        let _ = cx.send_notification(CancelNotification::new(session));
    }
}

fn rpc(error: agent_client_protocol::Error) -> AcpError {
    if agent_client_protocol::is_incoming_transport_closed(&error) {
        return AcpError::Closed;
    }
    let value = serde_json::to_value(&error).unwrap_or(Value::Null);
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if let Some(details) = value.pointer("/data/details").and_then(Value::as_str) {
        AcpError::Rpc(details.to_string())
    } else if !message.is_empty() {
        AcpError::Rpc(message.to_string())
    } else {
        AcpError::Closed
    }
}

fn forward(events: &impl Fn(HarnessEvent), update: SessionUpdate) {
    let item = match update {
        SessionUpdate::AgentMessageChunk(c) => match c.content {
            ContentBlock::Text(t) => Some(HarnessEvent::Chunk {
                message_id: c.message_id.map(|id| id.to_string()),
                text: t.text,
            }),
            _ => None,
        },
        SessionUpdate::ToolCall(t) => {
            let m = tool_meta(t.meta.as_ref(), t.raw_input.as_ref());
            Some(HarnessEvent::ToolCall {
                id: t.tool_call_id.to_string(),
                title: Some(t.title),
                status: Some(format!("{:?}", t.status).to_lowercase()),
                tool: m.tool.or(t.name),
                parent: m.parent,
                agent: m.agent,
            })
        }
        SessionUpdate::ToolCallUpdate(t) => {
            let m = tool_meta(t.meta.as_ref(), t.fields.raw_input.as_ref());
            Some(HarnessEvent::ToolCall {
                id: t.tool_call_id.to_string(),
                title: t.fields.title,
                status: t.fields.status.map(|s| format!("{s:?}").to_lowercase()),
                tool: m.tool.or(t.fields.name),
                parent: m.parent,
                agent: m.agent,
            })
        }
        SessionUpdate::UsageUpdate(u) => {
            let meta = serde_json::to_value(u.meta).unwrap_or(Value::Null);
            Some(HarnessEvent::Usage {
                cost: serde_json::to_value(u.cost)
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok()),
                used: u.used,
                size: u.size,
                model: meta
                    .get("_claude/model")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                rate_limit: meta.get("_claude/rateLimit").cloned(),
            })
        }
        SessionUpdate::ConfigOptionUpdate(o) => Some(HarnessEvent::Options(
            serde_json::to_value(o.config_options).unwrap_or(Value::Null),
        )),
        // A title, or only `_meta` (a goal, a file-change report): the rest is
        // not Shadows'. A `null` title clears the harness's own; Shadows keeps
        // its thread's.
        SessionUpdate::SessionInfoUpdate(i) => match i.title {
            MaybeUndefined::Value(title) => Some(HarnessEvent::SessionTitle { title }),
            _ => None,
        },
        SessionUpdate::AvailableCommandsUpdate(u) => Some(HarnessEvent::Commands(
            u.available_commands
                .into_iter()
                .map(|c| SlashCommand {
                    hint: match c.input {
                        Some(AvailableCommandInput::Unstructured(i)) => Some(i.hint),
                        _ => None,
                    },
                    name: c.name,
                    description: c.description,
                })
                .collect(),
        )),
        other => {
            tracing::trace!(?other, "ignored ACP update");
            None
        }
    };
    if let Some(item) = item {
        events(item);
    }
}

fn forward_stderr(stderr: ChildErr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(target: "harness.stderr", %line);
        }
    });
}

/// The client capabilities `initialize` advertises: the Claude adapter's
/// `recommendedValue` extension, under which it offers no effort or model
/// `default` and starts each model's effort at a level it reports (spec §12.4,
/// `docs/evidence/harness/EFFORT_DEFAULT_PROBE.md` at `92e6dae` §2).
fn capabilities() -> ClientCapabilities {
    let air = serde_json::json!({
        "jetbrains": { "air": { "version": 1, "capabilities": ["recommendedValue"] } }
    });
    let mut caps = ClientCapabilities::default();
    caps.meta = air.as_object().cloned();
    caps
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod tests;
