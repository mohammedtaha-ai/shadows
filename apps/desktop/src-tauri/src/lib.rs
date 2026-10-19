mod assistant;
pub mod auth;
mod commands;
pub mod native_login;
mod sidecar;
mod windows;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::hide_main,
            commands::auth_action,
            commands::cancel_login,
            commands::aylul_call,
            commands::assistant_call,
            commands::companion::companion_state,
            commands::companion::companion_ready,
            commands::companion::companion_failed,
            commands::companion::set_companion_visible,
            commands::companion::set_companion_paused,
            commands::companion::drag_companion,
            commands::companion::open_main_from_companion
        ])
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::open_main(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = windows::open_main(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri::Manager;
            let path = app.path().app_data_dir()?.join("auth-v1.dpapi");
            app.manage(auth::AuthController::new(path));
            app.manage(windows::companion::Store::default());
            app.manage(std::sync::Arc::new(sidecar::start_default()));
            windows::setup(app)
        })
        .build(tauri::generate_context!())
        .expect("NYLRA native runtime failed")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(sidecar) = app.try_state::<std::sync::Arc<sidecar::SidecarState>>() {
                    sidecar.stop();
                }
            }
        });
}
