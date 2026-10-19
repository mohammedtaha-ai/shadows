//! One job: the child environment of an isolated Claude Code profile: its home,
//! provider and a key read from an env file or the daemon's environment at spawn.

use std::path::{Path, PathBuf};

/// An isolated Claude Code home and provider. `env` holds non-secret values.
#[derive(Debug, Clone)]
pub struct ClaudeProfile {
    pub name: String,
    pub home: PathBuf,
    /// Name of the daemon env var holding the provider key.
    pub key_env: String,
    pub env: Vec<(String, String)>,
    /// A dotenv file read at every spawn, so an edit applies to the next session.
    pub env_file: Option<PathBuf>,
}

/// Never carries a value from the file or the environment.
#[derive(Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// The named variable is unset or empty.
    MissingKey(String),
    /// The env file could not be read: its path and the io error kind.
    EnvFile(PathBuf, std::io::ErrorKind),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKey(name) => write!(f, "aylul key env {name} is not set"),
            Self::EnvFile(path, kind) => write!(
                f,
                "aylul env file {} cannot be read: {kind}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProfileError {}

/// KEY=VALUE lines; blank lines and `#` comments skipped, an `export ` prefix
/// and one pair of surrounding quotes dropped, CRLF tolerated.
fn parse_dotenv(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            let unquoted = ["\"", "'"]
                .iter()
                .find_map(|q| value.strip_prefix(q).and_then(|v| v.strip_suffix(q)));
            Some((
                key.trim().to_string(),
                unquoted.unwrap_or(value).to_string(),
            ))
        })
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

fn read_env_file(path: &Path) -> Result<Vec<(String, String)>, ProfileError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ProfileError::EnvFile(path.to_path_buf(), e.kind()))?;
    Ok(parse_dotenv(&text))
}

impl ClaudeProfile {
    /// Order: the profile's `env`, the file's pairs, then the home, a blank
    /// API key and the token. The token is the file's `key_env` value, else
    /// the daemon's variable of that name.
    pub fn spawn_env(&self) -> Result<Vec<(String, String)>, ProfileError> {
        let file = match &self.env_file {
            Some(path) => read_env_file(path)?,
            None => Vec::new(),
        };
        let from_file = file
            .iter()
            .rev()
            .find(|(k, v)| *k == self.key_env && !v.is_empty())
            .map(|(_, v)| v.clone());
        let key = from_file
            .or_else(|| std::env::var(&self.key_env).ok().filter(|v| !v.is_empty()))
            .ok_or_else(|| ProfileError::MissingKey(self.key_env.clone()))?;
        // The profile's own values go last, so an extra `env` pair can never
        // move the home or replace the key (a later pair wins at spawn).
        let mut env = self.env.clone();
        env.extend(file);
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
            env: vec![
                ("ANTHROPIC_MODEL".into(), "MiniMax-M3".into()),
                // An extra pair naming the home or key never wins.
                ("CLAUDE_CONFIG_DIR".into(), "C:/elsewhere".into()),
                ("ANTHROPIC_AUTH_TOKEN".into(), "from-config".into()),
            ],
            env_file: None,
        };
        let env = p.spawn_env().unwrap();
        let last = |name: &str| env.iter().rev().find(|(k, _)| k == name).unwrap().1.clone();
        assert_eq!(last("CLAUDE_CONFIG_DIR"), "C:/a/home");
        assert_eq!(last("ANTHROPIC_AUTH_TOKEN"), k1);
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
    fn a_profile_spawn_spec_never_prints_its_key() {
        let key = std::env::var("PATH").unwrap();
        let adapter = crate::claude::ClaudeAdapter {
            node: "node".into(),
            adapter: "a.js".into(),
            agent: "claude".into(),
            adapter_version: String::new(),
            agent_version: String::new(),
        };
        let profile = ClaudeProfile {
            name: "a".into(),
            home: "h".into(),
            key_env: "PATH".into(),
            env: vec![],
            env_file: None,
        };
        let spec = adapter
            .process_spec_with("h".as_ref(), Some(&profile))
            .unwrap();
        let shown = format!("{spec:?}");
        assert!(shown.contains("ANTHROPIC_AUTH_TOKEN"), "{shown}");
        assert!(!shown.contains(&key));
    }

