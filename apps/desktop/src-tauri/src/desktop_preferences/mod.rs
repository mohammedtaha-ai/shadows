//! This machine's saved window-close policy and OS login-item coordination.
mod login_item;
mod store;

use login_item::LoginItem;
pub(crate) use login_item::PluginLoginItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferencesView {
    pub launch_at_login: bool,
    pub close_to_tray: bool,
}
impl Default for PreferencesView {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            close_to_tray: true,
        }
    }
}
pub struct DesktopPreferences {
    store: store::Store,
    login: Box<dyn LoginItem>,
}

#[derive(Debug, PartialEq)]
pub enum CloseAction {
    Hide,
    Exit,
}
impl DesktopPreferences {
    pub(crate) fn new(root: PathBuf, login: impl LoginItem + 'static) -> Self {
        Self {
            store: store::Store::new(root),
            login: Box::new(login),
        }
    }

    pub fn read(&self) -> Result<PreferencesView, &'static str> {
        self.store.read(&*self.login)
    }

    pub fn set(
        &self,
        launch_at_login: Option<bool>,
        close_to_tray: Option<bool>,
    ) -> Result<PreferencesView, &'static str> {
        self.store.set(&*self.login, launch_at_login, close_to_tray)
    }

    pub fn close_action(&self, tray_quit: bool) -> CloseAction {
        if tray_quit || !self.store.close_to_tray() {
            CloseAction::Exit
        } else {
            CloseAction::Hide
        }
    }
}

#[cfg(test)]
mod tests;
