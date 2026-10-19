//! Seeding isolated agent homes without replacing user files.
use crate::CoreError;
use crate::db::StorageError;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) fn seed(home: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(home).map_err(|_| StorageError::Unavailable("agent home".into()))?;
    for (name, content) in [
        (
            "CLAUDE.md",
            include_str!("../../assets/aylul-profile/CLAUDE.md"),
        ),
        (
            "settings.json",
            include_str!("../../assets/aylul-profile/settings.json"),
        ),
    ] {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(home.join(name))
        {
            Ok(mut file) => file
                .write_all(content.as_bytes())
                .map_err(|_| StorageError::Unavailable("agent home write".into()))?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(StorageError::Unavailable("agent home write".into())),
        }
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct Homes {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

fn check(home: &Path) -> Result<serde_json::Value, CoreError> {
    let bytes = std::fs::read(home.join("settings.json"))
        .map_err(|_| CoreError::ProfileInvalid("cannot read settings.json".into()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| CoreError::ProfileInvalid("invalid settings.json".into()))?;
    let object = value
        .as_object()
        .ok_or_else(|| CoreError::ProfileInvalid("settings.json must be an object".into()))?;
    if object.contains_key("modelOverrides") {
        return Err(CoreError::ProfileInvalid(
            "settings.json modelOverrides is refused".into(),
        ));
    }
    if let Some(env) = object.get("env") {
        let env = env.as_object().ok_or_else(|| {
            CoreError::ProfileInvalid("settings.json env must be an object".into())
        })?;
        for name in env.keys() {
            if crate::secrets::reserved_env_name(name) {
                return Err(CoreError::SecretNameReserved(name.clone()));
            }
        }
    }
    Ok(value)
}

impl Homes {
    pub(super) async fn write_models(
        &self,
        home: &Path,
        models: &[String],
        default: &str,
    ) -> Result<(), CoreError> {
        let home = dunce::canonicalize(home)
            .map_err(|_| CoreError::ProfileInvalid("agent home is unavailable".into()))?;
        let lock = self
            .locks
            .lock()
            .await
            .entry(home.clone())
            .or_default()
            .clone();
        let _held = lock.lock().await;
        let mut value = check(&home)?;
        let models = serde_json::json!(models);
        if value.get("availableModels") == Some(&models)
            && value.get("model").and_then(|v| v.as_str()) == Some(default)
        {
            return Ok(());
        }
        value["availableModels"] = models;
        value["model"] = default.into();
        let temp = home.join(format!(".settings-{}.tmp", uuid::Uuid::new_v4()));
        let write = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&serde_json::to_vec_pretty(&value).expect("JSON value serializes"))?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, home.join("settings.json"))
        })();
        if write.is_err() {
            let _ = std::fs::remove_file(&temp);
            return Err(CoreError::ProfileInvalid(
                "cannot write settings.json".into(),
            ));
        }
        Ok(())
    }
}
