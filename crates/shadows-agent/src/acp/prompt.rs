//! Prompt requests and cancellation.
use super::{AcpError, Connection};
use crate::events::TurnUsage;
use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, PromptRequest, StopReason, TextContent,
};
use base64::Engine;
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Ended,
    Cancelled,
    Refused(String),
}

/// What `_session/steering` answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steer {
    /// The message went into the running prompt; that prompt answers for it.
    Injected,
    /// No prompt was running; nothing was started.
    PromptRequired,
}

/// Backend-neutral image input; protocol encoding belongs to Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlock {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

/// Ordered, backend-neutral prompt input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptBlock {
    Text(String),
    Image(ImageBlock),
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct PromptCapabilities {
    pub image: bool,
    pub image_reason: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct PromptResult {
    pub end: TurnEnd,
    pub usage: Option<TurnUsage>,
}

impl Connection {
    /// The person's text is the first content block; each `context` entry
    /// is one more text block after it.
    pub async fn prompt(
        &self,
        session: &str,
        text: &str,
        context: &[String],
    ) -> Result<PromptResult, AcpError> {
        let blocks = std::iter::once(text)
            .chain(context.iter().map(String::as_str))
            .map(|text| PromptBlock::Text(text.into()))
            .collect::<Vec<_>>();
        self.prompt_blocks(session, &blocks).await
    }
    pub fn prompt_capabilities(&self) -> PromptCapabilities {
        self.capabilities.clone()
    }
    pub async fn prompt_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<PromptResult, AcpError> {
        self.require_images(blocks)?;
        let active = self.costs.begin(session).map_err(AcpError::Rpc)?;
        let blocks = Self::acp_blocks(blocks);
        let response = self
            .cx
            .send_request(PromptRequest::new(session.to_owned(), blocks))
            .block_task()
            .await;
        let delta = active.finish();
        let r = response.map_err(AcpError::from_rpc)?;
        let usage = r
            .usage
            .and_then(|u| serde_json::to_value(u).ok())
            .and_then(|v| serde_json::from_value::<TurnUsage>(v).ok())
            .map(|mut u| {
                u.cost = delta;
                u
            });
        let end = match r.stop_reason {
            StopReason::EndTurn => TurnEnd::Ended,
            StopReason::Cancelled => TurnEnd::Cancelled,
            StopReason::MaxTokens => TurnEnd::Refused("max_tokens".into()),
            StopReason::MaxTurnRequests => TurnEnd::Refused("max_turn_requests".into()),
            StopReason::Refusal => TurnEnd::Refused("refusal".into()),
            other => TurnEnd::Refused(format!("{other:?}")),
        };
        Ok(PromptResult { end, usage })
    }
    /// Sends `text` into the session's running prompt, asking the
    /// adapter to start nothing when none runs.
    pub async fn steer(&self, session: &str, text: &str) -> Result<Steer, AcpError> {
        self.steer_blocks(session, &[PromptBlock::Text(text.into())])
            .await
    }
    fn require_images(&self, blocks: &[PromptBlock]) -> Result<(), AcpError> {
        if blocks.iter().any(|b| matches!(b, PromptBlock::Image(_))) && !self.capabilities.image {
            return Err(AcpError::Rpc(
                self.capabilities.image_reason.clone().unwrap_or_default(),
            ));
        }
        Ok(())
    }
    fn acp_blocks(blocks: &[PromptBlock]) -> Vec<ContentBlock> {
        blocks
            .iter()
            .map(|block| match block {
                PromptBlock::Text(text) => ContentBlock::Text(TextContent::new(text)),
                PromptBlock::Image(image) => {
                    let data = base64::engine::general_purpose::STANDARD.encode(&image.bytes);
                    ContentBlock::Image(agent_client_protocol::schema::v1::ImageContent::new(
                        data,
                        &image.media_type,
                    ))
                }
            })
            .collect()
    }
    pub async fn steer_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<Steer, AcpError> {
        self.require_images(blocks)?;
        let params = serde_json::json!({
            "sessionId": session,
            "prompt": Self::acp_blocks(blocks),
            "_meta": { "steering": { "idleBehavior": "promptRequired" } },
        });
        let request = agent_client_protocol::UntypedMessage::new("_session/steering", params)
            .map_err(AcpError::from_rpc)?;
        let answer = self
            .cx
            .send_request(request)
            .block_task()
            .await
            .map_err(AcpError::from_rpc)?;
        match answer.get("outcome").and_then(Value::as_str) {
            Some("injected") => Ok(Steer::Injected),
            Some("promptRequired") => Ok(Steer::PromptRequired),
            other => Err(AcpError::Rpc(format!(
                "unexpected steering answer {other:?}"
            ))),
        }
    }
    pub fn cancel(&self, session: &str) {
        self.permissions.refuse_session(session, "cancelled");
        let cx = self.cx.clone();
        let session = session.to_string();
        let _ = cx.send_notification(CancelNotification::new(session));
    }
}
