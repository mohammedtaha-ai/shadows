use std::path::{Path, PathBuf};

use crate::process::ProcessSpec;

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
            env: vec![(
                "CLAUDE_CODE_EXECUTABLE".into(),
                self.agent.to_string_lossy().into_owned(),
            )],
            capture_stdout: true,
            pipe_stdin: true,
        }
    }
}
