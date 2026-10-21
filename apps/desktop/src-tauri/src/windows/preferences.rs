//! This machine's saved window-close policy and OS login-item coordination.
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri_plugin_autostart::ManagerExt;
const LOAD: &str = "desktop_preferences_load_failed";
const SAVE: &str = "desktop_preferences_save_failed";
const AUTOSTART: &str = "desktop_preferences_autostart_failed";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopPreferences {
    pub launch_at_login: bool,
    pub close_to_tray: bool,
}
impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            close_to_tray: true,
        }
    }
}
pub trait LoginItem {
    fn is_enabled(&self) -> Result<bool, ()>;
    fn set_enabled(&self, enabled: bool) -> Result<(), ()>;
}
pub struct PluginLoginItem(pub tauri::AppHandle);
impl LoginItem for PluginLoginItem {
    fn is_enabled(&self) -> Result<bool, ()> {
        self.0.autolaunch().is_enabled().map_err(|_| ())
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), ()> {
        let login = self.0.autolaunch();
        if enabled {
            login.enable()
        } else {
            login.disable()
        }
        .map_err(|_| ())
    }
}
struct Model {
    saved: DesktopPreferences,
    error: bool,
}
#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    model: Arc<Mutex<Model>>,
}
impl Store {
    pub fn new(root: PathBuf) -> Self {
        let path = root.join("desktop-preferences-v1.json");
        let saved = Self::load(&path);
        Self {
            path,
            model: Arc::new(Mutex::new(Model {
                error: saved.is_err(),
                saved: saved.unwrap_or_default(),
            })),
        }
    }
    fn load(path: &std::path::Path) -> Result<DesktopPreferences, &'static str> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| LOAD),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(DesktopPreferences::default())
            }
            Err(_) => Err(LOAD),
        }
    }
    fn recover(&self, model: &mut Model) -> Result<(), &'static str> {
        if model.error {
            model.saved = Self::load(&self.path)?;
            model.error = false;
        }
        Ok(())
    }
    pub fn close_to_tray(&self) -> bool {
        self.model.lock().unwrap().saved.close_to_tray
    }
    pub fn read(&self, login: &impl LoginItem) -> Result<DesktopPreferences, &'static str> {
        let mut model = self.model.lock().unwrap();
        self.recover(&mut model)?;
        Ok(DesktopPreferences {
            launch_at_login: login.is_enabled().map_err(|_| AUTOSTART)?,
            close_to_tray: model.saved.close_to_tray,
        })
    }
    pub fn set(
        &self,
        login: &impl LoginItem,
        launch_at_login: Option<bool>,
        close_to_tray: Option<bool>,
    ) -> Result<DesktopPreferences, &'static str> {
        let mut model = self.model.lock().unwrap();
        self.recover(&mut model)?;
        let before = login.is_enabled().map_err(|_| AUTOSTART)?;
        let view = DesktopPreferences {
            launch_at_login: launch_at_login.unwrap_or(before),
            close_to_tray: close_to_tray.unwrap_or(model.saved.close_to_tray),
        };
        if launch_at_login.is_none() && close_to_tray.is_none() {
            return Ok(view);
        }
        let changed = view.launch_at_login != before;
        if changed {
            login
                .set_enabled(view.launch_at_login)
                .map_err(|_| AUTOSTART)?;
            if login.is_enabled() != Ok(view.launch_at_login) {
                let _ = login.set_enabled(before);
                return Err(AUTOSTART);
            }
        }
        if let Err(error) = self.save(&view) {
            if changed {
                let _ = login.set_enabled(before);
                let _ = login.is_enabled();
            }
            return Err(error);
        }
        model.saved = view.clone();
        Ok(view)
    }
    fn save(&self, view: &DesktopPreferences) -> Result<(), &'static str> {
        let root = self.path.parent().ok_or(SAVE)?;
        std::fs::create_dir_all(root).map_err(|_| SAVE)?;
        let temporary = root.join(format!("desktop-preferences-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| SAVE)?;
            file.write_all(&serde_json::to_vec(view).map_err(|_| SAVE)?)
                .map_err(|_| SAVE)?;
            file.sync_all().map_err(|_| SAVE)?;
            drop(file);
            std::fs::rename(&temporary, &self.path).map_err(|_| SAVE)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}
#[derive(Debug, PartialEq)]
pub enum CloseAction {
    Hide,
    Exit,
}
pub fn close_action(close_to_tray: bool, tray_quit: bool) -> CloseAction {
    if tray_quit || !close_to_tray {
        CloseAction::Exit
    } else {
        CloseAction::Hide
    }
}
#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
