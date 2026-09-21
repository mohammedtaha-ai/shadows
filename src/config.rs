use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub bind: SocketAddr,
    /// Spec §1.4: resolved from configuration, never from `PATH`. This machine
    /// carries more than one `claude-code` installation at different versions.
    pub harness_path: PathBuf,
}
