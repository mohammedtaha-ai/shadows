//! Fake prompt and steering requests.
use crate::prompts::{self, Turn, update};
use crate::session::Shared;
use crate::steer;
use agent_client_protocol::schema::v1::{ContentBlock, PromptRequest, PromptResponse};
use agent_client_protocol::{Client, ConnectionTo, Responder};
use serde_json::{Value, json};

pub(crate) struct PromptHandlers;

impl PromptHandlers {
    pub(crate) async fn prompt(
        state: Shared,
        r: PromptRequest,
        responder: Responder<PromptResponse>,
        cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
        let worker = cx.clone();
        let state = state.clone();
        cx.spawn(async move {
            let cx = worker;
            let id = r.session_id.to_string();
            // Only an image prompt is captured: the file lands in the
            // project folder, and other tests need that folder untouched.
            let has_image = r.prompt.iter().any(|b| matches!(b, ContentBlock::Image(_)));
            if has_image && let Some(s) = state.lock().unwrap().sessions.get(&id) {
                std::fs::write(
                    s.cwd.join("image-prompt.json"),
                    serde_json::to_vec(&r.prompt).unwrap(),
                )
                .unwrap();
            }
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
                    return responder.respond_with_error(agent_client_protocol::Error::new(
                        -32603,
                        "Session not found",
                    ));
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
            let mut response = PromptResponse::new(stop);
            if prompt.starts_with("turn-usage") {
                response.usage = Some(
                    serde_json::from_value(json!({
                        "totalTokens":64,"inputTokens":10,"outputTokens":4,
                        "cachedReadTokens":20,"cachedWriteTokens":30
                    }))
                    .unwrap(),
                );
            }
            responder.respond(response)?;
            match after {
                Some(item) => update(&cx, &id, item),
                None => Ok(()),
            }
        })
    }

    pub(crate) async fn steer(
        state: Shared,
        r: agent_client_protocol::UntypedMessage,
        responder: Responder<Value>,
        _cx: ConnectionTo<Client>,
    ) -> agent_client_protocol::Result<()> {
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
}
