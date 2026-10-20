use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
};
pub type Failure = &'static str;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchConfig {
    #[serde(default)]
    pub configuration_id: String,
    pub executable: PathBuf,
    pub db: PathBuf,
    pub node: PathBuf,
    pub adapter: PathBuf,
    pub harness: PathBuf,
    pub aylul_config: PathBuf,
    pub origin: String,
}
impl LaunchConfig {
    pub fn socket(&self) -> Result<SocketAddr, Failure> {
        let raw = self
            .origin
            .strip_prefix("http://")
            .ok_or("core_origin_invalid")?;
        let socket: SocketAddr = raw.parse().map_err(|_| "core_origin_invalid")?;
        if !socket.ip().is_loopback() || socket.port() == 0 {
            return Err("core_origin_invalid");
        }
        Ok(socket)
    }
    pub fn normalized(mut self) -> Result<Self, Failure> {
        let socket = self.socket()?;
        for path in [
            &mut self.executable,
            &mut self.db,
            &mut self.node,
            &mut self.adapter,
            &mut self.harness,
            &mut self.aylul_config,
        ] {
            if !path.is_absolute() || !path.is_file() {
                return Err("core_path_invalid");
            }
            *path = fs::canonicalize(&*path).map_err(|_| "core_path_invalid")?;
        }
        self.origin = format!("http://{socket}");
        self.configuration_id.clear();
        let bytes = serde_json::to_vec(&self).map_err(|_| "core_config_invalid")?;
        if bytes.len() > 16_384 {
            return Err("core_config_invalid");
        }
        self.configuration_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, &bytes).to_string();
        Ok(self)
    }
}
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, Failure> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("core_config_unavailable"),
    };
    let mut bytes = Vec::new();
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| "core_config_unavailable")?;
    if bytes.len() > 16_384 {
        return Err("core_config_invalid");
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "core_config_invalid")
}
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Failure> {
    fs::create_dir_all(path.parent().ok_or("core_config_invalid")?)
        .map_err(|_| "core_config_unavailable")?;
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "core_config_unavailable")?;
        file.write_all(&serde_json::to_vec(value).map_err(|_| "core_config_invalid")?)
            .map_err(|_| "core_config_unavailable")?;
        file.sync_all().map_err(|_| "core_config_unavailable")?;
        fs::rename(&temporary, path).map_err(|_| "core_config_unavailable")
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
