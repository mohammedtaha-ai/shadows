//! An agent's isolated home: seeding it, and what its `CLAUDE.md` says.
use super::{Agents, AgentsError};
use core_db::StorageError;
use std::io::Write;
use std::path::Path;

/// The bundled instructions an agent without its own text follows.
pub(super) const TEMPLATE: &str = include_str!("../assets/aylul-profile/CLAUDE.md");

impl Agents {
    /// Writes the default `settings.json` when the home does not already hold one.
    pub(super) fn seed_home(home: &Path) -> Result<(), StorageError> {
        std::fs::create_dir_all(home)
            .map_err(|_| StorageError::Unavailable("agent home".into()))?;
        let settings = include_str!("../assets/aylul-profile/settings.json");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(home.join("settings.json"))
        {
            Ok(mut file) => file
                .write_all(settings.as_bytes())
                .map_err(|_| StorageError::Unavailable("agent home write".into()))?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(StorageError::Unavailable("agent home write".into())),
        }
        Ok(())
    }

    /// The bundled instructions a new agent's form starts from.
    pub fn instructions_template(&self) -> &'static str {
        TEMPLATE
    }

    /// What the agent's home `CLAUDE.md` says: its own text, the template for an agent
    /// that follows it, and `None` before the home file was imported.
    pub async fn home_instructions(&self, slug: &str) -> Result<Option<String>, AgentsError> {
        let stored = self.store.instructions(slug).await?;
        Ok(stored.map(|text| {
            if text.is_empty() {
                TEMPLATE.to_owned()
            } else {
                text
            }
        }))
    }

    /// Reads each live agent's never-imported `<home>/CLAUDE.md` once and stores what
    /// it says. Returns the slugs whose file could not be read as text, which stay
    /// unimported.
    pub async fn import_home_instructions(&self) -> Result<Vec<String>, AgentsError> {
        let mut skipped = Vec::new();
        if self.home_root.is_none() {
            return Ok(skipped);
        }
        for slug in self.store.unimported().await? {
            let text = match std::fs::read(self.home(&slug)?.join("CLAUDE.md")) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) if !text.contains('\0') => text,
                    _ => {
                        skipped.push(slug);
                        continue;
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(_) => {
                    skipped.push(slug);
                    continue;
                }
            };
            let text = if text == TEMPLATE {
                String::new()
            } else {
                text
            };
            self.store.import_instructions(slug, text).await?;
        }
        Ok(skipped)
    }
}
