use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub bind: SocketAddr,
    /// Spec §1.4: resolved from configuration, never from `PATH`. This machine
    /// carries more than one `claude-code` installation at different versions.
    /// Built through [`harness_path`], which is what enforces that.
    pub harness_path: PathBuf,
    /// Set in debug mode: the file this run's log lines also go to, which
    /// `serve` prints after its address. Spec §8.7.
    pub debug_log: Option<PathBuf>,
    /// Spec §1: the browser origins allowed to call this daemon cross-origin,
    /// each built through [`allowed_origin`]. Every client is on another
    /// origin — the daemon serves no page of its own.
    pub allowed_origins: Vec<String>,
}

/// Vite's dev server, under both names a browser may use for it. What
/// `--allow-origin` means when it is not given.
pub const DEFAULT_ALLOWED_ORIGINS: [&str; 2] = ["http://localhost:5173", "http://127.0.0.1:5173"];

/// The daemon's data directory: the one holding its database. Debug mode's
/// `logs/` directory lives here, beside the data it explains.
pub fn data_dir(db_path: &Path) -> PathBuf {
    match db_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(
        "the harness path must be absolute, got `{0}`. Spec §1.4: a harness \
         executable is resolved from explicit configuration, never from PATH. A \
         bare program name is a PATH lookup by another spelling, and a relative \
         path resolves against whatever directory the daemon happens to be in."
    )]
    HarnessNotAbsolute(String),
    #[error(
        "`{0}` is not an origin. An origin is exactly what a browser sends in its \
         Origin header: `http://` or `https://`, a host, an optional port, and \
         nothing after it — no path, not even a trailing slash."
    )]
    OriginInvalid(String),
}

/// The only way an allowed origin should be obtained from user input. A
/// browser compares its `Origin` header byte for byte, so an entry with a
/// trailing slash or a path would be accepted here and then match nothing,
/// silently; it is refused at startup instead.
pub fn allowed_origin(raw: &str) -> Result<String, ConfigError> {
    let invalid = || ConfigError::OriginInvalid(raw.to_string());
    let authority = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
        .ok_or_else(invalid)?;
    let usable = !authority.is_empty()
        && authority.is_ascii()
        && !authority.contains(['/', '?', '#', '@', ' '])
        && !authority.starts_with(':');
    if usable {
        Ok(raw.to_string())
    } else {
        Err(invalid())
    }
}

/// The only way a `harness_path` should be obtained from user input.
///
/// Spec §1.4 is not a defensive habit: the measured stream contract belongs to
/// one installation at one version, this machine carries several, and an
/// auto-update can change the contract underneath a running install. An
/// Operation records the path and version it ran, so a path that means
/// "whatever PATH resolved at spawn time" makes that record say nothing.
///
/// Requiring *absolute* rather than merely non-bare is deliberate. A relative
/// path is resolved against the process working directory, which for a daemon is
/// not a stable fact, so it carries the same ambiguity one step further in.
pub fn harness_path(raw: &Path) -> Result<PathBuf, ConfigError> {
    if raw.is_absolute() {
        Ok(raw.to_path_buf())
    } else {
        Err(ConfigError::HarnessNotAbsolute(
            raw.to_string_lossy().into_owned(),
        ))
    }
}
