use std::path::{Path, PathBuf};

use shadows_process::ProcessSpec;

use crate::profile::{ClaudeProfile, ProfileError};

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
        self.process_spec_with(cwd, None, &[])
            .expect("no profile means no key to miss")
    }

    /// With a profile, its home and provider env follow the inherited values
    /// (a later pair of the same name wins at spawn).
    pub fn process_spec_with(
        &self,
        cwd: &Path,
        profile: Option<&ClaudeProfile>,
        stored: &[(String, String)],
    ) -> Result<ProcessSpec, ProfileError> {
        let mut env = environment(&self.agent);
        if let Some(profile) = profile {
            env.extend(profile.spawn_env_with(stored)?);
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
