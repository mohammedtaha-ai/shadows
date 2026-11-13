use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

/// The thread a new opening answers; a fork answers `FORK`.
const THREAD: &str = "fixture-thread";
const FORK: &str = "fixture-fork";

struct Peer {
    output: io::StdoutLock<'static>,
    thread: String,
    turn: usize,
    pending_approval: bool,
    /// The approval policy in force: the thread's, then the last turn's override.
    approval: Value,
    pending_steer: bool,
    mcp: Option<String>,
    pending_tool: Option<tokio::task::JoinHandle<()>>,
    runtime: tokio::runtime::Runtime,
}

async fn mcp_tool(url: &str, tool: &str, arguments: Value) -> io::Result<Value> {
    let token = std::env::var("SHADOWS_CODEX_MCP_TOKEN").map_err(io::Error::other)?;
    let config = StreamableHttpClientTransportConfig::with_uri(url.to_owned()).auth_header(token);
    let client =
        ().serve(StreamableHttpClientTransport::from_config(config))
            .await
            .map_err(io::Error::other)?;
    let params = CallToolRequestParams::new(tool.to_owned())
        .with_arguments(arguments.as_object().cloned().unwrap_or_default());
    let answer = client.call_tool(params).await.map_err(io::Error::other)?;
    let _ = client.cancel().await;
    serde_json::to_value(answer).map_err(io::Error::other)
}

/// One model request's report: `last` is the request, `total` the thread's sum.
fn token_usage(turn: &str, last: u64, cached: u64, total: u64) -> Value {
    json!({
        "turnId": turn,
        "tokenUsage": {
            "last": {"totalTokens": last, "inputTokens": last - 130, "cachedInputTokens": cached,
                     "outputTokens": 130, "reasoningOutputTokens": 0},
            "total": {"totalTokens": total, "inputTokens": total - 130, "cachedInputTokens": cached,
                      "outputTokens": 130, "reasoningOutputTokens": 0},
            "modelContextWindow": 665000
        }
    })
}

fn rate_limits(five_hour_percent: u64) -> Value {
    json!({
        "primary": {"usedPercent": five_hour_percent, "windowDurationMins": 300,
                    "resetsAt": 1790212200},
        "secondary": {"usedPercent": 60, "windowDurationMins": 10080, "resetsAt": 1790542800}
    })
}

fn model(id: &str, efforts: &[&str]) -> Value {
    json!({
        "id": id, "model": id, "displayName": id, "description": "A fixture",
        "isDefault": id == "fixture-model", "defaultReasoningEffort": efforts[0],
        "supportedReasoningEfforts": efforts.iter()
            .map(|effort| json!({"reasoningEffort": effort, "description": effort}))
            .collect::<Vec<_>>()
    })
}

impl Peer {
    fn send(&mut self, message: Value) -> io::Result<()> {
        writeln!(self.output, "{message}")?;
        self.output.flush()
    }

    fn result(&mut self, id: &Value, result: Value) -> io::Result<()> {
        self.send(json!({"id": id, "result": result}))
    }

    fn notify(&mut self, method: &str, mut params: Value) -> io::Result<()> {
        params["threadId"] = json!(self.thread);
        self.send(json!({"method": method, "params": params}))
    }

    fn chunk(&mut self, text: &str) -> io::Result<()> {
        let turn = self.turn.to_string();
        self.notify(
            "item/agentMessage/delta",
            json!({"turnId": turn, "itemId": format!("message-{turn}"), "delta": text}),
        )
    }

    fn terminal(&mut self, turn: &str, status: &str) -> io::Result<()> {
        self.terminal_with(turn, status, Value::Null)
    }

    fn terminal_with(&mut self, turn: &str, status: &str, error: Value) -> io::Result<()> {
        self.notify(
            "turn/completed",
            json!({"turn": {"id": turn, "status": status, "error": error}}),
        )
    }

