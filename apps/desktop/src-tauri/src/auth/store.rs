use super::wire;
use crate::native_login;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// This type is native-only. Never Debug, IPC or an unencrypted disk artifact.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub handle: Option<String>,
    pub enrollment: Option<String>,
    pub account: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    pub origin: String,
}
pub struct Store {
    path: PathBuf,
    origin: String,
}
impl Store {
    pub fn new(path: PathBuf, origin: String) -> Self {
        Self { path, origin }
    }
    pub fn load(&self) -> Result<Credentials, &'static str> {
        let Some(bytes) = native_login::load_session(&self.path).map_err(|_| "storage_invalid")?
        else {
            return Ok(Credentials {
                origin: self.origin.clone(),
                ..Default::default()
            });
        };
        let c: Credentials = serde_json::from_slice(&bytes).map_err(|_| "storage_invalid")?;
        if c.origin != self.origin
            || c.handle.as_deref().is_some_and(|v| !wire::opaque(v))
            || c.enrollment.as_deref().is_some_and(|v| !wire::opaque(v))
        {
            return Err("storage_invalid");
        }
        Ok(c)
    }
    pub fn save(&self, c: &Credentials) -> Result<(), &'static str> {
        let bytes = serde_json::to_vec(c).map_err(|_| "storage_invalid")?;
        native_login::save_session(&self.path, &bytes).map_err(|_| "storage_unavailable")
    }
}
