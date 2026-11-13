//! One ACP client connection to one managed adapter process.
//!
//! Advertises AIR async tasks, forwards their raw lifecycle updates in order,
//! and sends the adapter's task stop request without cancelling the turn.
mod connection;
mod mapping;
mod permissions;
mod prompt;
mod session;

pub use crate::connection::{
    ImageBlock, McpServerSpec, Opened, PromptBlock, PromptCapabilities, PromptResult, SessionSetup,
    SessionStart, Steer, TurnEnd,
};
use agent_client_protocol::{Agent, v1::ConnectionTo};
pub use permissions::{
    PermissionError, PermissionPolicy, PreparedPermissionAnswer, WaitingPermission,
};
pub(crate) use permissions::{Permissions, Respond};

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
    permissions: permissions::Permissions,
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod tests;