    #[test]
    fn missing_key_names_the_variable_only() {
        let p = ClaudeProfile {
            name: "a".into(),
            home: "h".into(),
            key_env: "T_MISSING_PROFILE_2".into(),
            env: vec![],
            env_file: None,
        };
        let e = p.spawn_env().unwrap_err();
        assert_eq!(
            e.to_string(),
            "aylul key env T_MISSING_PROFILE_2 is not set"
        );
    }

    /// A unique temp file holding `text`; no set_var, no shared state.
    fn temp_env(name: &str, text: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("shadows-profile-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env");
        std::fs::write(&path, text).unwrap();
        path
    }

    fn with_file(path: PathBuf, key_env: &str) -> ClaudeProfile {
        ClaudeProfile {
            name: "a".into(),
            home: "C:/a/home".into(),
            key_env: key_env.into(),
            env: vec![("ANTHROPIC_MODEL".into(), "from-config".into())],
            env_file: Some(path),
        }
    }

    fn last(env: &[(String, String)], name: &str) -> Option<String> {
        env.iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn env_file_pairs_reach_the_session() {
        let path = temp_env(
            "pairs",
            "# c\r\n\r\nA=1\r\nexport B=\"two words\"\nC='3'\nMK=k\n",
        );
        let env = with_file(path, "MK").spawn_env().unwrap();
        assert_eq!(last(&env, "A").as_deref(), Some("1"));
        assert_eq!(last(&env, "B").as_deref(), Some("two words"));
        assert_eq!(last(&env, "C").as_deref(), Some("3"));
    }

    #[test]
    fn env_file_key_becomes_the_token() {
        let path = temp_env("key", "T_NEVER_SET_KEY_9=from-file\n");
        let env = with_file(path, "T_NEVER_SET_KEY_9").spawn_env().unwrap();
        assert_eq!(
            last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref(),
            Some("from-file")
        );
    }

    #[test]
    fn env_file_cannot_override_home_or_token() {
        let path = temp_env(
            "override",
            "CLAUDE_CONFIG_DIR=C:/elsewhere\nANTHROPIC_AUTH_TOKEN=evil\nANTHROPIC_API_KEY=x\nMK=good\n",
        );
        let env = with_file(path, "MK").spawn_env().unwrap();
        assert_eq!(
            last(&env, "CLAUDE_CONFIG_DIR").as_deref(),
            Some("C:/a/home")
        );
        assert_eq!(last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref(), Some("good"));
        assert_eq!(last(&env, "ANTHROPIC_API_KEY").as_deref(), Some(""));
    }

    #[test]
    fn missing_env_file_names_the_path_not_values() {
        let path = std::env::temp_dir()
            .join("shadows-profile-absent-dir")
            .join(".env");
        let error = with_file(path.clone(), "MK").spawn_env().unwrap_err();
        let shown = error.to_string();
        assert!(shown.contains(&path.display().to_string()), "{shown}");
        assert!(shown.contains("cannot be read"), "{shown}");
    }

    #[test]
    fn an_edit_to_the_env_file_applies_on_the_next_spawn() {
        let path = temp_env("edit", "MK=one\n");
        let profile = with_file(path.clone(), "MK");
        let first = profile.spawn_env().unwrap();
        std::fs::write(&path, "MK=two\n").unwrap();
        let second = profile.spawn_env().unwrap();
        assert_eq!(last(&first, "ANTHROPIC_AUTH_TOKEN").as_deref(), Some("one"));
        assert_eq!(
            last(&second, "ANTHROPIC_AUTH_TOKEN").as_deref(),
            Some("two")
        );
    }

    #[test]
    fn profile_error_never_prints_a_file_value() {
        let path = temp_env("secret", "OTHER=sekrit-value\n");
        let error = with_file(path, "T_NEVER_SET_KEY_10")
            .spawn_env()
            .unwrap_err();
        for shown in [error.to_string(), format!("{error:?}")] {
            assert!(!shown.contains("sekrit-value"), "{shown}");
        }
        assert_eq!(error, ProfileError::MissingKey("T_NEVER_SET_KEY_10".into()));
    }
}
