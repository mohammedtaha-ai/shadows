use tauri_plugin_autostart::ManagerExt;

pub(crate) trait LoginItem: Send + Sync {
    fn is_enabled(&self) -> Result<bool, ()>;
    fn set_enabled(&self, enabled: bool) -> Result<(), ()>;
}
pub(crate) struct PluginLoginItem(pub(crate) tauri::AppHandle);
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
