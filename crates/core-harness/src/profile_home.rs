//! Serialized writes to an agent home's settings at opening.
use crate::HarnessError;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub(super) struct Homes {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl Homes {
    /// The home's settings, refusing what would override the routing Core sets.
    pub(super) fn check(home: &Path) -> Result<serde_json::Value, HarnessError> {
        let bytes = std::fs::read(home.join("settings.json"))
            .map_err(|_| HarnessError::ProfileInvalid("cannot read settings.json".into()))?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| HarnessError::ProfileInvalid("invalid settings.json".into()))?;
        let object = value.as_object().ok_or_else(|| {
            HarnessError::ProfileInvalid("settings.json must be an object".into())
        })?;
        if object.contains_key("modelOverrides") {
            return Err(HarnessError::ProfileInvalid(
                "settings.json modelOverrides is refused".into(),
            ));
        }
        if let Some(env) = object.get("env") {
            let env = env.as_object().ok_or_else(|| {
                HarnessError::ProfileInvalid("settings.json env must be an object".into())
            })?;
            for name in env.keys() {
                if core_secrets::Secrets::reserved_env_name(name) {
                    return Err(HarnessError::SecretNameReserved(name.clone()));
                }
            }
        }
        Ok(value)
    }

    pub(super) async fn lock_home(
        &self,
        home: &Path,
    ) -> Result<tokio::sync::OwnedMutexGuard<()>, HarnessError> {
        let home = dunce::canonicalize(home)
            .map_err(|_| HarnessError::ProfileInvalid("agent home is unavailable".into()))?;
        let lock = self.locks.lock().await.entry(home).or_default().clone();
        Ok(lock.lock_owned().await)
    }
    pub(super) async fn write_models(
        &self,
        home: &Path,
        models: &[String],
        default: &str,
    ) -> Result<(), HarnessError> {
        let home = dunce::canonicalize(home)
            .map_err(|_| HarnessError::ProfileInvalid("agent home is unavailable".into()))?;
        let _held = self.lock_home(&home).await?;
        let mut value = Self::check(&home)?;
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
            return Err(HarnessError::ProfileInvalid(
                "cannot write settings.json".into(),
            ));
        }
        Ok(())
    }
}
