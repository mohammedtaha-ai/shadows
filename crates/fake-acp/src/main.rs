//! The ACP test agent binary: scripted answers over stdio, for the tests.
//! Besides the standard requests it answers `_session/steering` (`steer.rs`).

mod mcp;
mod prompts;
mod session;
mod steer;

use std::sync::Arc;

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ForkSessionRequest, ForkSessionResponse,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, ResumeSessionRequest, ResumeSessionResponse, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Responder, Stdio};
use serde_json::{Value, json};

use prompts::{Turn, update};
use session::{Shared, efforts, make_session, options, setup_of};

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("fake-claude-1");
        return Ok(());
    }
    let state: Shared = Arc::default();
    Agent
        .builder()
        .name("fake-acp")
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: InitializeRequest, responder, _cx| {
                    state.lock().unwrap().client_meta = r
                        .client_capabilities
                        .meta
                        .clone()
                        .map(Value::Object)
                        .unwrap_or(Value::Null);
                    let caps: AgentCapabilities = serde_json::from_value(json!({
                        "sessionCapabilities": {"fork": {}, "resume": {}},
                    }))
                    .unwrap();
                    responder.respond(
                        InitializeResponse::new(r.protocol_version).agent_capabilities(caps),
                    )
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: NewSessionRequest, responder, _cx| {
                    let mut st = state.lock().unwrap();
                    st.next += 1;
                    // Unique across adapter processes, as the real harness's ids are.
                    let id = format!("fake-{}-{}", std::process::id(), st.next);
                    let s = make_session(r.cwd, "new", setup_of(&r.mcp_servers, r.meta.as_ref()));
                    let opts = options(&s);
                    st.sessions.insert(id.clone(), s);
                    responder.respond(NewSessionResponse::new(id).config_options(opts))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: ResumeSessionRequest, responder, _cx| {
                    let id = r.session_id.to_string();
                    // A harness slow to open its session, as Claude Code's own
                    // startup is on Windows (seconds).
                    if id.starts_with("slow-") {
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    }
                    let mut st = state.lock().unwrap();
                    let how = if id.starts_with("fork-of-") {
                        "fork"
                    } else {
                        "resume"
                    };
                    st.resumes += 1;
                    let s = make_session(r.cwd, how, setup_of(&r.mcp_servers, r.meta.as_ref()));
                    let opts = options(&s);
                    st.sessions.insert(id, s);
                    responder.respond(ResumeSessionResponse::new().config_options(opts))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: ForkSessionRequest, responder, _cx| {
                    let st = state.lock().unwrap();
                    // The real adapter forks any session in Claude's own store; the
                    // fake's store is its process, so a fake id from another adapter
                    // process stands for one that exists.
                    let id = r.session_id.to_string();
                    if !st.sessions.contains_key(&id)
                        && !id.starts_with("fake-")
                        && !id.starts_with("fork-of-")
                    {
                        return responder.respond_with_error(agent_client_protocol::Error::new(
                            -32603,
                            "Session not found",
                        ));
                    }
                    responder.respond(ForkSessionResponse::new(format!(
                        "fork-of-{}",
                        r.session_id
                    )))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: SetSessionConfigOptionRequest, responder, _cx| {
                    let mut st = state.lock().unwrap();
                    let Some(s) = st.sessions.get_mut(&r.session_id.to_string()) else {
                        return responder.respond_with_error(agent_client_protocol::Error::new(
                            -32603,
                            "Session not found",
                        ));
                    };
                    let value = r
                        .value
                        .as_value_id()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    match r.config_id.to_string().as_str() {
                        "model" if value == "fake-locked" => {
                            return responder.respond_with_error(
                                agent_client_protocol::Error::new(
                                    -32603,
                                    concat!(
                                        "Usage credits are required for this model ",
                                        "· model not changed",
                                    ),
                                ),
                            );
                        }
                        "model" => {
                            s.effort = (!efforts(&value).is_empty()).then(|| "high".to_string());
                            s.model = value;
                            if s.mode == "auto" && s.model != "fake-large" {
                                s.mode = "acceptEdits".into();
                            }
                        }
                        "effort" if !efforts(&s.model).contains(&value.as_str()) => {
                            return responder.respond_with_error(
                                agent_client_protocol::Error::new(-32602, "effort not offered"),
                            );
                        }
                        "effort" => s.effort = Some(value),
                        "mode" if value == "auto" && s.model != "fake-large" => {
                            return responder.respond_with_error(
                                agent_client_protocol::Error::new(
                                    -32603,
                                    "auto mode is not available for this model",
                                ),
                            );
                        }
                        "mode" => s.mode = value,
                        _ => {}
                    }
                    responder.respond(SetSessionConfigOptionResponse::new(options(s)))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let state = state.clone();
                async move |r: CancelNotification, _cx| {
                    if let Some(s) = state
                        .lock()
                        .unwrap()
                        .sessions
                        .get(&r.session_id.to_string())
                    {
                        let _ = s.cancel.send(true);
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: PromptRequest,
                            responder: Responder<PromptResponse>,
                            cx: ConnectionTo<Client>| {
                    let worker = cx.clone();
                    let state = state.clone();
                    cx.spawn(async move {
                        let cx = worker;
                        let id = r.session_id.to_string();
                        let prompt = r
                            .prompt
                            .iter()
                            .find_map(|b| match b {
                                ContentBlock::Text(t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .unwrap_or("");
                        let blocks: Vec<&str> = r
                            .prompt
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::Text(t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .collect();
                        let (s, first_context, resumes, client_meta) = {
                            let mut st = state.lock().unwrap();
                            let Some(s) = st.sessions.get(&id).cloned() else {
                                return responder.respond_with_error(
                                    agent_client_protocol::Error::new(-32603, "Session not found"),
                                );
                            };
                            let first = prompt == "/context" && !st.context_seen;
                            if prompt == "/context" {
                                st.context_seen = true;
                            }
                            (s, first, st.resumes, st.client_meta.clone())
                        };
                        let turn = Turn {
                            id: &id,
                            prompt,
                            blocks: &blocks,
                            session: &s,
                            first_context,
                            resumes,
                            client_meta,
                        };
                        let (stop, after) = prompts::answer(&cx, turn).await?;
                        responder.respond(PromptResponse::new(stop))?;
                        match after {
                            Some(item) => update(&cx, &id, item),
                            None => Ok(()),
                        }
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        // Last: an `UntypedMessage` matches every method, so a handler after
        // it would never be reached.
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: agent_client_protocol::UntypedMessage,
                            responder: Responder<Value>,
                            _cx: ConnectionTo<Client>| {
                    if r.method() != "_session/steering" {
                        return responder.respond_with_error(agent_client_protocol::Error::new(
                            -32601,
                            "Method not found",
                        ));
                    }
                    let id = r
                        .params()
                        .get("sessionId")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let s = state.lock().unwrap().sessions.get(id).cloned();
                    match s {
                        Some(s) => responder.respond(steer::answer(&s, r.params())),
                        None => responder.respond_with_error(agent_client_protocol::Error::new(
                            -32603,
                            "Session not found",
                        )),
                    }
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}
