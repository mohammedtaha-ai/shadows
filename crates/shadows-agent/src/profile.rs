//! Child environment of an isolated Claude Code profile.
use std::path::PathBuf;
#[derive(Debug, Clone)]
pub struct ClaudeProfile {
    name: String,
    home: PathBuf,
    env: Vec<(String, String)>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProfileError {
    MissingKey(String),
}
impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKey(name) => write!(f, "provider key missing for {name}"),
        }
    }
}
impl std::error::Error for ProfileError {}
impl ClaudeProfile {
    pub fn new(name: String, home: PathBuf, env: Vec<(String, String)>) -> Self {
        Self { name, home, env }
    }

    /// The token is supplied by the core; daemon variables are never a source.
    pub fn spawn_env_with(
        &self,
        stored: &[(String, String)],
        token: Option<&str>,
    ) -> Result<Vec<(String, String)>, ProfileError> {
        let token = token
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProfileError::MissingKey(self.name.clone()))?;
        let mut env = stored.to_vec();
        env.extend_from_slice(&self.env);
        env.push((
            "CLAUDE_CONFIG_DIR".into(),
            self.home.to_string_lossy().into_owned(),
        ));
        env.push(("ANTHROPIC_API_KEY".into(), String::new()));
        env.push(("ANTHROPIC_AUTH_TOKEN".into(), token.into()));
        Ok(env)
    }
}
#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