    fn complete(&mut self, status: &str) -> io::Result<()> {
        self.terminal(&self.turn.to_string(), status)
    }

    fn item(&mut self, method: &str, item: Value) -> io::Result<()> {
        let turn = self.turn.to_string();
        self.notify(method, json!({"turnId": turn, "item": item}))
    }

    fn prompt(&mut self, id: &Value, params: &Value) -> io::Result<bool> {
        let text = params["input"][0]["text"].as_str().unwrap_or("");
        self.turn += 1;
        let turn = self.turn.to_string();
        self.notify(
            "turn/started",
            json!({"turn": {"id": turn, "status": "inProgress", "error": null}}),
        )?;
        if text == "rpc error" {
            self.send(json!({"id": id, "error": {"code": -32602, "message": "fixture error"}}))?;
            return Ok(true);
        }
        self.result(id, json!({"turn": {"id": turn, "status": "inProgress"}}))?;
        if !params["approvalPolicy"].is_null() {
            self.approval = params["approvalPolicy"].clone();
        }
        if params["model"] == "fixture-locked" {
            // The provider's words include a credential-shaped tail that must
            // never reach a person.
            let error = json!({"message": "The 'fixture-locked' model is not available on \
                this account. Bearer fixture-secret-value"});
            self.terminal_with(&turn, "failed", error)?;
            return Ok(true);
        }
        match text {
            "eof" => return Ok(false),
            "interrupt" => self.chunk("waiting")?,
            "settings" => {
                let words = format!(
                    "model={} effort={} sandbox={} approval={} reviewer={}",
                    params["model"],
                    params["effort"],
                    params["sandboxPolicy"]["type"],
                    params["approvalPolicy"],
                    params["approvalsReviewer"]
                );
                self.chunk(&words)?;
                self.complete("completed")?;
            }
            "usage" => {
                self.notify(
                    "thread/tokenUsage/updated",
                    token_usage(&turn, 29130, 24960, 54203),
                )?;
                self.notify(
                    "account/rateLimits/updated",
                    json!({"rateLimits": rate_limits(30)}),
                )?;
                // A repeated report is not a second request.
                self.notify(
                    "thread/tokenUsage/updated",
                    token_usage(&turn, 29130, 24960, 54203),
                )?;
                self.chunk("counted")?;
                self.complete("completed")?;
            }
            "usage small" => {
                self.notify(
                    "thread/tokenUsage/updated",
                    token_usage(&turn, 1000, 0, 55203),
                )?;
                self.chunk("counted small")?;
                self.complete("completed")?;
            }
            "steer" => {
                self.pending_steer = true;
                self.chunk("waiting for steer")?;
            }
            "mcp pending" => {
                self.item(
                    "item/started",
                    json!({"type":"mcpToolCall","id":"tool","tool":"workspace_get",
                    "server":"shadows","status":"inProgress","arguments":{}}),
                )?;
                if let Some(url) = self.mcp.clone() {
                    self.pending_tool = Some(self.runtime.spawn(async move {
                        let _ = mcp_tool(&url, "workspace_get", json!({})).await;
                    }));
                }
            }
            // Codex asks nothing under `never`: the command is refused at once.
            "approval" if self.approval == "never" => {
                self.chunk("refused without asking")?;
                self.complete("completed")?;
            }
            "approval" => {
                self.pending_approval = true;
                let thread = self.thread.clone();
                self.send(json!({
                    "id": "fixture-approval", "method": "item/commandExecution/requestApproval",
                    "params": {"threadId": thread, "turnId": turn, "itemId": "cmd"}
                }))?;
            }
            // The texts the turn's input carried, in order: what the model would read.
            "blocks" => {
                let texts: Vec<&str> = params["input"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|item| item["text"].as_str())
                    .collect();
                self.chunk(&json!(texts).to_string())?;
                self.complete("completed")?;
            }
            "foreign terminal" => {
                self.terminal("stale-turn", "interrupted")?;
                self.chunk("current")?;
                self.complete("completed")?;
            }
            line if line.starts_with("mcp ") => {
                let Some((name, args)) = line[4..].split_once(' ') else {
                    return Err(io::Error::other("fixture needs tool and arguments"));
                };
                let url = self
                    .mcp
                    .as_ref()
                    .ok_or_else(|| io::Error::other("missing MCP setup"))?;
                let args: Value = serde_json::from_str(args).map_err(io::Error::other)?;
                let answer = self.runtime.block_on(mcp_tool(url, name, args.clone()))?;
                self.item(
                    "item/started",
                    json!({"type":"mcpToolCall","id":"tool","tool":name,
                    "server":"shadows","status":"inProgress","arguments":args}),
                )?;
                self.item(
                    "item/completed",
                    json!({"type":"mcpToolCall","id":"tool","tool":name,
                    "server":"shadows","status":"completed","arguments":args,"result":answer}),
                )?;
                self.chunk(answer["content"][0]["text"].as_str().unwrap_or(""))?;
                self.complete("completed")?;
            }
            _ => {
                let frame = json!({"method": "item/agentMessage/delta", "params": {
                    "threadId": self.thread, "turnId": turn,
                    "itemId": format!("message-{turn}"), "delta": "first"
                }})
                .to_string();
                let (first, last) = frame.as_bytes().split_at(frame.len() / 2);
                self.output.write_all(first)?;
                self.output.flush()?;
                std::thread::sleep(Duration::from_millis(15));
                self.output.write_all(last)?;
                self.output.write_all(b"\n")?;
                self.send(json!({"method":"fixture/unrecognized","params":{}}))?;
                self.chunk("last")?;
                self.complete("completed")?;
            }
        }
        Ok(true)
    }

