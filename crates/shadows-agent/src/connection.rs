//! Operations shared by the two managed harness connections.
use std::path::Path;

use shadows_process::ProcessHandle;

use crate::acp::{
    self, PermissionError, PermissionPolicy, PreparedPermissionAnswer, WaitingPermission,
};
use crate::choices::Offered;
use crate::events::{HarnessEvent, TurnUsage};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConnectionError {
    #[error("harness connection closed")]
    Closed,
    #[error("harness request failed: {0}")]
    Rpc(String),
    #[error("invalid harness protocol: {0}")]
    Protocol(&'static str),
}

impl From<crate::acp::AcpError> for ConnectionError {
    fn from(error: crate::acp::AcpError) -> Self {
        match error {
            crate::acp::AcpError::Closed => Self::Closed,
            crate::acp::AcpError::Rpc(message) => Self::Rpc(message),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionStart {
    New,
    Resume(String),
    Fork(String),
}

#[derive(Debug, Clone, Default)]
pub struct SessionSetup {
    pub mcp: Vec<McpServerSpec>,
    pub append: Option<String>,
    pub allowed_tools: Vec<String>,
    pub tools: Option<Vec<String>>,
    pub setting_sources: Option<Vec<String>>,
}

/// Transport settings. The header and environment values stay out of Debug.
#[derive(Clone)]
pub enum McpServerSpec {
    Http {
        name: String,
        url: String,
        headers: Vec<(String, String)>,
    },
    Stdio {
        name: String,
        command: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    },
}

impl std::fmt::Debug for McpServerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { name, url, .. } => f
                .debug_struct("Http")
                .field("name", name)
                .field("url", url)
                .finish_non_exhaustive(),
            Self::Stdio { name, command, .. } => f
                .debug_struct("Stdio")
                .field("name", name)
                .field("command", command)
                .finish_non_exhaustive(),
        }
    }
}

#[derive(Debug)]
pub struct Opened {
    pub session_id: String,
    pub options: Offered,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Ended,
    Cancelled,
    Refused(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steer {
    Injected,
    PromptRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlock {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

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

#[derive(Clone)]
pub enum ManagedConnection {
    Acp(acp::Connection),
    Codex(crate::codex::Connection),
}

impl ManagedConnection {
    pub async fn open(
        handle: &mut ProcessHandle,
        codex: bool,
        policy: PermissionPolicy,
        events: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
    ) -> Result<Self, ConnectionError> {
        if codex {
            Ok(Self::Codex(
                crate::codex::Connection::open(handle, events).await?,
            ))
        } else {
            Ok(Self::Acp(
                acp::Connection::open(handle, policy, events).await?,
            ))
        }
    }

    pub async fn start_session(
        &self,
        cwd: &Path,
        start: SessionStart,
        setup: &SessionSetup,
        permitted_tools: &[&str],
    ) -> Result<Opened, ConnectionError> {
        match self {
            Self::Acp(connection) => Ok(connection.start_session(cwd, start, setup).await?),
            Self::Codex(connection) => {
                connection
                    .start_session(cwd, start, setup, permitted_tools)
                    .await
            }
        }
    }

    pub async fn set_option(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Offered, ConnectionError> {
        match self {
            Self::Acp(connection) => Ok(connection.set_option(session, id, value).await?),
            Self::Codex(connection) => connection.set_option(session, id, value),
        }
    }

    pub fn is_closed(&self) -> bool {
        match self {
            Self::Acp(connection) => connection.is_closed(),
            Self::Codex(connection) => connection.is_closed(),
        }
    }

    pub fn prompt_capabilities(&self) -> PromptCapabilities {
        match self {
            Self::Acp(connection) => connection.prompt_capabilities(),
            Self::Codex(connection) => connection.prompt_capabilities(),
        }
    }

    pub async fn prompt(
        &self,
        session: &str,
        text: &str,
        context: &[String],
    ) -> Result<PromptResult, ConnectionError> {
        let blocks = std::iter::once(text)
            .chain(context.iter().map(String::as_str))
            .map(|text| PromptBlock::Text(text.into()))
            .collect::<Vec<_>>();
        self.prompt_blocks(session, &blocks).await
    }

    pub async fn prompt_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<PromptResult, ConnectionError> {
        match self {
            Self::Acp(connection) => Ok(connection.prompt_blocks(session, blocks).await?),
            Self::Codex(connection) => connection.prompt_blocks(session, blocks).await,
        }
    }

    pub async fn steer_blocks(
        &self,
        session: &str,
        blocks: &[PromptBlock],
    ) -> Result<Steer, ConnectionError> {
        match self {
            Self::Acp(connection) => Ok(connection.steer_blocks(session, blocks).await?),
            Self::Codex(_) => Err(ConnectionError::Rpc("Codex steering is unavailable".into())),
        }
    }

    pub fn cancel(&self, session: &str) {
        match self {
            Self::Acp(connection) => connection.cancel(session),
            Self::Codex(connection) => connection.cancel(session),
        }
    }

    pub async fn stop_task(&self, session: &str, task: &str) -> Result<bool, ConnectionError> {
        match self {
            Self::Acp(connection) => Ok(connection.stop_task(session, task).await?),
            Self::Codex(_) => Err(ConnectionError::Rpc(
                "Codex background tasks are unavailable".into(),
            )),
        }
    }

    pub fn held_permissions(&self) -> Vec<WaitingPermission> {
        match self {
            Self::Acp(connection) => connection.held_permissions(),
            Self::Codex(_) => Vec::new(),
        }
    }

    pub async fn prepare_permission_answer(
        &self,
        request: &str,
        option: &str,
    ) -> Result<PreparedPermissionAnswer, PermissionError> {
        match self {
            Self::Acp(connection) => connection.prepare_permission_answer(request, option).await,
            Self::Codex(_) => Err(PermissionError::Unknown),
        }
    }

    pub async fn refuse_held(&self, reason: &str) {
        if let Self::Acp(connection) = self {
            connection.refuse_held(reason).await;
        }
    }
}
