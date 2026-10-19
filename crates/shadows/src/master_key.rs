//! The binary's only interface to the Windows credential store.
use shadows_core::SecretKey;

#[cfg(windows)]
pub fn load_or_create() -> anyhow::Result<SecretKey> {
    use rand::{RngCore, rngs::OsRng};
    let entry = keyring::Entry::new("shadows", "secrets-master-key")
        .map_err(|_| anyhow::anyhow!("cannot open the secrets master key entry"))?;
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
    Ok(SecretKey::from_bytes(key))
}

#[cfg(not(windows))]
pub fn load_or_create() -> anyhow::Result<SecretKey> {
    anyhow::bail!("the secrets master key requires Windows Credential Manager")
}