    fn open(&mut self, id: &Value, method: &str, params: &Value) -> io::Result<()> {
        let config = &params["config"]["mcp_servers.shadows"];
        self.mcp = config["url"]
            .as_str()
            .filter(|s| !s.contains(":1/"))
            .map(str::to_owned);
        match method {
            "thread/start" => self.thread = THREAD.into(),
            "thread/fork" if params["threadId"] == THREAD => self.thread = FORK.into(),
            "thread/resume" => {
                self.thread = params["threadId"].as_str().unwrap_or(THREAD).into();
                // History and its usage arrive again on resume; neither is new.
                self.notify(
                    "item/completed",
                    json!({"turnId": "old-turn", "item": {"type": "agentMessage",
                        "id": "old-message", "text": "replayed history"}}),
                )?;
                self.notify(
                    "thread/tokenUsage/updated",
                    token_usage("old-turn", 29130, 24960, 54203),
                )?;
            }
            _ => {
                return self.send(json!({"id": id, "error": {"code": -32602,
                    "message": "unknown source thread"}}));
            }
        }
        let thread = self.thread.clone();
        self.approval = params["approvalPolicy"].clone();
        // What it was asked for, as Codex answers it.
        let sandbox = match params["sandbox"].as_str() {
            Some("workspace-write") => "workspaceWrite",
            _ => "readOnly",
        };
        self.result(
            id,
            json!({
                "thread": {"id": thread}, "sandbox": {"type": sandbox},
                "approvalPolicy": params["approvalPolicy"],
                "approvalsReviewer": params["approvalsReviewer"].as_str().unwrap_or("user"),
                "model": "fixture-model",
                "reasoningEffort": null
            }),
        )
    }

