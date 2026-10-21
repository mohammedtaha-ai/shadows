mod app_paths;
mod assistant;
pub mod auth;
mod commands;
mod core_connection;
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
            use tauri::Emitter;
            use tauri::Manager;
            let paths = app_paths::AppPaths::resolve(app)?;
            app.manage(windows::preferences::Store::new(paths.root().to_path_buf()));
            app.manage(auth::AuthController::new(paths.auth()));
            app.manage(windows::companion::Store::default());
            let connection = core_connection::Connection::new(paths.root().to_path_buf());
            let presence = presence::Presence::new(paths.root().to_path_buf(), connection.clone());
            let publisher = app.handle().clone();
            let watcher =
                presence::watcher::PresenceWatcher::new(std::sync::Arc::new(move |mut update| {
                    let app = publisher.clone();
                    let _ = publisher.run_on_main_thread(move || {
                        if !update.is_current() {
                            return;
                        }
                        if let Some(owner) = app.try_state::<std::sync::Arc<presence::Presence>>() {
                            if !update.matches_binding(&owner.view()) {
                                return;
                            }
                        }
                        if let Some(scope) = update.scope.as_ref() {
                            let Some(owner) = app.try_state::<std::sync::Arc<presence::Presence>>()
                            else {
                                return;
                            };
                            let binding = owner.view();
                            let Some(ready) = owner.connection.ready() else {
                                return;
                            };
                            if binding.generation != scope.generation
                                || binding.configuration_id.as_deref()
                                    != Some(scope.configuration_id.as_str())
                                || ready.runtime_id != scope.runtime_id
                            {
                                return;
                            }
                        }
                        if update.republish_only {
                            windows::companion::republish(&app);
                        } else {
                            windows::companion::present(
                                &app,
                                update.projection.clone(),
                                update.appearance.clone(),
                            );
                        }
                        update.acknowledge();
                    });
                }));
            app.manage(watcher.clone());
            let binding_watcher = watcher.clone();
            let binding_connection = std::sync::Arc::downgrade(&connection);
            let emitter = app.handle().clone();
            presence.set_notifier(std::sync::Arc::new(move |view| {
                binding_watcher.sync(
                    view.clone(),
                    binding_connection
                        .upgrade()
                        .and_then(|connection| connection.ready()),
                );
                let _ = emitter.emit_to("main", "companion-binding", view);
                let owner = emitter.clone();
                let _ = emitter.run_on_main_thread(move || {
                    let _ = windows::companion::input::refresh(&owner);
                });
            }));
            app.manage(presence.clone());
            app.manage(std::sync::Arc::new(navigation::Navigation::default()));
            let binding = std::sync::Arc::downgrade(&presence);
            let owner = app.handle().clone();
            connection.set_notifier(std::sync::Arc::new(move |view| {
                if let Some(presence) = binding.upgrade() {
                    presence.configuration_changed(view.configuration_id.clone());
                    watcher.sync(presence.view(), presence.connection.ready());
                }
                let _ = owner.emit_to("main", "desktop-connection", view);
            }));
            app.manage(connection.clone());
            let acceptance = paths.acceptance;
            app.manage(paths);
            tauri::async_runtime::spawn_blocking(move || connection.retry());
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
                if let Some(watcher) =
                    app.try_state::<std::sync::Arc<presence::watcher::PresenceWatcher>>()
                {
                    watcher.stop();
                }
                if let Some(sidecar) = app.try_state::<std::sync::Arc<sidecar::SidecarState>>() {
                    sidecar.stop();
                }
            }
        });
}
