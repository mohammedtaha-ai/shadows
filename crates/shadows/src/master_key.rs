//! The binary's only interface to the Windows credential store.
use shadows_core::SecretKey;

#[cfg(windows)]
pub fn load_or_create() -> anyhow::Result<SecretKey> {
    let directory = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| anyhow::anyhow!("cannot locate the secrets master key lock"))?
        .join("shadows");
    std::fs::create_dir_all(&directory)
        .map_err(|_| anyhow::anyhow!("cannot create the secrets master key lock directory"))?;
    let entry = keyring::Entry::new("shadows", "secrets-master-key")
        .map_err(|_| anyhow::anyhow!("cannot open the secrets master key entry"))?;
    Ok(SecretKey::from_bytes(read_or_create_entry(
        &entry,
        &directory.join("secrets-master-key.lock"),
    )?))
}

#[cfg(any(windows, test))]
fn read_or_create_entry(
    entry: &keyring::Entry,
    lock_path: &std::path::Path,
) -> anyhow::Result<[u8; 32]> {
    use rand::{RngCore, rngs::OsRng};
    // All daemons use the same per-user lock, regardless of database or profile.
    // The OS releases it on close/crash; its empty file must never be unlinked.
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|_| anyhow::anyhow!("cannot open the secrets master key lock"))?;
    lock.lock()
        .map_err(|_| anyhow::anyhow!("cannot acquire the secrets master key lock"))?;
    let bytes = match entry.get_secret() {
        Ok(bytes) => bytes,
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0; 32];
            OsRng
                .try_fill_bytes(&mut bytes)
                .map_err(|_| anyhow::anyhow!("cannot generate the secrets master key"))?;
            entry
                .set_secret(&bytes)
                .map_err(|_| anyhow::anyhow!("cannot save the secrets master key"))?;
            entry
                .get_secret()
                .map_err(|_| anyhow::anyhow!("cannot read back the secrets master key"))?
        }
        Err(_) => anyhow::bail!("cannot read the secrets master key"),
    };
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("the secrets master key must contain 32 bytes"))?;
    Ok(key)
}

#[cfg(not(windows))]
pub fn load_or_create() -> anyhow::Result<SecretKey> {
    anyhow::bail!("the secrets master key requires Windows Credential Manager")
}

#[cfg(test)]
#[path = "master_key_tests.rs"]
mod tests;
