//! One job: the child environment of an isolated Claude Code profile: its home,
//! provider and a key read from the daemon's environment at spawn.

use std::path::PathBuf;

/// An isolated Claude Code home and provider. `env` holds non-secret values.
#[derive(Debug, Clone)]
pub struct ClaudeProfile {
    pub name: String,
    pub home: PathBuf,
    /// Name of the daemon env var holding the provider key.
    pub key_env: String,
    pub env: Vec<(String, String)>,
}

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
    pub fn spawn_env(&self) -> Result<Vec<(String, String)>, ProfileError> {
        let key = std::env::var(&self.key_env)
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ProfileError::MissingKey(self.key_env.clone()))?;
        let mut env = vec![(
            "CLAUDE_CONFIG_DIR".to_string(),
            self.home.to_string_lossy().into_owned(),
        )];
        env.extend(self.env.iter().cloned());
        env.push(("ANTHROPIC_API_KEY".into(), String::new()));
        env.push(("ANTHROPIC_AUTH_TOKEN".into(), key));
        Ok(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_env_sets_home_key_and_blank_api_key() {
        // The crate forbids unsafe, so no set_var: PATH is always set.
        let k1 = std::env::var("PATH").unwrap();
        let p = ClaudeProfile {
            name: "a".into(),
            home: "C:/a/home".into(),
            key_env: "PATH".into(),
            env: vec![("ANTHROPIC_MODEL".into(), "MiniMax-M3".into())],
        };
        let env = p.spawn_env().unwrap();
        for pair in [
            ("CLAUDE_CONFIG_DIR", "C:/a/home"),
            ("ANTHROPIC_AUTH_TOKEN", k1.as_str()),
            ("ANTHROPIC_API_KEY", ""),
            ("ANTHROPIC_MODEL", "MiniMax-M3"),
        ] {
            assert!(
                env.contains(&(pair.0.to_string(), pair.1.to_string())),
                "{pair:?}"
            );
        }
    }

    #[test]
    fn missing_key_names_the_variable_only() {
        let p = ClaudeProfile {
            name: "a".into(),
            home: "h".into(),
            key_env: "T_MISSING_PROFILE_2".into(),
            env: vec![],
        };
        let e = p.spawn_env().unwrap_err();
        assert_eq!(
            e.to_string(),
            "aylul key env T_MISSING_PROFILE_2 is not set"
        );
    }
}
