//! One ACP client connection to one managed adapter process.
use std::path::Path;

use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        CancelNotification, ContentBlock, ForkSessionRequest, InitializeRequest, NewSessionRequest,
        PermissionOptionKind, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
        RequestPermissionResponse, ResumeSessionRequest, SelectedPermissionOutcome,
        SessionConfigOptionValue, SessionNotification, SessionUpdate,
        SetSessionConfigOptionRequest, StopReason, TextContent,
    },
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::events::HarnessEvent;
use crate::process::{ChildErr, ProcessHandle};

#[derive(Debug, Clone)]
pub enum SessionStart {
    New,
    Resume(String),
    Fork(String),
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

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("ACP connection closed")]
    Closed,
    #[error("ACP request failed: {0}")]
    Rpc(String),
}

#[derive(Clone)]
pub struct Connection {
    cx: ConnectionTo<Agent>,
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
        tokio::spawn(async move {
            let _ = Client
                .builder()
                .on_receive_notification(
                    async move |n: SessionNotification, _cx| {
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
        cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
            .block_task()
            .await
            .map_err(rpc)?;
        Ok(Self { cx })
    }

    pub async fn start_session(&self, cwd: &Path, how: SessionStart) -> Result<Opened, AcpError> {
        let (session_id, options) = match how {
            SessionStart::New => {
                let r = self
                    .cx
                    .send_request(NewSessionRequest::new(cwd))
                    .block_task()
                    .await
                    .map_err(rpc)?;
                (r.session_id.to_string(), r.config_options)
            }
            SessionStart::Resume(id) => {
                let r = self
                    .cx
                    .send_request(ResumeSessionRequest::new(id.clone(), cwd))
                    .block_task()
                    .await
                    .map_err(rpc)?;
                (id, r.config_options)
            }
            SessionStart::Fork(src) => {
                let fork = self
                    .cx
                    .send_request(ForkSessionRequest::new(src, cwd))
                    .block_task()
                    .await
                    .map_err(rpc)?;
                let id = fork.session_id.to_string();
                let r = self
                    .cx
                    .send_request(ResumeSessionRequest::new(id.clone(), cwd))
                    .block_task()
                    .await
                    .map_err(rpc)?;
                (id, r.config_options)
            }
        };
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

    pub async fn prompt(&self, session: &str, text: &str) -> Result<TurnEnd, AcpError> {
        let r = self
            .cx
            .send_request(PromptRequest::new(
                session.to_owned(),
                vec![ContentBlock::Text(TextContent::new(text))],
            ))
            .block_task()
            .await
            .map_err(rpc)?;
        Ok(match r.stop_reason {
            StopReason::EndTurn => TurnEnd::Ended,
            StopReason::Cancelled => TurnEnd::Cancelled,
            StopReason::MaxTokens => TurnEnd::Refused("max_tokens".into()),
            StopReason::MaxTurnRequests => TurnEnd::Refused("max_turn_requests".into()),
            StopReason::Refusal => TurnEnd::Refused("refusal".into()),
            other => TurnEnd::Refused(format!("{other:?}")),
        })
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
        SessionUpdate::ToolCall(t) => Some(HarnessEvent::ToolCall {
            id: t.tool_call_id.to_string(),
            title: Some(t.title),
            status: Some(format!("{:?}", t.status).to_lowercase()),
        }),
        SessionUpdate::ToolCallUpdate(t) => Some(HarnessEvent::ToolCall {
            id: t.tool_call_id.to_string(),
            title: t.fields.title,
            status: t.fields.status.map(|s| format!("{s:?}").to_lowercase()),
        }),
        SessionUpdate::UsageUpdate(u) => {
            let meta = serde_json::to_value(u.meta).unwrap_or(Value::Null);
            Some(HarnessEvent::Usage {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_error_mentioning_connection_closed_is_still_an_rpc_error() {
        let error = agent_client_protocol::Error::new(-32603, "connection closed by policy");
        assert!(
            matches!(rpc(error), AcpError::Rpc(message) if message == "connection closed by policy")
        );
    }
}
