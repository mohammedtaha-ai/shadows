//! ACP test agent. Prompts: ordinary (two chunks), `two-messages` (tool updates),
//! `report` (session state), `/context` (delayed first report), `hang` (cancel),
//! `ignore-cancel` (never), `exit` (code 3), `ask-permission` (reject),
//! `usage` (two context updates), `refuse` (max_tokens), `title <text>` (names
//! the session `<text>` in a `session_info_update` after answering), and
//! `mcp <tool> <json>` (calls the session's MCP server, §13.8). `report` also
//! shows what the session opened with, the client capabilities' `_meta` that
//! `initialize` carried, and the prompt's blocks. Models:
//! `fake-large` (efforts, `auto`), `fake-small` (efforts, no `auto`),
//! `fake-tiny` (no effort), `fake-locked` (refused, as an account without
//! credits is). A session starts at `fake-large`, `high`, `auto`: as the real
//! adapter does, at the person's own defaults rather than Shadows'.
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, ForkSessionRequest,
    ForkSessionResponse, InitializeRequest, InitializeResponse, McpServer, Meta, NewSessionRequest,
    NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionRequest, ResumeSessionRequest, ResumeSessionResponse, SessionConfigOption,
    SessionInfoUpdate, SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, StopReason, TextContent, ToolCall, ToolCallUpdate, UsageUpdate,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Responder, Stdio};
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use tokio::sync::watch;

#[derive(Clone)]
struct Session {
    cwd: PathBuf,
    how: &'static str,
    model: String,
    effort: Option<String>,
    mode: String,
    cancel: watch::Sender<bool>,
    setup: Setup,
}

/// What a session opened with: Shadows' MCP server, the appended prompt and
/// the pre-approved tools. The bearer is kept to call `/mcp` and reported only
/// as its hash, because a reply is stored as an entry.
#[derive(Clone, Default)]
struct Setup {
    mcp: Value,
    url: Option<String>,
    bearer: Option<String>,
    append: Option<String>,
    allowed: Value,
}

fn setup_of(servers: &[McpServer], meta: Option<&Meta>) -> Setup {
    let http = servers.iter().find_map(|s| match s {
        McpServer::Http(h) => Some(h),
        _ => None,
    });
    let auth = http.and_then(|h| h.headers.iter().find(|x| x.name == "Authorization"));
    let meta = meta
        .map(|m| Value::Object(m.clone()))
        .unwrap_or(Value::Null);
    Setup {
        mcp: http
            .map(|h| json!({"name":h.name,"url":h.url}))
            .unwrap_or(Value::Null),
        url: http.map(|h| h.url.clone()),
        bearer: auth
            .and_then(|x| x.value.strip_prefix("Bearer "))
            .map(str::to_owned),
        append: meta
            .pointer("/systemPrompt/append")
            .and_then(Value::as_str)
            .map(str::to_owned),
        allowed: meta
            .pointer("/claudeCode/options/allowedTools")
            .cloned()
            .unwrap_or(Value::Null),
    }
}

/// `mcp <tool> <json args>`: one call on the session's MCP server with its
/// bearer, answering the result's text.
async fn call_mcp(setup: &Setup, line: &str) -> String {
    let mut words = line.splitn(3, ' ').skip(1);
    let tool = words.next().unwrap_or_default().to_string();
    let args: Value = serde_json::from_str(words.next().unwrap_or("{}")).unwrap_or(json!({}));
    let (Some(url), Some(bearer)) = (&setup.url, &setup.bearer) else {
        return "mcp: no server".into();
    };
    let config =
        StreamableHttpClientTransportConfig::with_uri(url.clone()).auth_header(bearer.clone());
    let client = match ().serve(StreamableHttpClientTransport::from_config(config)).await {
        Ok(client) => client,
        Err(e) => return format!("mcp: {e}"),
    };
    let params = CallToolRequestParams::new(tool)
        .with_arguments(args.as_object().cloned().unwrap_or_default());
    let text = match client.call_tool(params).await {
        Ok(r) => r
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .unwrap_or_default(),
        Err(e) => format!("mcp: {e}"),
    };
    let _ = client.cancel().await;
    text
}