    fn receive(&mut self, m: Value) -> io::Result<bool> {
        let id = &m["id"];
        let params = &m["params"];
        match m["method"].as_str() {
            Some("initialize") => self.result(id, json!({"userAgent": "codex/0.167.0"}))?,
            Some("initialized") => {}
            Some("account/read") => self.result(id, json!({"account": {"type": "chatgpt"}}))?,
            Some("account/rateLimits/read") => {
                self.result(id, json!({"rateLimits": rate_limits(25)}))?
            }
            Some("config/read") => self.result(id, json!({"config": {"mcp_servers": {}}}))?,
            // Two pages, so the catalog's order is read across a cursor.
            Some("model/list") => {
                let page = if params["cursor"].is_null() {
                    json!({"data": [model("fixture-model", &["low", "high"])],
                           "nextCursor": "page-2"})
                } else {
                    json!({"data": [model("fixture-small", &["minimal"]),
                                    model("fixture-locked", &["low"])],
                           "nextCursor": null})
                };
                self.result(id, page)?
            }
            Some(method @ ("thread/start" | "thread/resume" | "thread/fork")) => {
                self.open(id, method, params)?
            }
            Some("mcpServerStatus/list") => {
                let mut tools = json!({"workspace_get": {}});
                if let Some(url) = &self.mcp {
                    let token =
                        std::env::var("SHADOWS_CODEX_MCP_TOKEN").map_err(io::Error::other)?;
                    let listed = self.runtime.block_on(async {
                        let cfg = StreamableHttpClientTransportConfig::with_uri(url.clone())
                            .auth_header(token);
                        let client =
                            ().serve(StreamableHttpClientTransport::from_config(cfg))
                                .await
                                .map_err(io::Error::other)?;
                        let tools = client.list_all_tools().await.map_err(io::Error::other)?;
                        let _ = client.cancel().await;
                        Ok::<_, io::Error>(tools)
                    })?;
                    tools = Value::Object(
                        listed
                            .into_iter()
                            .map(|tool| (tool.name.into_owned(), json!({})))
                            .collect(),
                    );
                }
                if std::env::var_os("CODEX_HOME")
                    .map(PathBuf::from)
                    .is_some_and(|path| path.join("fail-codex-setup").is_file())
                {
                    tools["unexpected_fixture_tool"] = json!({});
                }
                self.result(
                    id,
                    json!({"data": [{
                    "name": "shadows", "tools": tools,
                    "resources": [], "resourceTemplates": []
                }], "nextCursor": null}),
                )?;
            }
            Some("turn/start") => return self.prompt(id, params),
            Some("turn/steer")
                if self.pending_steer
                    && params["expectedTurnId"].as_str() == Some(&self.turn.to_string()) =>
            {
                self.pending_steer = false;
                let turn = self.turn.to_string();
                self.result(id, json!({"turnId": turn}))?;
                let text = params["input"][0]["text"].as_str().unwrap_or("").to_owned();
                self.chunk(&format!("steered: {text}"))?;
                self.complete("completed")?;
            }
            Some("turn/interrupt") => {
                if let Some(pending) = self.pending_tool.take() {
                    pending.abort();
                }
                self.pending_steer = false;
                self.result(id, json!({}))?;
                self.terminal("stale-turn", "interrupted")?;
                std::thread::sleep(Duration::from_millis(200));
                self.complete("interrupted")?;
            }
            None if id == "fixture-approval" && self.pending_approval => {
                self.pending_approval = false;
                if m["result"]["decision"] == "decline" {
                    self.chunk("refused")?;
                    self.complete("completed")?;
                } else {
                    self.complete("failed")?;
                }
            }
            _ => self.send(json!({"id": id, "error": {"code": -32601, "message": "unknown"}}))?,
        }
        Ok(true)
    }
}

fn main() -> io::Result<()> {
    let mut peer = Peer {
        output: io::stdout().lock(),
        thread: THREAD.into(),
        turn: 0,
        pending_approval: false,
        approval: Value::Null,
        pending_steer: false,
        mcp: None,
        pending_tool: None,
        runtime: tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?,
    };
    for line in io::stdin().lock().lines() {
        let message = serde_json::from_str(&line?).map_err(io::Error::other)?;
        if !peer.receive(message)? {
            break;
        }
    }
    Ok(())
}
