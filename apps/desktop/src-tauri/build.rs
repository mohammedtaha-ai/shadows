fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "hide_main",
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
    .expect("native build configuration");
    // Temporary probe: examples need the same Common Controls v6 manifest as bins.
    #[cfg(windows)]
    println!("cargo:rustc-link-arg-examples={}/resource.lib", std::env::var("OUT_DIR").unwrap())
}
