use crate::core_connection::{config::LaunchConfig, Connection, ConnectionView};
use std::sync::Arc;

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
