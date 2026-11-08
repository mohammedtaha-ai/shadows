use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

struct Peer {
    output: io::StdoutLock<'static>,
    turn: usize,
    pending_approval: bool,
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

impl Peer {
    fn send(&mut self, message: Value) -> io::Result<()> {
        writeln!(self.output, "{message}")?;
        self.output.flush()
    }

    fn result(&mut self, id: &Value, result: Value) -> io::Result<()> {
        self.send(json!({"id": id, "result": result}))
    }

    fn chunk(&mut self, text: &str) -> io::Result<()> {
        self.send(json!({"method": "item/agentMessage/delta", "params": {
            "threadId": "fixture-thread", "turnId": self.turn.to_string(),
            "itemId": "message", "delta": text
        }}))
    }

    fn terminal(&mut self, turn: &str, status: &str) -> io::Result<()> {
        self.send(json!({"method": "turn/completed", "params": {
            "threadId": "fixture-thread",
            "turn": {"id": turn, "status": status, "error": null}
        }}))
    }

    fn complete(&mut self, status: &str) -> io::Result<()> {
        self.terminal(&self.turn.to_string(), status)
    }

    fn item(&mut self, method: &str, item: Value) -> io::Result<()> {
        self.send(json!({"method": method, "params": {
            "threadId": "fixture-thread", "turnId": self.turn.to_string(), "item": item
        }}))
    }

    fn prompt(&mut self, id: &Value, text: &str) -> io::Result<bool> {
        self.turn += 1;
        let turn = self.turn.to_string();
        self.send(json!({"method": "turn/started", "params": {
            "threadId": "fixture-thread",
            "turn": {"id": turn, "status": "inProgress", "error": null}
        }}))?;
        if text == "rpc error" {
            self.send(json!({"id": id, "error": {"code": -32602, "message": "fixture error"}}))?;
            return Ok(true);
        }
        self.result(id, json!({"turn": {"id": turn, "status": "inProgress"}}))?;
        match text {
            "eof" => return Ok(false),
            "interrupt" => self.chunk("waiting")?,
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
            "approval" => {
                self.pending_approval = true;
                self.send(json!({
                    "id": "fixture-approval", "method": "item/commandExecution/requestApproval",
                    "params": {"threadId": "fixture-thread", "turnId": turn, "itemId": "cmd"}
                }))?;
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
                    "threadId": "fixture-thread", "turnId": turn,
                    "itemId": "message", "delta": "first"
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

    fn receive(&mut self, m: Value) -> io::Result<bool> {
        let id = &m["id"];
        let params = &m["params"];
        match m["method"].as_str() {
            Some("initialize") => self.result(id, json!({"userAgent": "codex/0.167.0"}))?,
            Some("initialized") => {}
            Some("account/read") => self.result(id, json!({"account": {"type": "chatgpt"}}))?,
            Some("config/read") => self.result(id, json!({"config": {"mcp_servers": {}}}))?,
            Some("model/list") => self.result(
                id,
                json!({"data":[{
                "id":"fixture-model", "model":"fixture-model", "displayName":"Fixture Model",
                "description":"A fixture", "isDefault":true, "defaultReasoningEffort":"low",
                "supportedReasoningEfforts":[{"reasoningEffort":"low","description":"Fast"}]
            }], "nextCursor":null}),
            )?,
            Some("thread/start" | "thread/resume") => {
                let config = &params["config"]["mcp_servers.shadows"];
                self.mcp = config["url"]
                    .as_str()
                    .filter(|s| !s.contains(":1/"))
                    .map(str::to_owned);
                self.result(id, json!({
                    "thread": {"id": "fixture-thread"}, "sandbox": {"type":"readOnly"},
                    "approvalPolicy": "never", "model": "fixture-model", "reasoningEffort": "low"
                }))?;
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
            Some("turn/start") => {
                return self.prompt(id, params["input"][0]["text"].as_str().unwrap_or(""));
            }
            Some("turn/interrupt") => {
                if let Some(pending) = self.pending_tool.take() {
                    pending.abort();
                }
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
        turn: 0,
        pending_approval: false,
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
