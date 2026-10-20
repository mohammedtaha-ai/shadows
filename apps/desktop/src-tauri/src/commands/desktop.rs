use crate::core_connection::{config::LaunchConfig, Connection, ConnectionView};
use std::sync::Arc;
use tauri::Manager;

fn main_only(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("main_required".into())
    }
}
#[tauri::command]
pub fn desktop_connection(
    window: tauri::WebviewWindow,
    connection: tauri::State<'_, Arc<Connection>>,
) -> Result<ConnectionView, String> {
    main_only(&window)?;
    Ok(connection.snapshot())
}
#[tauri::command]
pub fn desktop_connection_config(
    window: tauri::WebviewWindow,
    connection: tauri::State<'_, Arc<Connection>>,
) -> Result<Option<LaunchConfig>, String> {
    main_only(&window)?;
    Ok(connection.configured())
}
#[tauri::command]
pub fn save_desktop_connection(
    window: tauri::WebviewWindow,
    connection: tauri::State<'_, Arc<Connection>>,
    config: LaunchConfig,
) -> Result<bool, String> {
    main_only(&window)?;
    let changed = connection.save(config).map_err(str::to_owned)?;
    if changed {
        let connection = connection.inner().clone();
        tauri::async_runtime::spawn_blocking(move || connection.retry());
    }
    Ok(changed)
}
#[tauri::command]
pub async fn retry_desktop_connection(
    window: tauri::WebviewWindow,
    connection: tauri::State<'_, Arc<Connection>>,
) -> Result<ConnectionView, String> {
    main_only(&window)?;
    let connection = connection.inner().clone();
    tauri::async_runtime::spawn_blocking(move || connection.retry())
        .await
        .map_err(|_| "core_unavailable".into())
}
#[tauri::command]
pub fn open_external(window: tauri::WebviewWindow, url: String) -> Result<(), String> {
    main_only(&window)?;
    let parsed = url::Url::parse(&url).map_err(|_| "external_url_invalid")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || url.len() > 4096
    {
        return Err("external_url_invalid".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let text: Vec<u16> = std::ffi::OsStr::new(parsed.as_str())
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: the UTF-16 URL is terminated and remains alive through this call.
        let result = unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                text.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result as isize <= 32 {
            return Err("external_open_failed".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("external_open_unavailable".into())
}

#[tauri::command]
pub fn companion_binding(
    window: tauri::WebviewWindow,
    presence: tauri::State<'_, Arc<crate::presence::Presence>>,
) -> Result<crate::presence::BindingView, String> {
    main_only(&window)?;
    presence.snapshot().map_err(str::to_owned)
}
#[tauri::command]
pub async fn select_companion_agent(
    window: tauri::WebviewWindow,
    presence: tauri::State<'_, Arc<crate::presence::Presence>>,
    agent: Option<String>,
) -> Result<crate::presence::BindingView, String> {
    main_only(&window)?;
    let request = presence.request(None).map_err(str::to_owned)?;
    let before = presence.view().generation;
    let token = crate::windows::companion::visibility_token(window.app_handle());
    let presence = presence.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || presence.select(request, agent))
        .await
        .map_err(|_| "companion_core_unavailable")?
        .map_err(str::to_owned)?;
    if result.agent.is_some() && result.generation != before {
        let app = window.app_handle().clone();
        crate::windows::companion::on_ui(app, move |app| {
            crate::windows::companion::show_for_selection(app, token)
        })
        .await?;
    }
    Ok(result)
}
#[tauri::command]
pub async fn target_companion_thread(
    window: tauri::WebviewWindow,
    presence: tauri::State<'_, Arc<crate::presence::Presence>>,
    watcher: tauri::State<'_, Arc<crate::presence::watcher::PresenceWatcher>>,
    thread_id: String,
    expected_generation: u64,
) -> Result<crate::presence::BindingView, String> {
    main_only(&window)?;
    let request = presence
        .request(Some(expected_generation))
        .map_err(str::to_owned)?;
    let presence = presence.inner().clone();
    let current = presence.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || presence.retarget(request, thread_id))
            .await
            .map_err(|_| "companion_core_unavailable")?
            .map_err(str::to_owned)?;
    watcher
        .wait_ready(&result, std::time::Duration::from_secs(2))
        .await
        .map_err(str::to_owned)?;
    if current.view().generation != result.generation {
        return Err("companion_binding_changed".into());
    }
    Ok(result)
}
#[tauri::command]
pub fn set_companion_previews(
    window: tauri::WebviewWindow,
    presence: tauri::State<'_, Arc<crate::presence::Presence>>,
    enabled: bool,
) -> Result<crate::presence::BindingView, String> {
    main_only(&window)?;
    presence.previews(enabled).map_err(str::to_owned)
}
#[tauri::command]
pub fn navigation_snapshot(
    window: tauri::WebviewWindow,
    navigation: tauri::State<'_, Arc<crate::navigation::Navigation>>,
) -> Result<Option<crate::navigation::PendingNavigation>, String> {
    main_only(&window)?;
    Ok(navigation.snapshot())
}
#[tauri::command]
pub fn ack_navigation(
    window: tauri::WebviewWindow,
    navigation: tauri::State<'_, Arc<crate::navigation::Navigation>>,
    revision: u64,
) -> Result<(), String> {
    main_only(&window)?;
    navigation.ack(revision);
    Ok(())
}
