//! ACP test agent. Prompts: ordinary (two chunks), `two-messages` (tool updates),
//! `report` (session state), `/context` (delayed first report), `hang` (cancel),
//! `ignore-cancel` (never), `exit` (code 3), `ask-permission` (reject),
//! `usage` (two context updates), and `refuse` (max_tokens).
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, ForkSessionRequest,
    ForkSessionResponse, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionRequest, ResumeSessionRequest, ResumeSessionResponse, SessionConfigOption,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, StopReason, TextContent, ToolCall, ToolCallUpdate, UsageUpdate,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Responder, Stdio};
use serde_json::json;
use tokio::sync::watch;

#[derive(Clone)]
struct Session {
    cwd: PathBuf,
    how: &'static str,
    model: String,
    effort: String,
    mode: String,
    cancel: watch::Sender<bool>,
}

#[derive(Default)]
struct State {
    next: usize,
    sessions: HashMap<String, Session>,
    context_seen: bool,
}
type Shared = Arc<Mutex<State>>;

fn options(s: &Session) -> Vec<SessionConfigOption> {
    let efforts: Vec<&str> = match s.model.as_str() {
        "fake-large" => vec!["low", "high", "max"],
        "fake-small" => vec!["low", "high"],
        _ => vec![],
    };
    let mut values = vec![
        json!({"id":"model","name":"Model","type":"select","currentValue":s.model,
            "options":[
                {"value":"fake-large","name":"fake-large"},
                {"value":"fake-small","name":"fake-small"},
                {"value":"fake-tiny","name":"fake-tiny"},
                {"value":"fake-locked","name":"fake-locked"}]}),
        json!({"id":"mode","name":"Mode","type":"select","currentValue":s.mode,
            "options":(["default","acceptEdits","plan","auto","bypassPermissions"].iter()
              .map(|v| json!({"value":v,"name":v})).collect::<Vec<_>>()) }),
    ];
    if !efforts.is_empty() {
        values.push(
            json!({"id":"thought_level","name":"Effort","type":"select","currentValue":s.effort,
            "options":efforts.iter().map(|v| json!({"value":v,"name":v})).collect::<Vec<_>>() }),
        );
    }
    values
        .into_iter()
        .map(|v| serde_json::from_value(v).expect("fake option schema"))
        .collect()
}

fn make_session(cwd: PathBuf, how: &'static str) -> Session {
    let (cancel, _) = watch::channel(false);
    Session {
        cwd,
        how,
        model: "fake-large".into(),
        effort: "high".into(),
        mode: "auto".into(),
        cancel,
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
        .on_receive_request(async |r: InitializeRequest, responder, _cx| {
            let caps: AgentCapabilities = serde_json::from_value(json!({"sessionCapabilities":{"fork":{},"resume":{}}})).unwrap();
            responder.respond(InitializeResponse::new(r.protocol_version).agent_capabilities(caps))
        }, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: NewSessionRequest, responder, _cx| {
            let mut st = state.lock().unwrap();
            st.next += 1;
            // Unique across adapter processes, as the real harness's ids are.
            let id = format!("fake-{}-{}", std::process::id(), st.next);
            let s = make_session(r.cwd, "new");
            let opts = options(&s);
            st.sessions.insert(id.clone(), s);
            responder.respond(NewSessionResponse::new(id).config_options(opts))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: ResumeSessionRequest, responder, _cx| {
            let id = r.session_id.to_string();
            let mut st = state.lock().unwrap();
            let how = if id.starts_with("fork-of-") { "fork" } else { "resume" };
            let s = make_session(r.cwd, how);
            let opts = options(&s);
            st.sessions.insert(id, s);
            responder.respond(ResumeSessionResponse::new().config_options(opts))
        }}, agent_client_protocol::on_receive_request!())
        .on_receive_request({ let state = state.clone(); async move |r: ForkSessionRequest, responder, _cx| {
            let st = state.lock().unwrap();
            if !st.sessions.contains_key(&r.session_id.to_string()) {
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
                    s.model = value;
                    s.effort = "high".into();
                    if s.mode == "auto" && s.model != "fake-large" { s.mode = "acceptEdits".into(); }
                },
                "thought_level" => s.effort = value,
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
            let (s, first_context) = {
                let mut st = state.lock().unwrap();
                let Some(s) = st.sessions.get(&id).cloned() else { return responder.respond_with_error(agent_client_protocol::Error::new(-32603, "Session not found")); };
                let first = prompt == "/context" && !st.context_seen;
                if prompt == "/context" { st.context_seen = true; }
                (s, first)
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
                    let text = json!({"cwd":s.cwd,"session":id,"how":s.how,"model":s.model,"effort":s.effort,"mode":s.mode,"claude":std::env::var("CLAUDE_CODE_EXECUTABLE").unwrap_or_default()}).to_string();
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
                _ => { chunk(&cx, &id, "m1", "hello ")?; chunk(&cx, &id, "m1", "from fake_acp")?; },
            }
            responder.respond(PromptResponse::new(StopReason::EndTurn))
            })
        }}, agent_client_protocol::on_receive_request!())
        .connect_to(Stdio::new()).await
}