#[derive(Default)]
struct State {
    next: usize,
    sessions: HashMap<String, Session>,
    context_seen: bool,
    /// `session/resume` requests this process received.
    resumes: usize,
    /// The client capabilities' `_meta` `initialize` carried.
    client_meta: Value,
}
type Shared = Arc<Mutex<State>>;

/// The efforts a fake model offers; `fake-tiny` offers none, as Haiku 4.5
/// does (ACP_PROBE §1).
fn efforts(model: &str) -> &'static [&'static str] {
    match model {
        "fake-large" => &["low", "high", "max"],
        "fake-small" => &["low", "high"],
        _ => &[],
    }
}

/// The options as the real adapter shapes them (ACP_PROBE §1): ids that are
/// not their categories (`effort` is `thought_level`), the effort option
/// absent for a model without one, and a `model_config` option to ignore.
fn options(s: &Session) -> Vec<SessionConfigOption> {
    let efforts = efforts(&s.model);
    let mut values = vec![
        json!({"id":"mode","name":"Mode","category":"mode","type":"select","currentValue":s.mode,
            "options":(["default","acceptEdits","plan","auto","bypassPermissions"].iter()
              .map(|v| json!({"value":v,"name":v})).collect::<Vec<_>>()) }),
        json!({"id":"model","name":"Model","category":"model","type":"select","currentValue":s.model,
            "options":[
                {"value":"fake-large","name":"Fake Large","description":"The biggest fake"},
                {"value":"fake-small","name":"Fake Small"},
                {"value":"fake-tiny","name":"Fake Tiny"},
                {"value":"fake-locked","name":"Fake Locked"}]}),
    ];
    if let Some(effort) = &s.effort {
        values.push(
            json!({"id":"effort","name":"Effort","category":"thought_level","type":"select","currentValue":effort,
            "options":efforts.iter().map(|v| json!({"value":v,"name":v})).collect::<Vec<_>>() }),
        );
    }
    values.push(json!({"id":"fast","name":"Fast","category":"model_config","type":"select","currentValue":"off",
        "options":[{"value":"on","name":"On"},{"value":"off","name":"Off"}]}));
    values
        .into_iter()
        .map(|v| serde_json::from_value(v).expect("fake option schema"))
        .collect()
}

fn make_session(cwd: PathBuf, how: &'static str, setup: Setup) -> Session {
    let (cancel, _) = watch::channel(false);
    Session {
        cwd,
        how,
        model: "fake-large".into(),
        effort: Some("high".into()),
        mode: "auto".into(),
        cancel,
        setup,
    }
}

fn update(
    cx: &ConnectionTo<Client>,
    id: &str,
    item: SessionUpdate,
) -> agent_client_protocol::Result<()> {
    cx.send_notification(SessionNotification::new(id.to_owned(), item))
}

