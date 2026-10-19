//! One job: the child environment of an isolated Claude Code profile: its home,
//! provider and a key resolved from stored pairs or the daemon environment at spawn.

use std::path::PathBuf;

/// An isolated Claude Code home and provider. `env` holds non-secret values.
#[derive(Debug, Clone)]
pub struct ClaudeProfile {
    pub name: String,
    pub home: PathBuf,
    /// Name of the stored pair, with the daemon variable as fallback.
    pub key_env: String,
    pub env: Vec<(String, String)>,
}

/// Never carries a value from storage or the environment.
#[derive(Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// The named variable is unset or empty.
    MissingKey(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKey(name) => write!(f, "aylul key env {name} is not set"),
        }
    }
}

impl std::error::Error for ProfileError {}

impl ClaudeProfile {
    /// Order: the profile's `env`, the stored pairs, then the home, a blank
    /// API key and the token. The token is the stored `key_env` value, else
    /// the daemon's variable of that name.
    pub fn spawn_env_with(
        &self,
        stored: &[(String, String)],
    ) -> Result<Vec<(String, String)>, ProfileError> {
        let from_stored = stored
            .iter()
            .rev()
            .find(|(k, v)| *k == self.key_env && !v.is_empty())
            .map(|(_, v)| v.clone());
        let key = from_stored
            .or_else(|| std::env::var(&self.key_env).ok().filter(|v| !v.is_empty()))
            .ok_or_else(|| ProfileError::MissingKey(self.key_env.clone()))?;
        // The profile's own values go last, so an extra `env` pair can never
        // move the home or replace the key (a later pair wins at spawn).
        let mut env = self.env.clone();
        env.extend_from_slice(stored);
        env.push((
            "CLAUDE_CONFIG_DIR".into(),
            self.home.to_string_lossy().into_owned(),
        ));
        env.push(("ANTHROPIC_API_KEY".into(), String::new()));
        env.push(("ANTHROPIC_AUTH_TOKEN".into(), key));
        Ok(env)
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
