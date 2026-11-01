fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Native dialog imports require versioned controls in test executables too.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(concat!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' ",
            "name='Microsoft.Windows.Common-Controls' version='6.0.0.0' ",
            "processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        ));
    }
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "hide_main",
                "desktop_preferences",
                "set_desktop_preferences",
                "companion_binding",
                "refresh_companion_appearance",
                "select_companion_agent",
                "target_companion_thread",
                "set_companion_previews",
                "navigation_snapshot",
                "ack_navigation",
                "desktop_connection",
                "desktop_connection_config",
                "save_desktop_connection",
                "retry_desktop_connection",
                "open_external",
                "auth_action",
                "cancel_login",
                "aylul_call",
                "companion_state",
                "companion_panel",
                "answer_companion_permission",
                "companion_send",
                "companion_input",
                "companion_ready",
                "companion_failed",
                "set_companion_visible",
                "set_companion_paused",
                "drag_companion",
                "open_main_from_companion",
            ])),
    )
    .expect("native build configuration")
}
