//! Project the enabled library into one isolated agent home at opening.
use super::profile_home::Homes;
use crate::extensions::Extensions;
use crate::{CoreError, Item};
use serde_json::json;
use std::{io::Write, path::Path};

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Manifest {
    skills: Vec<String>,
    plugins: Vec<String>,
    generation: String,
    #[serde(default)]
    items: Vec<String>,
}
fn invalid() -> CoreError {
    CoreError::ProfileInvalid("cannot project agent extensions".into())
}
fn safe_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn local(path: &Path) -> Result<(), CoreError> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(invalid());
            }
        }
        if meta.file_type().is_symlink() {
            return Err(invalid());
        }
    }
    Ok(())
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let temp = path.with_file_name(format!(".extension-write-{}", uuid::Uuid::new_v4()));
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
        return Err(invalid());
    }
    Ok(())
}
fn copy(root: &Path, stage: &Path, item: &Item) -> Result<(), CoreError> {
    if uuid::Uuid::parse_str(&item.id).is_err() || !safe_name(&item.name) {
        return Err(invalid());
    }
    Extensions::copy_tree(&root.join("library").join(&item.id), stage)?;
    Ok(())
}
fn project(
    root: &Path,
    home: &Path,
    skills: Vec<Item>,
    plugins: Vec<Item>,
) -> Result<(), CoreError> {
    let mut settings = Homes::check(home)?;
    let settings_path = home.join("settings.json");
    let prior_settings = std::fs::read(&settings_path).map_err(|_| invalid())?;
    let manifest_path = home.join(".aylul-extensions.json");
    local(&manifest_path)?;
    local(&settings_path)?;
    local(&home.join("skills"))?;
    local(&home.join(".aylul-extension-generations"))?;
    let prior_bytes = match std::fs::read(&manifest_path) {
        Ok(v) => Some(v),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(invalid()),
    };
    let prior: Manifest = match &prior_bytes {
        Some(v) => serde_json::from_slice(v).map_err(|_| invalid())?,
        None => Manifest::default(),
    };
    if skills.is_empty() && plugins.is_empty() && prior_bytes.is_none() {
        return Ok(());
    }
    if prior
        .skills
        .iter()
        .chain(&prior.plugins)
        .any(|s| !safe_name(s))
        || (!prior.generation.is_empty() && uuid::Uuid::parse_str(&prior.generation).is_err())
    {
        return Err(invalid());
    }
    if prior_bytes.is_none()
        && settings
            .pointer("/extraKnownMarketplaces/aylul-managed")
            .is_some()
    {
        return Err(CoreError::ProfileInvalid(
            "managed marketplace name is already owned".into(),
        ));
    }
    for item in &skills {
        local(&home.join("skills").join(&item.name))?;
        if home.join("skills").join(&item.name).exists() && !prior.skills.contains(&item.name) {
            return Err(CoreError::ProfileInvalid(
                "skill name is already owned".into(),
            ));
        }
    }
    for name in &prior.skills {
        local(&home.join("skills").join(name))?;
    }
    let item_ids: Vec<_> = skills
        .iter()
        .chain(&plugins)
        .map(|i| i.id.clone())
        .collect();
    let expected_market = home
        .join(".aylul-extension-generations")
        .join(&prior.generation)
        .join("marketplace");
    if prior_bytes.is_some()
        && prior.items == item_ids
        && expected_market.is_dir()
        && skills
            .iter()
            .all(|i| home.join("skills").join(&i.name).join("SKILL.md").is_file())
        && prior.plugins.iter().all(|name| {
            let enabled = plugins.iter().any(|i| &i.name == name);
            settings["enabledPlugins"][format!("{name}@aylul-managed")] == enabled
        })
        && settings
            .pointer("/extraKnownMarketplaces/aylul-managed/source/path")
            .and_then(|v| v.as_str())
            == expected_market.to_str()
    {
        return Ok(());
    }
    let mut managed_plugins = prior.plugins.clone();
    for item in &plugins {
        if !managed_plugins.contains(&item.name) {
            managed_plugins.push(item.name.clone());
        }
    }
    let generation = uuid::Uuid::new_v4().to_string();
    let stage = home.join(".aylul-extension-generations").join(&generation);
    let marketplace = stage.join("marketplace");
    std::fs::create_dir_all(marketplace.join(".claude-plugin")).map_err(|_| invalid())?;
    let result = (|| {
        for item in &skills {
            copy(root, &stage.join("skills").join(&item.name), item)?;
        }
        for item in &plugins {
            let target = marketplace.join("plugins").join(&item.name);
            copy(root, &target, item)?;
            let manifest_path = target.join(".claude-plugin/plugin.json");
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&manifest_path).map_err(|_| invalid())?)
                    .map_err(|_| invalid())?;
            manifest["version"] = format!("0.0.0-aylul{}", item.id.replace('-', "")).into();
            std::fs::write(
                &manifest_path,
                serde_json::to_vec(&manifest).map_err(|_| invalid())?,
            )
            .map_err(|_| invalid())?;
        }
        let entries: Vec<_> = plugins
            .iter()
            .map(|p| {
                json!({"name":p.name,"version":format!("0.0.0-aylul{}",p.id.replace('-',"")),
            "source":format!("./plugins/{}",p.name)})
            })
            .collect();
        std::fs::write(
            marketplace.join(".claude-plugin/marketplace.json"),
            serde_json::to_vec(&json!({
                "name":"aylul-managed","owner":{"name":"AYLUL"},"plugins":entries
            }))
            .map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        for key in ["extraKnownMarketplaces", "enabledPlugins"] {
            if settings.get(key).is_none() {
                settings[key] = json!({});
            }
            if !settings[key].is_object() {
                return Err(invalid());
            }
        }
        settings["extraKnownMarketplaces"]["aylul-managed"] = json!({"source":{
            "source":"directory","path":marketplace}});
        for name in &prior.plugins {
            settings["enabledPlugins"][format!("{name}@aylul-managed")] = false.into();
        }
        for item in &plugins {
            let key = format!("{}@aylul-managed", item.name);
            if !prior.plugins.contains(&item.name) && settings["enabledPlugins"].get(&key).is_some()
            {
                return Err(CoreError::ProfileInvalid(
                    "plugin setting is already owned".into(),
                ));
            }
            settings["enabledPlugins"][key] = true.into();
        }
        let current = Manifest {
            skills: skills.iter().map(|i| i.name.clone()).collect(),
            plugins: managed_plugins,
            generation: generation.clone(),
            items: skills
                .iter()
                .chain(&plugins)
                .map(|i| i.id.clone())
                .collect(),
        };
        let settings_bytes = serde_json::to_vec_pretty(&settings).map_err(|_| invalid())?;
        let manifest_bytes = serde_json::to_vec(&current).map_err(|_| invalid())?;
        let backup = stage.join("backup");
        std::fs::create_dir(&backup).map_err(|_| invalid())?;
        std::fs::create_dir_all(home.join("skills")).map_err(|_| invalid())?;
        let mut moved = Vec::new();
        let mut published = Vec::new();
        let commit = (|| {
            for name in &prior.skills {
                let target = home.join("skills").join(name);
                if target.exists() {
                    std::fs::rename(&target, backup.join(name)).map_err(|_| invalid())?;
                    moved.push(name.clone());
                }
            }
            for item in &skills {
                std::fs::rename(
                    stage.join("skills").join(&item.name),
                    home.join("skills").join(&item.name),
                )
                .map_err(|_| invalid())?;
                published.push(item.name.clone());
            }
            atomic(&settings_path, &settings_bytes)?;
            atomic(&manifest_path, &manifest_bytes)?;
            Ok(())
        })();
        if commit.is_err() {
            for name in published {
                let _ = std::fs::remove_dir_all(home.join("skills").join(name));
            }
            for name in moved {
                let _ = std::fs::rename(backup.join(&name), home.join("skills").join(name));
            }
            let _ = atomic(&settings_path, &prior_settings);
            return commit;
        }
        if !prior.generation.is_empty() {
            let _ = std::fs::remove_dir_all(
                home.join(".aylul-extension-generations")
                    .join(prior.generation),
            );
        }
        let _ = std::fs::remove_dir_all(backup);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(stage);
    }
    result
}
impl Homes {
    pub(super) async fn write_extensions(
        &self,
        extensions: &Extensions,
        root: &Path,
        home: &Path,
        slug: &str,
    ) -> Result<(), CoreError> {
        let _library = extensions.extension_guard().await;
        let _home = self.lock_home(home).await?;
        let items = extensions.agent_extensions(slug).await?;
        let skills = items
            .skills
            .into_iter()
            .filter(|i| i.enabled)
            .map(|i| i.item)
            .collect();
        let plugins = items
            .plugins
            .into_iter()
            .filter(|i| i.enabled)
            .map(|i| i.item)
            .collect();
        // No imported code is executed by projection.
        project(root, home, skills, plugins)
    }
}
