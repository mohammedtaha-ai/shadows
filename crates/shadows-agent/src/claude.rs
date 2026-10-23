use std::path::{Path, PathBuf};

use shadows_process::ProcessSpec;

use crate::profile::{ClaudeProfile, ProfileError};

/// The pinned Node ACP adapter launched with the configured Claude executable.
pub struct ClaudeAdapter {
    node: PathBuf,
    adapter: PathBuf,
    agent: PathBuf,
    adapter_version: String,
    agent_version: String,
}

impl ClaudeAdapter {
    pub fn new(
        node: PathBuf,
        adapter: PathBuf,
        agent: PathBuf,
        adapter_version: String,
        agent_version: String,
    ) -> Self {
        Self {
            node,
            adapter,
            agent,
            adapter_version,
            agent_version,
        }
    }

    pub fn adapter(&self) -> &Path {
        &self.adapter
    }

    pub fn agent(&self) -> &Path {
        &self.agent
    }

    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn agent_version(&self) -> &str {
        &self.agent_version
    }

    pub fn process_spec(&self, cwd: &Path) -> ProcessSpec {
        self.process_spec_with(cwd, None, &[], None)
            .expect("no profile means no key to miss")
    }

    /// With a profile, its home and provider env follow the inherited values
    /// (a later pair of the same name wins at spawn).
    pub fn process_spec_with(
        &self,
        cwd: &Path,
        profile: Option<&ClaudeProfile>,
        stored: &[(String, String)],
        token: Option<&str>,
    ) -> Result<ProcessSpec, ProfileError> {
        let mut env = environment(&self.agent);
        if let Some(profile) = profile {
            env.extend(profile.spawn_env_with(stored, token)?);
        }
        Ok(ProcessSpec {
            executable: self.node.clone(),
            args: vec![self.adapter.to_string_lossy().into_owned()],
            cwd: cwd.to_path_buf(),
            env,
            capture_stdout: true,
            pipe_stdin: true,
        })
    }
}

/// What the adapter's tree inherits from the daemon, by name (architecture
/// §4: the child environment is built explicitly, never cleared blindly).
/// Claude runs the Planner's shell tools, which need `PATH` to find `git` or
/// `ls`, and reads its own configuration and sign-in from the home directory.
/// An isolation profile (`process_spec_with`) adds its home and provider after
/// these, so its `CLAUDE_CONFIG_DIR` decides where Claude reads them instead.
const INHERITED: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG"];

fn environment(agent: &Path) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = INHERITED
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| (key.to_string(), value))
        })
        .collect();
    env.push((
        "CLAUDE_CODE_EXECUTABLE".into(),
        agent.to_string_lossy().into_owned(),
    ));
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_adapter_preserves_launch_identity() {
        let adapter = ClaudeAdapter::new(
            "node".into(),
            "adapter.js".into(),
            "claude".into(),
            "adapter-version".into(),
            "agent-version".into(),
        );
        let spec = adapter.process_spec(Path::new("project"));
        assert_eq!(spec.executable, Path::new("node"));
        assert_eq!(spec.args, ["adapter.js"]);
        assert_eq!(adapter.adapter_version(), "adapter-version");
        assert_eq!(adapter.agent_version(), "agent-version");
        assert!(
            spec.env
                .iter()
                .any(|(key, value)| { key == "CLAUDE_CODE_EXECUTABLE" && value == "claude" })
        );
    }
}
