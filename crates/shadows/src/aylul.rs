//! The daemon config supplies only the root of isolated agent homes.
use serde::Deserialize;
use shadows_core::{AppCore, AylulConfig};
use std::io;
use std::path::{Path, PathBuf};
pub const PROJECT_SLUG: &str = "aylul";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AylulSection {
    home_root: PathBuf,
}
pub fn load(path: &Path) -> anyhow::Result<AylulSection> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
pub async fn ensure_project(core: &AppCore, config: Option<&AylulConfig>) -> anyhow::Result<()> {
    if let Some(config) = config {
        let project = core
            .projects()
            .ensure_assistant(&config.project_slug, "AYLUL", &config.home_root)
            .await?;
        anyhow::ensure!(
            project.assistant,
            "aylul project slug is already used by a regular project"
        );
    }
    Ok(())
}
pub fn prepare(section: Option<AylulSection>) -> io::Result<Option<AylulConfig>> {
    let Some(section) = section else {
        return Ok(None);
    };
    if !section.home_root.is_absolute() {
        return Err(io::Error::other("aylul home_root must be absolute"));
    }
    std::fs::create_dir_all(&section.home_root)?;
    Ok(Some(AylulConfig {
        home_root: section.home_root,
        project_slug: PROJECT_SLUG.into(),
    }))
}
#[cfg(test)]
#[path = "aylul_tests.rs"]
mod tests;
