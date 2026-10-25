//! One ACP client connection to one managed adapter process.
mod connection;
mod mapping;
mod prompt;
mod session;

use agent_client_protocol::{Agent, ConnectionTo};
pub use prompt::{ImageBlock, PromptBlock, PromptCapabilities, PromptResult, Steer, TurnEnd};
pub use session::{McpServerSpec, Opened, SessionSetup, SessionStart};

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("ACP connection closed")]
    Closed,
    #[error("ACP request failed: {0}")]
    Rpc(String),
}

#[derive(Clone)]
pub struct Connection {
    cx: ConnectionTo<Agent>,
    capabilities: PromptCapabilities,
    costs: crate::usage::Costs,
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod tests;
