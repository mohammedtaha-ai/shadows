mod app_paths;
mod assistant;
pub mod auth;
mod commands;
mod core_connection;
pub mod native_login;
mod sidecar;
mod windows;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::hide_main,
            commands::desktop::desktop_connection,
            commands::desktop::desktop_connection_config,
            commands::desktop::save_desktop_connection,
            commands::desktop::retry_desktop_connection,
            commands::desktop::open_external,
            commands::auth_action,
            commands::cancel_login,
            commands::aylul_call,
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
            use tauri::Emitter;
            use tauri::Manager;
            let paths = app_paths::AppPaths::resolve(app)?;
            app.manage(auth::AuthController::new(paths.auth()));
            let connection = core_connection::Connection::new(paths.root().to_path_buf());
            let owner = app.handle().clone();
            connection.set_notifier(std::sync::Arc::new(move |view| {
                let _ = owner.emit_to("main", "desktop-connection", view);
            }));
            app.manage(connection.clone());
            let acceptance = paths.acceptance;
            app.manage(paths);
            tauri::async_runtime::spawn_blocking(move || connection.retry());
            app.manage(windows::companion::Store::default());
            app.manage(std::sync::Arc::new(if acceptance {
                sidecar::SidecarState(None)
            } else {
                sidecar::start_default()
            }));
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
