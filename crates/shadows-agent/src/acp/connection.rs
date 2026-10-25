//! Adapter transport initialization and stderr draining.
use super::mapping::Mapping;
use super::{AcpError, Connection, PromptCapabilities};
use crate::events::HarnessEvent;
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, InitializeRequest, PermissionOptionKind, RequestPermissionOutcome,
        RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
        SessionNotification, SessionUpdate,
    },
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use serde_json::Value;
use shadows_process::{ChildErr, ProcessHandle};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

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
        let costs = crate::usage::Costs::default();
        let dispatch_costs = costs.clone();
        tokio::spawn(async move {
            let _ = Client
                .builder()
                .on_receive_notification(
                    async move |n: SessionNotification, _cx| {
                        if let SessionUpdate::UsageUpdate(u) = &n.update {
                            let value = serde_json::to_value(&u.cost).unwrap_or(Value::Null);
                            dispatch_costs.record(
                                &n.session_id.to_string(),
                                serde_json::from_value(value).ok(),
                            );
                        }
                        Mapping::forward(&notify, n.update);
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
        let answer = cx
            .send_request(request)
            .block_task()
            .await
            .map_err(AcpError::from_rpc)?;
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
}

impl AcpError {
    pub(super) fn from_rpc(error: agent_client_protocol::Error) -> AcpError {
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
}

fn forward_stderr(stderr: ChildErr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(target: "harness.stderr", bytes = line.len());
        }
    });
}

/// The client capabilities `initialize` advertises: the Claude adapter's
/// `recommendedValue` extension, under which it offers no effort or model
/// `default` and starts each model's effort at a level it reports.
fn capabilities() -> ClientCapabilities {
    let air = serde_json::json!({
        "jetbrains": { "air": { "version": 1, "capabilities": ["recommendedValue"] } }
    });
    let mut caps = ClientCapabilities::default();
    caps.meta = air.as_object().cloned();
    caps
}
