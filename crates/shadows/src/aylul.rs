//! One job: turning the daemon's `aylul` config section into a seeded profile
//! home and the `AylulConfig` the core runs it with.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use shadows_core::{AppCore, AylulConfig, ClaudeProfile};

const PERSONA: &str = include_str!("../assets/aylul-profile/CLAUDE.md");
const SETTINGS: &str = include_str!("../assets/aylul-profile/settings.json");

/// The project AYLUL's conversations live in.
pub const PROJECT_SLUG: &str = "aylul";

/// Claude Code does not know MiniMax-M3 and would assume 200k tokens; these
/// declare its real window (docs/evidence/aylul/2026-10-19-acp-adapter-profile-probe.md).
const CONTEXT_WINDOW: [(&str, &str); 2] = [
    ("CLAUDE_CODE_MAX_CONTEXT_TOKENS", "1000000"),
    ("CLAUDE_CODE_AUTO_COMPACT_WINDOW", "1000000"),
];

/// Holds names and non-secret values only: the key stays in the environment
/// variable `key_env` names.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AylulSection {
    home_root: PathBuf,
    key_env: String,
    base_url: String,
    model: String,
    #[serde(default)]
    env: BTreeMap<String, String>,
    /// A dotenv file (absolute) imported once at startup.
    #[serde(default)]
    env_file: Option<PathBuf>,
}

/// Creates the home and writes the two profile files that are absent; a file
/// that exists, edited or not, is never touched.
pub fn seed_profile(home: &Path) -> io::Result<()> {
    std::fs::create_dir_all(home)?;
    for (name, text) in [("CLAUDE.md", PERSONA), ("settings.json", SETTINGS)] {
        let path = home.join(name);
        if !path.exists() {
            std::fs::write(path, text)?;
        }
    }
    Ok(())
}

/// Reads a section file; an unknown field is refused.
pub fn load(path: &Path) -> anyhow::Result<AylulSection> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// Creates AYLUL's project once the core is up; nothing without a profile.
pub async fn ensure_project(core: &AppCore, config: Option<&AylulConfig>) -> anyhow::Result<()> {
    if let Some(config) = config {
        core.projects()
            .ensure_assistant(&config.project_slug, "AYLUL", &config.profile.home)
            .await?;
    }
    Ok(())
}

/// Startup import errors are retryable and never expose source text or stop the daemon.
pub async fn import_env(core: &AppCore, config: &AylulConfig, path: &Path) {
    let profile = &config.profile.name;
    match core.secrets().import_needed(profile).await {
        Ok(false) => return,
        Ok(true) => {}
        Err(_) => {
            tracing::warn!("secrets.import_unavailable");
            return;
        }
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            tracing::warn!(kind = ?error.kind(), "secrets.import_read_failed");
            return;
        }
    };
    match core
        .secrets()
        .import_env_once(profile, parse_dotenv(&text))
        .await
    {
        Ok(Some(report)) => tracing::info!(stored = report.stored, skipped = ?report.skipped,
            "secrets.imported"),
        Ok(None) => {}
        Err(_) => tracing::warn!("secrets.import_failed"),
    }
}

/// Seeds the home and builds what the core runs it with; `None` without a
/// section, so AYLUL is absent everywhere.
pub fn prepare(
    section: Option<AylulSection>,
) -> io::Result<Option<(AylulConfig, Option<PathBuf>)>> {
    let Some(section) = section else {
        return Ok(None);
    };
    if !section.home_root.is_absolute() {
        return Err(io::Error::other("aylul home_root must be absolute"));
    }
    if section.env_file.as_ref().is_some_and(|p| !p.is_absolute()) {
        return Err(io::Error::other("aylul env_file must be absolute"));
    }
    shadows_core::validate_env_name(&section.key_env).map_err(io::Error::other)?;
    let home = section.home_root.join("profiles").join("aylul");
    seed_profile(&home)?;
    let mut env: Vec<(String, String)> = vec![("ANTHROPIC_BASE_URL".into(), section.base_url)];
    for key in [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    ] {
        env.push((key.into(), section.model.clone()));
    }
    env.extend(CONTEXT_WINDOW.map(|(k, v)| (k.to_string(), v.to_string())));
    env.extend(section.env);
    Ok(Some((
        AylulConfig {
            profile: ClaudeProfile {
                name: "aylul".into(),
                home,
                key_env: section.key_env,
                env,
            },
            project_slug: PROJECT_SLUG.into(),
        },
        section.env_file,
    )))
}

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

#[cfg(test)]
#[path = "aylul_tests.rs"]
mod tests;
