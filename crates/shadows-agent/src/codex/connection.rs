//! Ordered JSONL dispatch for one managed child.
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use shadows_process::ProcessHandle;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use crate::connection::ConnectionError;

type Reply = oneshot::Sender<Result<Value, ConnectionError>>;

#[derive(Default)]
struct Pending {
    calls: HashMap<u64, Reply>,
    closed: Option<ConnectionError>,
}

struct Registration<'a> {
    pending: &'a Mutex<Pending>,
    id: u64,
}

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        self.pending.lock().unwrap().calls.remove(&self.id);
    }
}

#[derive(Clone)]
pub(super) struct Rpc {
    outgoing: mpsc::UnboundedSender<Value>,
    pending: Arc<Mutex<Pending>>,
    next_id: Arc<AtomicU64>,
}

impl Rpc {
    pub(super) fn open(
        handle: &mut ProcessHandle,
        notify: impl Fn(&str, Value) + Send + Sync + 'static,
        close: impl Fn(ConnectionError) + Send + Sync + 'static,
    ) -> Result<Self, ConnectionError> {
        let (mut stdin, stdout, mut stderr) = handle.take_stdio().ok_or(ConnectionError::Closed)?;
        let (outgoing, mut writes) = mpsc::unbounded_channel::<Value>();
        let pending = Arc::new(Mutex::new(Pending::default()));
        let close = Arc::new(close);
        let write_pending = pending.clone();
        let write_close = close.clone();
        tokio::spawn(async move {
            while let Some(frame) = writes.recv().await {
                let mut bytes = frame.to_string().into_bytes();
                bytes.push(b'\n');
                if stdin.write_all(&bytes).await.is_err() {
                    fail(&write_pending, ConnectionError::Closed, &*write_close);
                    break;
                }
            }
        });
        let read_pending = pending.clone();
        let responses = outgoing.downgrade();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            let error = loop {
                let line = match lines.next_line().await {
                    Ok(Some(line)) => line,
                    _ => break ConnectionError::Closed,
                };
                let Ok(frame) = serde_json::from_str::<Value>(&line) else {
                    break ConnectionError::Protocol("malformed JSONL frame");
                };
                let Some(frame) = frame.as_object() else {
                    break ConnectionError::Protocol("invalid JSONL envelope");
                };
                if let Some(method) = frame.get("method").and_then(Value::as_str) {
                    if let Some(id) = frame.get("id") {
                        let Some(writer) = responses.upgrade() else {
                            break ConnectionError::Closed;
                        };
                        let (reply, refused) = match method {
                            "item/commandExecution/requestApproval"
                            | "item/fileChange/requestApproval" => {
                                (json!({"id":id,"result":{"decision":"decline"}}), true)
                            }
                            "item/permissions/requestApproval" => (
                                json!({"id":id,"result":{"permissions":{},"scope":"turn"}}),
                                true,
                            ),
                            _ => (
                                json!({"id":id,"error":{"code":-32601,
                                "message":"Method not supported by Shadows"}}),
                                false,
                            ),
                        };
                        if refused {
                            notify("shadows/permissionRefused", Value::Null);
                        }
                        if writer.send(reply).is_err() {
                            break ConnectionError::Closed;
                        }
                    } else {
                        notify(method, frame.get("params").cloned().unwrap_or(Value::Null));
                    }
                    continue;
                }
                let Some(id) = frame.get("id").and_then(Value::as_u64) else {
                    break ConnectionError::Protocol("missing response ID");
                };
                let reply = read_pending.lock().unwrap().calls.remove(&id);
                if let Some(reply) = reply {
                    let answer = match (frame.get("result"), frame.get("error")) {
                        (Some(result), None) => Ok(result.clone()),
                        (None, Some(error)) => {
                            let code = error["code"].as_i64().unwrap_or(-32603);
                            Err(ConnectionError::Rpc(format!("Codex RPC error {code}")))
                        }
                        _ => Err(ConnectionError::Protocol("invalid JSON-RPC response")),
                    };
                    let _ = reply.send(answer);
                }
            };
            fail(&read_pending, error, &*close);
        });
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
        });
        Ok(Self {
            outgoing,
            pending,
            next_id: Arc::default(),
        })
    }

    pub(super) fn is_closed(&self) -> bool {
        self.pending.lock().unwrap().closed.is_some()
    }

    pub(super) fn notify(&self, method: &'static str) -> Result<(), ConnectionError> {
        self.outgoing
            .send(json!({"method":method}))
            .map_err(|_| ConnectionError::Closed)
    }

    pub(super) async fn request<T: DeserializeOwned>(
        &self,
        method: &'static str,
        params: Value,
    ) -> Result<T, ConnectionError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if let Some(error) = &pending.closed {
                return Err(error.clone());
            }
            pending.calls.insert(id, send);
        }
        let _registration = Registration {
            pending: &self.pending,
            id,
        };
        self.outgoing
            .send(json!({"id":id,"method":method,"params":params}))
            .map_err(|_| ConnectionError::Closed)?;
        let answer = receive.await.map_err(|_| ConnectionError::Closed)??;
        serde_json::from_value(answer).map_err(|_| ConnectionError::Protocol("invalid RPC result"))
    }
}

fn fail(pending: &Mutex<Pending>, error: ConnectionError, close: &impl Fn(ConnectionError)) {
    let calls = {
        let mut pending = pending.lock().unwrap();
        if pending.closed.is_some() {
            return;
        }
        pending.closed = Some(error.clone());
        std::mem::take(&mut pending.calls)
    };
    close(error.clone());
    for (_, reply) in calls {
        let _ = reply.send(Err(error.clone()));
    }
}
