use crate::windows::preferences::DesktopPreferences;
fn main_only(window: &tauri::WebviewWindow) -> Result<(), String> {
    if super::companion::allowed_caller(window.label(), "desktop_preferences") {
        Ok(())
    } else {
        Err("main_required".into())
    }
}
#[tauri::command]
pub async fn desktop_preferences(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    desktop: tauri::State<'_, std::sync::Arc<crate::app::DesktopApp>>,
) -> Result<DesktopPreferences, String> {
    main_only(&window)?;
    let desktop = desktop.inner().clone();
    let store = desktop.preferences().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store.read(&desktop.login_item(app)).map_err(str::to_owned)
    })
    .await
    .map_err(|_| "desktop_preferences_load_failed".to_owned())?
}
#[tauri::command(rename_all = "camelCase")]
pub async fn set_desktop_preferences(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    desktop: tauri::State<'_, std::sync::Arc<crate::app::DesktopApp>>,
    launch_at_login: Option<bool>,
    close_to_tray: Option<bool>,
) -> Result<DesktopPreferences, String> {
    main_only(&window)?;
    let desktop = desktop.inner().clone();
    let store = desktop.preferences().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .set(&desktop.login_item(app), launch_at_login, close_to_tray)
            .map_err(str::to_owned)
    })
    .await
    .map_err(|_| "desktop_preferences_save_failed".to_owned())?
}
