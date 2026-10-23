//! The ACP test agent binary: scripted answers over stdio, for the tests.
//! Besides the standard requests it answers `_session/steering` (`steer.rs`).

mod handlers;
mod mcp;
mod prompts;
mod session;
mod steer;

use std::sync::Arc;

use agent_client_protocol::schema::v1::{
    CancelNotification, ForkSessionRequest, InitializeRequest, NewSessionRequest, PromptRequest,
    PromptResponse, ResumeSessionRequest, SetSessionConfigOptionRequest,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Responder, Stdio};
use serde_json::Value;

use handlers::{PromptHandlers, SessionHandlers};
use session::Shared;

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
                    SessionHandlers::initialize(state.clone(), r, responder, _cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: NewSessionRequest, responder, cx| {
                    SessionHandlers::new_session(state.clone(), r, responder, cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: ResumeSessionRequest, responder, cx| {
                    SessionHandlers::resume(state.clone(), r, responder, cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: ForkSessionRequest, responder, _cx| {
                    SessionHandlers::fork(state.clone(), r, responder, _cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: SetSessionConfigOptionRequest, responder, _cx| {
                    SessionHandlers::set_option(state.clone(), r, responder, _cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let state = state.clone();
                async move |r: CancelNotification, _cx| {
                    SessionHandlers::cancel(state.clone(), r, _cx).await
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
                    PromptHandlers::prompt(state.clone(), r, responder, cx).await
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
                    PromptHandlers::steer(state.clone(), r, responder, _cx).await
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}
