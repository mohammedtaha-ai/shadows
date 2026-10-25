use crate::desktop_preferences::PreferencesView;
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
    desktop: tauri::State<'_, std::sync::Arc<crate::app::DesktopApp>>,
) -> Result<PreferencesView, String> {
    main_only(&window)?;
    let desktop = desktop.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        desktop.preferences().read().map_err(str::to_owned)
    })
    .await
    .map_err(|_| "desktop_preferences_load_failed".to_owned())?
}
#[tauri::command(rename_all = "camelCase")]
pub async fn set_desktop_preferences(
    window: tauri::WebviewWindow,
    desktop: tauri::State<'_, std::sync::Arc<crate::app::DesktopApp>>,
    launch_at_login: Option<bool>,
    close_to_tray: Option<bool>,
) -> Result<PreferencesView, String> {
    main_only(&window)?;
    let desktop = desktop.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        desktop
            .preferences()
            .set(launch_at_login, close_to_tray)
            .map_err(str::to_owned)
    })
    .await
    .map_err(|_| "desktop_preferences_save_failed".to_owned())?
}
