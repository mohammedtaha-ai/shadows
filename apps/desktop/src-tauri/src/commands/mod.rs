pub mod companion;
pub mod desktop;
pub mod preferences;
#[tauri::command]
pub fn hide_main(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("main surface required".into());
    }
    window.hide().map_err(|_| "unable to hide panel".into())
}

#[tauri::command]
pub async fn auth_action(
    window: tauri::WebviewWindow,
    auth: tauri::State<'_, std::sync::Arc<crate::auth::AuthController>>,
    action: String,
) -> Result<crate::auth::service::View, String> {
    if window.label() != "main" {
        return Err("main_required".into());
    }
    let auth = auth.inner().clone();
    tauri::async_runtime::spawn_blocking(move || auth.perform(&action).map_err(str::to_string))
        .await
        .map_err(|_| "action_failed".to_string())?
}
#[tauri::command]
pub fn cancel_login(
    window: tauri::WebviewWindow,
    auth: tauri::State<'_, std::sync::Arc<crate::auth::AuthController>>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("main_required".into());
    }
    auth.cancel();
    Ok(())
}

#[tauri::command]
pub async fn aylul_call(
    window: tauri::WebviewWindow,
    sidecar: tauri::State<'_, std::sync::Arc<crate::sidecar::SidecarState>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if window.label() != "main" {
        return Err("main_required".into());
    }
    let sidecar = sidecar.inner().clone();
    tauri::async_runtime::spawn_blocking(move || sidecar.call(&method, params))
        .await
        .map_err(|_| "action_failed".to_string())?
}
