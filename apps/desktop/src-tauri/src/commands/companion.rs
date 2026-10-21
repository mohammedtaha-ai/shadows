use crate::windows::companion::{self, state::CompanionState};
use tauri::Manager;
pub fn allowed_caller(label: &str, command: &str) -> bool {
    match command {
        "companion_state" => matches!(label, "main" | "companion"),
        "companion_send"
        | "companion_input"
        | "companion_ready"
        | "companion_failed"
        | "drag_companion"
        | "open_main_from_companion" => label == "companion",
        "set_companion_visible"
        | "set_companion_paused"
        | "auth_action"
        | "aylul_call"
        | "cancel_login"
        | "companion_binding"
        | "select_companion_agent"
        | "target_companion_thread"
        | "set_companion_previews"
        | "navigation_snapshot"
        | "desktop_preferences"
        | "set_desktop_preferences"
        | "refresh_companion_appearance"
        | "ack_navigation"
        | "hide_main" => label == "main",
        _ => false,
    }
}
fn guard(window: &tauri::WebviewWindow, name: &str) -> Result<(), String> {
    if !allowed_caller(window.label(), name) {
        return Err("surface_refused".into());
    }
    if window.label() == "companion" {
        let current = window
            .app_handle()
            .get_webview_window("companion")
            .ok_or("surface_refused")?;
        if window.hwnd().ok() != current.hwnd().ok() {
            return Err("surface_refused".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn companion_state(window: tauri::WebviewWindow) -> Result<CompanionState, String> {
    guard(&window, "companion_state")?;
    companion::on_ui(window.app_handle().clone(), |app| {
        Ok(companion::snapshot(app))
    })
    .await
}
#[tauri::command]
pub async fn companion_send(
    window: tauri::WebviewWindow,
    submission: crate::presence::send::Submission,
) -> Result<crate::presence::send::Sent, String> {
    guard(&window, "companion_send")?;
    let owner = window
        .app_handle()
        .state::<std::sync::Arc<crate::presence::Presence>>()
        .inner()
        .clone();
    tauri::async_runtime::spawn_blocking(move || owner.send(submission).map_err(str::to_owned))
        .await
        .map_err(|_| "companion_send_failed".to_string())?
}
#[tauri::command]
pub async fn companion_input(
    window: tauri::WebviewWindow,
    open: bool,
) -> Result<CompanionState, String> {
    guard(&window, "companion_input")?;
    companion::on_ui(window.app_handle().clone(), move |app| {
        companion::input::set_input(app, open)
    })
    .await
}
#[tauri::command]
pub async fn companion_ready(window: tauri::WebviewWindow) -> Result<CompanionState, String> {
    guard(&window, "companion_ready")?;
    companion::on_ui(window.app_handle().clone(), companion::ready).await
}
#[tauri::command]
pub async fn companion_failed(window: tauri::WebviewWindow) -> Result<CompanionState, String> {
    guard(&window, "companion_failed")?;
    companion::on_ui(window.app_handle().clone(), companion::failed).await
}
#[tauri::command]
pub async fn set_companion_visible(
    window: tauri::WebviewWindow,
    visible: bool,
) -> Result<CompanionState, String> {
    guard(&window, "set_companion_visible")?;
    companion::on_ui(window.app_handle().clone(), move |app| {
        companion::set_visible(app, visible)
    })
    .await
}
#[tauri::command]
pub async fn set_companion_paused(
    window: tauri::WebviewWindow,
    paused: bool,
) -> Result<CompanionState, String> {
    guard(&window, "set_companion_paused")?;
    companion::on_ui(window.app_handle().clone(), move |app| {
        companion::set_paused(app, paused)
    })
    .await
}
#[tauri::command]
pub async fn drag_companion(window: tauri::WebviewWindow) -> Result<(), String> {
    guard(&window, "drag_companion")?;
    companion::on_ui(window.app_handle().clone(), companion::drag).await
}
#[tauri::command]
pub async fn open_main_from_companion(window: tauri::WebviewWindow) -> Result<(), String> {
    guard(&window, "open_main_from_companion")?;
    let presence = window
        .app_handle()
        .state::<std::sync::Arc<crate::presence::Presence>>()
        .inner()
        .clone();
    let destination = tauri::async_runtime::spawn_blocking(move || presence.destination())
        .await
        .map_err(|_| "main_open_failed")?;
    companion::on_ui(window.app_handle().clone(), move |app| {
        use tauri::Emitter;
        let pending = app
            .state::<std::sync::Arc<crate::navigation::Navigation>>()
            .request(destination);
        let _ = app.emit_to("main", "main-navigation", pending);
        crate::windows::open_main(app).map_err(|_| "main_open_failed".into())
    })
    .await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companion_callers_are_restricted_for_every_operation() {
        for label in ["main", "companion", "unknown"] {
            for name in [
                "companion_send",
                "companion_input",
                "companion_ready",
                "companion_failed",
                "companion_state",
                "set_companion_visible",
                "set_companion_paused",
                "drag_companion",
                "open_main_from_companion",
                "auth_action",
                "aylul_call",
                "cancel_login",
                "hide_main",
                "companion_binding",
                "select_companion_agent",
                "target_companion_thread",
                "set_companion_previews",
                "navigation_snapshot",
                "ack_navigation",
                "refresh_companion_appearance",
                "desktop_preferences",
                "set_desktop_preferences",
            ] {
                let expected = match name {
                    "companion_state" => label == "main" || label == "companion",
                    "companion_send"
                    | "companion_input"
                    | "companion_ready"
                    | "companion_failed"
                    | "drag_companion"
                    | "open_main_from_companion" => label == "companion",
                    _ => label == "main",
                };
                assert_eq!(allowed_caller(label, name), expected, "{label} {name}");
            }
        }
        assert!(!allowed_caller("main", "unknown_operation"));
    }
}
