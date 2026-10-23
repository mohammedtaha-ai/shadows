//! Fake session lifecycle requests.
use crate::prompts::update;
use crate::session::{self, Shared, efforts, make_session, options, setup_of};
use agent_client_protocol::schema::v1::{
    AgentCapabilities, AvailableCommandsUpdate, CancelNotification, ForkSessionRequest,
    ForkSessionResponse, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, ResumeSessionRequest, ResumeSessionResponse, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse,
};
use agent_client_protocol::{Client, ConnectionTo, Responder};
use serde_json::{Value, json};

pub(crate) struct SessionHandlers;

impl SessionHandlers {
    pub(crate) async fn initialize(
        state: Shared,
        r: InitializeRequest,
        responder: Responder<InitializeResponse>,
        _cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
        state.lock().unwrap().client_meta = r
            .client_capabilities
            .meta
            .clone()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let caps: AgentCapabilities = serde_json::from_value(json!({
            "sessionCapabilities": {"fork": {}, "resume": {}},
            "promptCapabilities": {"image": !std::env::args()
                .any(|a| a == "--no-images")
                && !std::path::Path::new(".fake-acp-no-images").exists()},
        }))
        .unwrap();
        responder.respond(InitializeResponse::new(r.protocol_version).agent_capabilities(caps))
    }

    pub(crate) async fn new_session(
        state: Shared,
        r: NewSessionRequest,
        responder: Responder<NewSessionResponse>,
        cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
        let mut st = state.lock().unwrap();
        st.next += 1;
        // Unique across adapter processes, as the real harness's ids are.
        let id = format!("fake-{}-{}", std::process::id(), st.next);
        let s = make_session(r.cwd, "new", setup_of(&r.mcp_servers, r.meta.as_ref()));
        let opts = options(&s);
        st.sessions.insert(id.clone(), s);
        drop(st);
        responder.respond(NewSessionResponse::new(id.clone()).config_options(opts))?;
        let list = AvailableCommandsUpdate::new(session::commands());
        update(&cx, &id, SessionUpdate::AvailableCommandsUpdate(list))
    }

    pub(crate) async fn resume(
        state: Shared,
        r: ResumeSessionRequest,
        responder: Responder<ResumeSessionResponse>,
        cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
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
        st.sessions.insert(id.clone(), s);
        drop(st);
        responder.respond(ResumeSessionResponse::new().config_options(opts))?;
        let list = AvailableCommandsUpdate::new(session::commands());
        update(&cx, &id, SessionUpdate::AvailableCommandsUpdate(list))
    }

    pub(crate) async fn fork(
        state: Shared,
        r: ForkSessionRequest,
        responder: Responder<ForkSessionResponse>,
        _cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
        let st = state.lock().unwrap();
        // The real adapter forks any session in Claude's own store; the
        // fake's store is its process, so a fake id from another adapter
        // process stands for one that exists.
        let id = r.session_id.to_string();
        if !st.sessions.contains_key(&id) && !id.starts_with("fake-") && !id.starts_with("fork-of-")
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

    pub(crate) async fn set_option(
        state: Shared,
        r: SetSessionConfigOptionRequest,
        responder: Responder<SetSessionConfigOptionResponse>,
        _cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
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
                return responder.respond_with_error(agent_client_protocol::Error::new(
                    -32603,
                    concat!(
                        "Usage credits are required for this model ",
                        "· model not changed",
                    ),
                ));
            }
            "model" => {
                s.effort = (!efforts(&value).is_empty()).then(|| "high".to_string());
                s.model = value;
                if s.mode == "auto" && s.model != "fake-large" {
                    s.mode = "acceptEdits".into();
                }
            }
            "effort" if !efforts(&s.model).contains(&value.as_str()) => {
                return responder.respond_with_error(agent_client_protocol::Error::new(
                    -32602,
                    "effort not offered",
                ));
            }
            "effort" => s.effort = Some(value),
            "mode" if value == "auto" && s.model != "fake-large" => {
                return responder.respond_with_error(agent_client_protocol::Error::new(
                    -32603,
                    "auto mode is not available for this model",
                ));
            }
            "mode" => s.mode = value,
            _ => {}
        }
        responder.respond(SetSessionConfigOptionResponse::new(options(s)))
    }

    pub(crate) async fn cancel(
        state: Shared,
        r: CancelNotification,
        _cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
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
}
