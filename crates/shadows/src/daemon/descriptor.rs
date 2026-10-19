//! Exclusive ownership of a private local connection descriptor.
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub origin: String,
    pub installation_id: String,
    pub token: String,
}

pub fn claim(path: &Path) -> anyhow::Result<File> {
    if !path.is_absolute() {
        anyhow::bail!("descriptor path must be absolute");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid descriptor directory"))?;
    std::fs::create_dir_all(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path.with_extension("owner"))?;
    file.try_lock()
        .map_err(|_| anyhow::anyhow!("product daemon already owns this descriptor"))?;
    Ok(file)
}

pub fn write(path: &Path, descriptor: &Descriptor) -> anyhow::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&serde_json::to_vec(descriptor)?)?;
    file.sync_all()?;
    Ok(())
}
