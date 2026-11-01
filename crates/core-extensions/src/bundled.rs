//! Import embedded items through the library's existing transactional copy path.
use crate::{ExtensionKind, Extensions, ExtensionsError};
use core_db::CommandContext;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;

include!(concat!(env!("OUT_DIR"), "/bundled.rs"));

impl Extensions {
    pub async fn import_bundled(&self) -> Result<(), ExtensionsError> {
        let Some(home) = &self.home_root else {
            return Ok(());
        };
        let _guard = self.extension_guard().await;
        let mut items = BTreeMap::<(&str, &str), Vec<(&str, &[u8])>>::new();
        for &(path, bytes) in FILES {
            let mut parts = path.splitn(3, '/');
            let kind = parts.next().expect("bundled kind");
            let name = parts.next().expect("bundled name");
            let relative = parts.next().expect("bundled file");
            items
                .entry((kind, name))
                .or_default()
                .push((relative, bytes));
        }
        for ((kind, name), files) in items {
            let kind = match kind {
                "skills" => ExtensionKind::Skill,
                "plugins" => ExtensionKind::Plugin,
                _ => return Err(ExtensionsError::Invalid("invalid bundled kind".into())),
            };
            if self.store.bundled_present_or_declined(kind, name).await? {
                continue;
            }
            let mut hash = Sha256::new();
            for (path, bytes) in &files {
                hash.update((path.len() as u64).to_le_bytes());
                hash.update(path.as_bytes());
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            }
            let digest = format!("{:x}", hash.finalize());
            let home = dunce::canonicalize(home)
                .map_err(|_| ExtensionsError::Invalid("bundled staging unavailable".into()))?;
            let stage = tempfile::Builder::new()
                .prefix(".bundled-")
                .tempdir_in(&home)
                .map_err(|_| ExtensionsError::Invalid("bundled staging unavailable".into()))?;
            let source = stage.path().to_path_buf();
            for (path, bytes) in files {
                let target = source.join(path);
                std::fs::create_dir_all(target.parent().expect("bundled parent"))
                    .map_err(|_| ExtensionsError::Invalid("bundled staging unavailable".into()))?;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target)
                    .map_err(|_| ExtensionsError::Invalid("bundled staging unavailable".into()))?;
                file.write_all(bytes)
                    .map_err(|_| ExtensionsError::Invalid("bundled staging unavailable".into()))?;
            }
            let command = CommandContext::user(
                format!("bundled:{}:{name}:{digest}", kind.as_str()),
                "extension.add",
                serde_json::json!({"kind":kind,"name":name,"digest":digest}),
            );
            self.store
                .add(command, kind, source, home.join("library"), true)
                .await?;
        }
        Ok(())
    }
}
