mod app;
mod app_paths;
mod assistant;
pub mod auth;
mod commands;
mod core_connection;
mod desktop_preferences;
pub mod native_login;
mod navigation;
mod presence;
mod sidecar;
mod windows;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::hide_main,
            commands::preferences::desktop_preferences,
            commands::preferences::set_desktop_preferences,
            commands::desktop::companion_binding,
            commands::desktop::refresh_companion_appearance,
            commands::desktop::select_companion_agent,
            commands::desktop::target_companion_thread,
            commands::desktop::set_companion_previews,
            commands::desktop::navigation_snapshot,
            commands::desktop::ack_navigation,
            commands::desktop::desktop_connection,
            commands::desktop::desktop_connection_config,
            commands::desktop::save_desktop_connection,
            commands::desktop::retry_desktop_connection,
            commands::desktop::open_external,
            commands::auth_action,
            commands::cancel_login,
            commands::aylul_call,
            commands::companion::companion_state,
            commands::companion::companion_panel,
            commands::companion::companion_send,
            commands::companion::companion_input,
            commands::companion::companion_ready,
            commands::companion::companion_failed,
            commands::companion::set_companion_visible,
            commands::companion::set_companion_paused,
            commands::companion::drag_companion,
            commands::companion::open_main_from_companion
        ])
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
            let desktop = app::DesktopApp::start(app)?;
            app.manage(desktop.clone());
            desktop.setup_windows(app)
        })
        .build(tauri::generate_context!())
        .expect("NYLRA native runtime failed")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(desktop) = app.try_state::<std::sync::Arc<app::DesktopApp>>() {
                    desktop.stop();
                }
            }
        });
}
