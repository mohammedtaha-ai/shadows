use std::path::{Path, PathBuf};
use tauri::Manager;

/// Existing application storage, with an isolated debug acceptance override.
pub struct AppPaths {
    pub root: PathBuf,
    pub acceptance: bool,
}
impl AppPaths {
    pub fn resolve(app: &tauri::App) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        {
            let args: Vec<_> = std::env::args_os().collect();
            if let Some(index) = args.iter().position(|arg| arg == "--acceptance-root") {
                let root = args
                    .get(index + 1)
                    .map(PathBuf::from)
                    .ok_or("acceptance_root_invalid")?;
                if !root.is_absolute() || !root.is_dir() {
                    return Err("acceptance_root_invalid".into());
                }
                return Ok(Self {
                    root: root.canonicalize()?,
                    acceptance: true,
                });
            }
        }
        Ok(Self {
            root: app.path().app_data_dir()?,
            acceptance: false,
        })
    }
    pub fn webview(&self, label: &str) -> Option<PathBuf> {
        self.acceptance
            .then(|| self.root.join("webviews").join(label))
    }
    pub fn browser_args(&self, label: &str) -> Option<String> {
        #[cfg(debug_assertions)]
        if self.acceptance {
            let args: Vec<_> = std::env::args().collect();
            let port = args
                .iter()
                .position(|arg| arg == "--acceptance-debug-port")
                .and_then(|index| args.get(index + 1))
                .and_then(|value| value.parse::<u16>().ok())?;
            let port = port.checked_add(u16::from(label == "companion"))?;
            return (port > 0).then(|| format!("--remote-debugging-port={port}"));
        }
        let _ = label;
        None
    }
    pub fn auth(&self) -> PathBuf {
        self.root.join("auth-v1.dpapi")
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}
