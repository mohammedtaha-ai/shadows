fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "hide_main",
            "desktop_connection",
            "desktop_connection_config",
            "save_desktop_connection",
            "retry_desktop_connection",
            "open_external",
            "auth_action",
            "cancel_login",
            "aylul_call",
            "assistant_call",
            "assistant_local_call",
            "assistant_sources",
            "companion_state",
            "companion_ready",
            "companion_failed",
            "set_companion_visible",
            "set_companion_paused",
            "drag_companion",
            "open_main_from_companion",
        ]),
    ))
    .expect("native build configuration")
}
