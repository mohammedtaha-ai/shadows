//! Shared apparatus: `fake_acp` (a test-support binary) stands in for Node and
//! the pinned ACP adapter, so a test runs a whole turn with nothing installed.
//! Its prompts script the agent: `hang`, `exit`, `two-messages`, `report`, ...

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::db::Storage;
use crate::harness::{Sessions, SessionsConfig};
use shadows_agent::claude::ClaudeAdapter;

/// The `/mcp` URL a daemon built from these fixtures reports: the address
/// `serve` would have bound, which no test binds.
pub const MCP_URL: &str = "http://127.0.0.1:4318/mcp";

pub fn adapter_at(node: PathBuf) -> Arc<ClaudeAdapter> {
    Arc::new(ClaudeAdapter {
        node,
        adapter: PathBuf::from("fake-adapter/dist/index.js"),
        agent: super::fake_acp_path(),
        // What `serve` reads from the adapter's package.json and from the
        // agent's `--version` (`fake_acp --version` answers the second).
        adapter_version: "fake-adapter-1".into(),
        agent_version: "fake-claude-1".into(),
    })
}

pub fn fake_adapter() -> Arc<ClaudeAdapter> {
    adapter_at(super::fake_acp_path())
}

/// A short cancel wait, so a Stop the fake ignores escalates quickly.
pub fn test_config() -> SessionsConfig {
    SessionsConfig {
        cancel_wait: Duration::from_secs(1),
        ..Default::default()
    }
}

/// Sessions over `fake_acp` on the core's `storage`, as `AppCore::start`
/// builds them: one pool, so a session's grant events wake subscribers.
pub fn fake_sessions(storage: Arc<Storage>) -> Arc<Sessions> {
    Sessions::new(fake_adapter(), storage, test_config())
}
