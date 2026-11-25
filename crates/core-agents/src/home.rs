//! An agent's isolated home: seeding it, and what its `CLAUDE.md` says.
use super::{Agents, AgentsError};
use core_db::StorageError;
use std::io::Write;
use std::path::Path;

/// The bundled instructions an agent without its own text follows.
pub(super) const TEMPLATE: &str = include_str!("../assets/aylul-profile/CLAUDE.md");

impl Agents {
    /// Writes the default `settings.json` when the home does not already hold one. The file
    /// appears whole or not at all: it is written to a private temporary name, then linked
    /// in place, which fails rather than replace a file already there (the person's own).
    pub(super) fn seed_home(home: &Path) -> Result<(), StorageError> {
        let unwritable = || StorageError::Unavailable("agent home write".into());
        std::fs::create_dir_all(home)
            .map_err(|_| StorageError::Unavailable("agent home".into()))?;
        let target = home.join("settings.json");
        if target.exists() {
            return Ok(());
        }
        let settings = include_str!("../assets/aylul-profile/settings.json");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let temp = home.join(format!(".settings-{}-{nanos}.json", std::process::id()));
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .and_then(|mut file| {
                file.write_all(settings.as_bytes())?;
                file.sync_all()
            });
        let linked = match written {
            Ok(()) => std::fs::hard_link(&temp, &target),
            Err(e) => Err(e),
        };
        let _ = std::fs::remove_file(&temp);
        match linked {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && target.exists() => Ok(()),
            // A filesystem without hard links (FAT, some shares): create the file in place,
            // still never over one already there.
            Err(_) => match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
            {
                Ok(mut file) => file
                    .write_all(settings.as_bytes())
                    .map_err(|_| unwritable()),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
                Err(_) => Err(unwritable()),
            },
        }
    }

    /// The bundled instructions a new agent's form starts from.
    pub fn instructions_template(&self) -> &'static str {
        TEMPLATE
    }

    /// What the agent's home `CLAUDE.md` says: its own text, the template for an agent
    /// that follows it, and `None` before the home file was imported.
    pub async fn home_instructions(&self, slug: &str) -> Result<Option<String>, AgentsError> {
        let stored = self.store.instructions(slug).await?;
        let Some(text) = stored else {
            return Ok(None);
        };
        let agent = self.agent(slug).await?;
        let mut identity = format!("Your name is {}.\n", agent.name);
        if !agent.description.is_empty() {
            identity.push_str(&format!("Your role: {}.\n", agent.description));
        }
        identity.push('\n');
        identity.push_str(if text.is_empty() { TEMPLATE } else { &text });
        Ok(Some(identity))
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
