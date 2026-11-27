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
    /// Called while the opening holds the home lock and the library snapshot.
    pub(super) fn write_instructions(
        &self,
        home: &Path,
        instructions: &str,
    ) -> Result<(), HarnessError> {
        Self::replace_file(&home.join("CLAUDE.md"), instructions.as_bytes())
            .map_err(|_| HarnessError::ProfileInvalid("cannot write CLAUDE.md".into()))
    }

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
        let bytes = serde_json::to_vec_pretty(&value).expect("JSON value serializes");
        Self::replace_file(&home.join("settings.json"), &bytes)
            .map_err(|_| HarnessError::ProfileInvalid("cannot write settings.json".into()))
    }

    /// The one way a home file is replaced: written whole under a private temporary name,
    /// then renamed over the target, so a reader sees the old file or the new one.
    pub(super) fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().trim_start_matches('.').to_owned())
            .unwrap_or_default();
        let temp = path.with_file_name(format!(".{stem}-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            Self::rename_over(&temp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }

    /// On Windows another process (an antivirus scan of the file just written, an
    /// indexer) may hold either file without sharing delete access, and the rename is
    /// then denied until it lets go; such holds were measured at up to about a second
    /// under load, so the rename is retried for up to five.
    fn rename_over(from: &Path, to: &Path) -> std::io::Result<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match std::fs::rename(from, to) {
                Err(e)
                    if cfg!(windows)
                        && matches!(e.raw_os_error(), Some(5 | 32))
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                result => return result,
            }
        }
    }
}
