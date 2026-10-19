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
    /// A dotenv file (absolute) read at each AYLUL spawn.
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

/// Seeds the home and builds what the core runs it with; `None` without a
/// section, so AYLUL is absent everywhere.
pub fn prepare(section: Option<AylulSection>) -> io::Result<Option<AylulConfig>> {
    let Some(section) = section else {
        return Ok(None);
    };
    if !section.home_root.is_absolute() {
        return Err(io::Error::other("aylul home_root must be absolute"));
    }
    if section.env_file.as_ref().is_some_and(|p| !p.is_absolute()) {
        return Err(io::Error::other("aylul env_file must be absolute"));
    }
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
    Ok(Some(AylulConfig {
        profile: ClaudeProfile {
            name: "aylul".into(),
            home,
            key_env: section.key_env,
            env,
            env_file: section.env_file,
        },
        project_slug: PROJECT_SLUG.into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeding_creates_both_files_in_an_empty_home() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("h");
        seed_profile(&home).unwrap();
        assert!(
            std::fs::read_to_string(home.join("CLAUDE.md"))
                .unwrap()
                .contains("AYLUL")
        );
        assert!(
            std::fs::read_to_string(home.join("settings.json"))
                .unwrap()
                .contains("permissions")
        );
    }

    #[test]
    fn seeding_keeps_existing_profile_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "mine").unwrap();
        seed_profile(tmp.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap(),
            "mine"
        );
        assert!(tmp.path().join("settings.json").exists());
    }

    #[test]
    fn a_section_file_with_an_unknown_field_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.json");
        std::fs::write(
            &file,
            r#"{"home_root":"/x","key_env":"K","base_url":"u","model":"m","key":"s"}"#,
        )
        .unwrap();
        assert!(load(&file).is_err());
    }

    #[test]
    fn a_section_with_a_relative_env_file_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let section: AylulSection = serde_json::from_value(serde_json::json!({
            "home_root": tmp.path(), "key_env": "K", "base_url": "u", "model": "m",
            "env_file": "relative/.env",
        }))
        .unwrap();
        assert!(prepare(Some(section)).is_err());
    }

    #[test]
    fn no_aylul_config_means_no_aylul_project() {
        assert!(prepare(None).unwrap().is_none());
    }

    #[test]
    fn a_section_builds_the_profile_with_context_window_and_no_secret() {
        let tmp = tempfile::tempdir().unwrap();
        let section: AylulSection = serde_json::from_value(serde_json::json!({
            "home_root": tmp.path(), "key_env": "MINIMAX_API_KEY",
            "base_url": "https://api.minimax.io/anthropic", "model": "MiniMax-M3",
        }))
        .unwrap();
        let config = prepare(Some(section)).unwrap().unwrap();
        let env = &config.profile.env;
        assert!(env.contains(&("ANTHROPIC_MODEL".into(), "MiniMax-M3".into())));
        assert!(env.contains(&("CLAUDE_CODE_MAX_CONTEXT_TOKENS".into(), "1000000".into())));
        assert_eq!(config.profile.key_env, "MINIMAX_API_KEY");
        assert_eq!(
            config.profile.home,
            tmp.path().join("profiles").join("aylul")
        );
    }
}
