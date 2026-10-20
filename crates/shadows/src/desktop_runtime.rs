//! Identity and private receipt for an explicitly configured desktop launch.
use serde::{Deserialize, Serialize};
use shadows_http::RuntimeIdentity;
use shadows_process::ProcessIdentity;
use std::{
    fs::File,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub const NATIVE_ORIGIN: &str = "http://tauri.localhost";

#[derive(Clone, Debug)]
pub struct DesktopLaunch {
    pub record_path: PathBuf,
    pub configuration_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeReceipt {
    #[serde(flatten)]
    pub identity: RuntimeIdentity,
    pub origin: String,
    pub database: PathBuf,
    pub process: ProcessIdentity,
}

pub struct DesktopRuntime {
    _owner: File,
    path: PathBuf,
    identity: RuntimeIdentity,
}

impl DesktopRuntime {
    pub fn claim(launch: &DesktopLaunch, bind: SocketAddr) -> anyhow::Result<Self> {
        anyhow::ensure!(bind.ip().is_loopback(), "desktop runtime requires loopback");
        uuid::Uuid::parse_str(&launch.configuration_id)?;
        let owner = crate::runtime_record::claim(&launch.record_path)?;
        Ok(Self {
            _owner: owner,
            path: launch.record_path.clone(),
            identity: RuntimeIdentity {
                runtime_id: uuid::Uuid::new_v4().to_string(),
                configuration_id: launch.configuration_id.clone(),
            },
        })
    }
    pub fn identity(&self) -> RuntimeIdentity {
        self.identity.clone()
    }

    pub fn allowed_origins(mut origins: Vec<String>) -> Vec<String> {
        if !origins.iter().any(|origin| origin == NATIVE_ORIGIN) {
            origins.push(NATIVE_ORIGIN.into());
        }
        origins
    }

    /// Called only after the listener, Core and assistant project are ready.
    pub fn publish(&self, address: SocketAddr, database: &Path) -> anyhow::Result<()> {
        let process = shadows_process::inspect_process(std::process::id())?
            .ok_or_else(|| anyhow::anyhow!("runtime process identity unavailable"))?;
        crate::runtime_record::write(
            &self.path,
            &RuntimeReceipt {
                identity: self.identity(),
                origin: format!("http://{address}"),
                database: std::fs::canonicalize(database)?,
                process,
            },
        )
    }
}
impl Drop for DesktopRuntime {
    fn drop(&mut self) {
        let Ok(file) = File::open(&self.path) else {
            return;
        };
        let mut bytes = Vec::new();
        if file.take(16_385).read_to_end(&mut bytes).is_err() || bytes.len() > 16_384 {
            return;
        }
        if let Ok(receipt) = serde_json::from_slice::<RuntimeReceipt>(&bytes)
            && receipt.identity.runtime_id == self.identity.runtime_id
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
