fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "hide_main",
            "auth_action",
            "cancel_login",
            "aylul_call",
            "assistant_call",
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
