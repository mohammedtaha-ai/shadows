// Capture only a prior external app: closing the editor must never activate main.
pub fn previous_foreground(window: &tauri::WebviewWindow) -> Option<(isize, u32)> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        let owned = window.hwnd().ok()?;
        let previous = GetForegroundWindow();
        if previous.is_null() {
            return None;
        }
        let mut owner_pid = 0;
        let mut previous_pid = 0;
        GetWindowThreadProcessId(owned.0 as _, &mut owner_pid);
        GetWindowThreadProcessId(previous, &mut previous_pid);
        if owner_pid == previous_pid || previous_pid == 0 {
            return None;
        }
        Some((previous as isize, previous_pid))
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}
pub fn restore_foreground(window: &tauri::WebviewWindow, previous: Option<(isize, u32)>) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
            SetForegroundWindow,
        };
        let Some((handle, expected_pid)) = previous else {
            return;
        };
        let Ok(owned) = window.hwnd() else {
            return;
        };
        let previous = handle as windows_sys::Win32::Foundation::HWND;
        let mut actual_pid = 0;
        GetWindowThreadProcessId(previous, &mut actual_pid);
        if std::ptr::eq(GetForegroundWindow(), owned.0 as _)
            && actual_pid == expected_pid
            && IsWindow(previous) != 0
            && IsWindowVisible(previous) != 0
            && IsIconic(previous) == 0
        {
            SetForegroundWindow(previous);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, previous);
    }
}
pub fn show(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            IsWindowVisible, ShowWindow, SW_SHOWNOACTIVATE,
        };
        let handle = window.hwnd().map_err(|_| "companion_show_failed")?;
        // ShowWindow returns previous visibility, not success. Never activate this surface.
        unsafe {
            ShowWindow(handle.0 as _, SW_SHOWNOACTIVATE);
        }
        if unsafe { IsWindowVisible(handle.0 as _) } == 0 {
            return Err("companion_show_failed".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("companion_platform_unavailable".into())
    }
}

pub fn hide(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindowVisible, ShowWindow, SW_HIDE};
        let handle = window.hwnd().map_err(|_| "companion_hide_failed")?;
        // Pair direct show with direct hide: Tao's cached visibility flags do not
        // observe SW_SHOWNOACTIVATE and can otherwise skip a requested hide.
        unsafe {
            ShowWindow(handle.0 as _, SW_HIDE);
        }
        if unsafe { IsWindowVisible(handle.0 as _) } != 0 {
            return Err("companion_hide_failed".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("companion_platform_unavailable".into())
    }
}
