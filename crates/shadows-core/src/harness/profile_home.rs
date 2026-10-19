//! Seeding isolated agent homes without replacing user files.
use crate::db::StorageError;
use std::io::Write;
use std::path::Path;

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
