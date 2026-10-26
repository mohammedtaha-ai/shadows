use super::{login_item::LoginItem, PreferencesView};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
const LOAD: &str = "desktop_preferences_load_failed";
const SAVE: &str = "desktop_preferences_save_failed";
const AUTOSTART: &str = "desktop_preferences_autostart_failed";

struct Model {
    saved: PreferencesView,
    error: bool,
}
#[derive(Clone)]
pub(super) struct Store {
    path: PathBuf,
    model: Arc<Mutex<Model>>,
}
impl Store {
    pub(super) fn new(root: PathBuf) -> Self {
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
    fn load(path: &std::path::Path) -> Result<PreferencesView, &'static str> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| LOAD),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(PreferencesView::default())
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
    pub(super) fn close_to_tray(&self) -> bool {
        self.model.lock().unwrap().saved.close_to_tray
    }
    pub(super) fn read(
        &self,
        login: &(impl LoginItem + ?Sized),
    ) -> Result<PreferencesView, &'static str> {
        let mut model = self.model.lock().unwrap();
        self.recover(&mut model)?;
        Ok(PreferencesView {
            launch_at_login: login.is_enabled().map_err(|_| AUTOSTART)?,
            close_to_tray: model.saved.close_to_tray,
        })
    }
    pub(super) fn set(
        &self,
        login: &(impl LoginItem + ?Sized),
        launch_at_login: Option<bool>,
        close_to_tray: Option<bool>,
    ) -> Result<PreferencesView, &'static str> {
        let mut model = self.model.lock().unwrap();
        self.recover(&mut model)?;
        let before = login.is_enabled().map_err(|_| AUTOSTART)?;
        let view = PreferencesView {
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
    fn save(&self, view: &PreferencesView) -> Result<(), &'static str> {
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
