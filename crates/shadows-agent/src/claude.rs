use std::path::{Path, PathBuf};

use shadows_process::ProcessSpec;

/// The pinned Node ACP adapter launched with the configured Claude executable.
pub struct ClaudeAdapter {
    pub node: PathBuf,
    pub adapter: PathBuf,
    pub agent: PathBuf,
    pub adapter_version: String,
    pub agent_version: String,
}

impl ClaudeAdapter {
    pub fn process_spec(&self, cwd: &Path) -> ProcessSpec {
        ProcessSpec {
            executable: self.node.clone(),
            args: vec![self.adapter.to_string_lossy().into_owned()],
            cwd: cwd.to_path_buf(),
            env: environment(&self.agent),
            capture_stdout: true,
            pipe_stdin: true,
        }
    }
}

/// What the adapter's tree inherits from the daemon, by name (architecture
/// §4: the child environment is built explicitly, never cleared blindly).
/// Claude runs the Planner's shell tools, which need `PATH` to find `git` or
/// `ls`, and reads its own configuration and sign-in from the home directory.
/// No isolation profile exists yet; when one does, it replaces these values.
const INHERITED: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG"];
const PROVIDER: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "API_TIMEOUT_MS",
    "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
];
const NETWORK: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "NODE_USE_ENV_PROXY",
    "NODE_EXTRA_CA_CERTS",
];

fn environment(agent: &Path) -> Vec<(String, String)> {
    environment_with(agent, |name| std::env::var(name).ok())
}

fn environment_with(agent: &Path, get: impl Fn(&str) -> Option<String>) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = INHERITED
        .iter()
        .chain(PROVIDER.iter())
        .chain(NETWORK.iter())
        .filter_map(|key| get(key).map(|value| (key.to_string(), value)))
        .collect();
    env.push((
        "CLAUDE_CODE_EXECUTABLE".into(),
        agent.to_string_lossy().into_owned(),
    ));
    env
}

#[cfg(test)]
mod provider_environment_tests {
    use super::*;

    #[test]
    fn only_named_provider_settings_reach_the_acp_child() {
        let vars = |name: &str| match name {
            "PATH" => Some("/usr/bin".to_owned()),
            "ANTHROPIC_BASE_URL" => Some("https://api.minimax.io/anthropic".to_owned()),
            "ANTHROPIC_AUTH_TOKEN" => Some("synthetic-secret".to_owned()),
            "ANTHROPIC_MODEL" => Some("MiniMax-M3[1m]".to_owned()),
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS" => Some("1000000".to_owned()),
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW" => Some("1000000".to_owned()),
            "HTTPS_PROXY" => Some("http://session-proxy:8080".to_owned()),
            "NODE_USE_ENV_PROXY" => Some("1".to_owned()),
            "UNRELATED_SECRET" => Some("must-not-pass".to_owned()),
            _ => None,
        };
        let env = environment_with(Path::new("/opt/claude"), vars);
        assert!(env.contains(&(
            "ANTHROPIC_BASE_URL".into(),
            "https://api.minimax.io/anthropic".into()
        )));
        assert!(env.contains(&("ANTHROPIC_AUTH_TOKEN".into(), "synthetic-secret".into())));
        assert!(env.contains(&("ANTHROPIC_MODEL".into(), "MiniMax-M3[1m]".into())));
        assert!(env.contains(&("CLAUDE_CODE_MAX_CONTEXT_TOKENS".into(), "1000000".into())));
        assert!(env.contains(&("CLAUDE_CODE_AUTO_COMPACT_WINDOW".into(), "1000000".into())));
        assert!(env.contains(&("HTTPS_PROXY".into(), "http://session-proxy:8080".into())));
        assert!(env.contains(&("NODE_USE_ENV_PROXY".into(), "1".into())));
        assert!(env.iter().all(|(name, _)| name != "UNRELATED_SECRET"));
    }
}