fn chunk(
    cx: &ConnectionTo<Client>,
    id: &str,
    mid: &str,
    text: &str,
) -> agent_client_protocol::Result<()> {
    update(
        cx,
        id,
        SessionUpdate::AgentMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new(text))).message_id(mid),
        ),
    )
}

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("fake-claude-1");
        return Ok(());
    }
    let state: Shared = Arc::default();
    Agent.builder().name("fake-acp")
        .on_receive_request({ let state = state.clone(); async move |r: InitializeRequest, responder, _cx| {
            state.lock().unwrap().client_meta = r.client_capabilities.meta.clone().map(Value::Object).unwrap_or(Value::Null);
            let caps: AgentCapabilities = serde_json::from_value(json!({"sessionCapabilities":{"fork":{},"resume":{}}})).unwrap();
            responder.respond(InitializeResponse::new(r.protocol_version).agent_capabilities(caps))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: NewSessionRequest, responder, _cx| {
            let mut st = state.lock().unwrap();
            st.next += 1;
            // Unique across adapter processes, as the real harness's ids are.
            let id = format!("fake-{}-{}", std::process::id(), st.next);
            let s = make_session(r.cwd, "new", setup_of(&r.mcp_servers, r.meta.as_ref()));
            let opts = options(&s);
            st.sessions.insert(id.clone(), s);
            responder.respond(NewSessionResponse::new(id).config_options(opts))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: ResumeSessionRequest, responder, _cx| {
            let id = r.session_id.to_string();
            // A harness slow to open its session, as Claude Code's own
            // startup is on Windows (seconds).
            if id.starts_with("slow-") {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
            let mut st = state.lock().unwrap();
            let how = if id.starts_with("fork-of-") { "fork" } else { "resume" };
            st.resumes += 1;
            let s = make_session(r.cwd, how, setup_of(&r.mcp_servers, r.meta.as_ref()));
            let opts = options(&s);
            st.sessions.insert(id, s);
            responder.respond(ResumeSessionResponse::new().config_options(opts))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: ForkSessionRequest, responder, _cx| {
            let st = state.lock().unwrap();
            // The real adapter forks any session in Claude's own store; the
            // fake's store is its process, so a fake id from another adapter
            // process stands for one that exists.
            let id = r.session_id.to_string();
            if !st.sessions.contains_key(&id) && !id.starts_with("fake-") && !id.starts_with("fork-of-") {
                return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "Session not found"));
            }
            responder.respond(ForkSessionResponse::new(format!("fork-of-{}", r.session_id)))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: SetSessionConfigOptionRequest, responder, _cx| {
            let mut st = state.lock().unwrap();
            let Some(s) = st.sessions.get_mut(&r.session_id.to_string()) else {
                return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "Session not found"));
            };
            let value = r.value.as_value_id().map(ToString::to_string).unwrap_or_default();
            match r.config_id.to_string().as_str() {
                "model" if value == "fake-locked" => return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "Usage credits are required for this model · model not changed")),
                "model" => {
                    s.effort = (!efforts(&value).is_empty()).then(|| "high".to_string());
                    s.model = value;
                    if s.mode == "auto" && s.model != "fake-large" { s.mode = "acceptEdits".into(); }
                },
                "effort" if !efforts(&s.model).contains(&value.as_str()) => return responder.respond_with_error(agent_client_protocol::Error::new(-32602, "effort not offered")),
                "effort" => s.effort = Some(value),
                "mode" if value == "auto" && s.model != "fake-large" => return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "auto mode is not available for this model")),
                "mode" => s.mode = value,
                _ => {},
            }
            responder.respond(SetSessionConfigOptionResponse::new(options(s)))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_notification({ let state = state.clone(); async move |r: CancelNotification, _cx| {
            if let Some(s) = state.lock().unwrap().sessions.get(&r.session_id.to_string()) { let _ = s.cancel.send(true); }
            Ok(())
        }}, agent_client_protocol::on_receive_notification!())
        .on_receive_request({ let state = state.clone(); async move |r: PromptRequest, responder: Responder<PromptResponse>, cx: ConnectionTo<Client>| {
            let worker = cx.clone();
            let state = state.clone();
            cx.spawn(async move {
            let cx = worker;
            let id = r.session_id.to_string();
            let prompt = r.prompt.iter().find_map(|b| match b { ContentBlock::Text(t) => Some(t.text.as_str()), _ => None }).unwrap_or("");
            let blocks: Vec<&str> = r.prompt.iter().filter_map(|b| match b { ContentBlock::Text(t) => Some(t.text.as_str()), _ => None }).collect();
            let (s, first_context, resumes, client_meta) = {
                let mut st = state.lock().unwrap();
                let Some(s) = st.sessions.get(&id).cloned() else { return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "Session not found")); };
                let first = prompt == "/context" && !st.context_seen;
                if prompt == "/context" { st.context_seen = true; }
                (s, first, st.resumes, st.client_meta.clone())
            };
            match prompt {
                "two-messages" => {
                    chunk(&cx, &id, "m1", "first")?;
                    let tool: ToolCall = serde_json::from_value(json!({"toolCallId":"t1","title":"Terminal","status":"pending"})).unwrap();
                    update(&cx, &id, SessionUpdate::ToolCall(tool))?;
                    let rename: ToolCallUpdate = serde_json::from_value(json!({"toolCallId":"t1","title":"Read notes.md"})).unwrap();
                    update(&cx, &id, SessionUpdate::ToolCallUpdate(rename))?;
                    let done: ToolCallUpdate = serde_json::from_value(json!({"toolCallId":"t1","status":"completed"})).unwrap();
                    update(&cx, &id, SessionUpdate::ToolCallUpdate(done))?;
                    chunk(&cx, &id, "m2", "second")?;
                },
                "report" => {
                    let bearer_hash = s.setup.bearer.as_deref().map(shadows_core::testing::hash_token);
                    let text = json!({"cwd":s.cwd,"session":id,"how":s.how,"model":s.model,"effort":s.effort,"mode":s.mode,"claude":std::env::var("CLAUDE_CODE_EXECUTABLE").unwrap_or_default(),
                        "mcp":s.setup.mcp,"bearer_hash":bearer_hash,"append":s.setup.append,"allowed":s.setup.allowed,"blocks":blocks,"resumes":resumes,"client_meta":client_meta}).to_string();
                    chunk(&cx, &id, "m1", &text)?;
                },
                "/context" => {
                    if first_context { tokio::time::sleep(Duration::from_secs(1)).await; }
                    chunk(&cx, &id, "m1", "| Category | Tokens | Percentage |\n| Messages | 3.8k | 0.4% |\n| System tools | 19.1k | 1.9% |\n| Free space | 923.9k | 92.4% |")?;
                },
                "hang" => {
                    chunk(&cx, &id, "m1", "waiting")?;
                    let mut cancelled = s.cancel.subscribe();
                    while !*cancelled.borrow() { if cancelled.changed().await.is_err() { break; } }
                    return responder.respond(PromptResponse::new(StopReason::Cancelled));
                },
                "wait-for-release" => {
                    chunk(&cx, &id, "m1", "waiting")?;
                    while !s.cwd.join("release").exists() { tokio::time::sleep(Duration::from_millis(20)).await; }
                },
                "ignore-cancel" => { chunk(&cx, &id, "m1", "waiting")?; std::future::pending::<()>().await; },
                "exit" => { chunk(&cx, &id, "m1", "exiting")?; std::process::exit(3); },
                "ask-permission" => {
                    let call: ToolCallUpdate = serde_json::from_value(json!({"toolCallId":"p1","title":"Run echo probe"})).unwrap();
                    let req = RequestPermissionRequest::new(id.clone(), call, vec![
                        PermissionOption::new("allow_once", "Allow", PermissionOptionKind::AllowOnce),
                        PermissionOption::new("reject_once", "Reject", PermissionOptionKind::RejectOnce),
                    ]);
                    let answer = cx.send_request(req).block_task().await?;
                    let chosen = match answer.outcome { agent_client_protocol::schema::v1::RequestPermissionOutcome::Selected(x) => x.option_id.to_string(), _ => "cancelled".into() };
                    chunk(&cx, &id, "m1", &format!("permission: {chosen}"))?;
                },
                "usage" => {
                    let u = UsageUpdate::new(1234, 200000).meta(json!({"_claude/model":"fake-large-answering"}).as_object().unwrap().clone());
                    update(&cx, &id, SessionUpdate::UsageUpdate(u))?;
                    chunk(&cx, &id, "m1", "usage")?;
                    let u = UsageUpdate::new(1234, 1000000).meta(json!({"_claude/model":"fake-large-answering","_claude/rateLimit":{"unifiedWindows":{"five_hour":{"utilization":0.25,"resetsAt":1790212200},"seven_day":{"utilization":0.5,"resetsAt":1790542800}}}}).as_object().unwrap().clone());
                    update(&cx, &id, SessionUpdate::UsageUpdate(u))?;
                },
                "refuse" => return responder.respond(PromptResponse::new(StopReason::MaxTokens)),
                line if line.starts_with("mcp ") => { let text = call_mcp(&s.setup, line).await; chunk(&cx, &id, "m1", &text)?; },
                // As the real adapter does: the title is sent after the turn has answered.
                line if line.starts_with("title ") => {
                    chunk(&cx, &id, "m1", "titled")?;
                    responder.respond(PromptResponse::new(StopReason::EndTurn))?;
                    let title = line.strip_prefix("title ").unwrap_or_default();
                    return update(&cx, &id, SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().title(title.to_owned())));
                },
                _ => { chunk(&cx, &id, "m1", "hello ")?; chunk(&cx, &id, "m1", "from fake_acp")?; },
            }
            responder.respond(PromptResponse::new(StopReason::EndTurn))
            })
        }}, agent_client_protocol::on_receive_request!())
        .connect_to(Stdio::new()).await
}
