//! Native owner assembly and lifetime.
use crate::{app_paths, auth, core_connection, navigation, presence, sidecar, windows};
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub struct DesktopApp {
    preferences: windows::preferences::Store,
    auth: Arc<auth::AuthController>,
    companion: windows::companion::Store,
    watcher: Arc<presence::watcher::PresenceWatcher>,
    presence: Arc<presence::Presence>,
    navigation: Arc<navigation::Navigation>,
    connection: Arc<core_connection::Connection>,
    paths: app_paths::AppPaths,
    sidecar: Arc<sidecar::SidecarState>,
}

impl DesktopApp {
    pub fn start(app: &mut tauri::App) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let paths = app_paths::AppPaths::resolve(app)?;
        let preferences = windows::preferences::Store::new(paths.root().to_path_buf());
        let auth = auth::AuthController::new(paths.auth());
        let companion = windows::companion::Store::default();
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
                    if let Some(owner) = app.try_state::<Arc<DesktopApp>>() {
                        if !update.matches_binding(&owner.presence().view()) {
                            return;
                        }
                    }
                    if let Some(scope) = update.scope.as_ref() {
                        let Some(owner) = app.try_state::<Arc<DesktopApp>>() else {
                            return;
                        };
                        let binding = owner.presence().view();
                        let Some(ready) = owner.presence().connection.ready() else {
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
        let navigation = Arc::new(navigation::Navigation::default());
        let binding = std::sync::Arc::downgrade(&presence);
        let owner = app.handle().clone();
        let connection_watcher = watcher.clone();
        connection.set_notifier(std::sync::Arc::new(move |view| {
            if let Some(presence) = binding.upgrade() {
                presence.configuration_changed(view.configuration_id.clone());
                connection_watcher.sync(presence.view(), presence.connection.ready());
            }
            let _ = owner.emit_to("main", "desktop-connection", view);
        }));
        let acceptance = paths.acceptance;
        tauri::async_runtime::spawn_blocking({
            let connection = connection.clone();
            move || connection.retry()
        });
        let sidecar = Arc::new(if acceptance {
            sidecar::SidecarState(None)
        } else {
            sidecar::start_default()
        });
        Ok(Arc::new(Self {
            preferences,
            auth,
            companion,
            watcher,
            presence,
            navigation,
            connection,
            paths,
            sidecar,
        }))
    }

    pub fn preferences(&self) -> &windows::preferences::Store {
        &self.preferences
    }

    pub fn auth(&self) -> &Arc<auth::AuthController> {
        &self.auth
    }

    pub fn companion(&self) -> &windows::companion::Store {
        &self.companion
    }

    pub fn watcher(&self) -> &Arc<presence::watcher::PresenceWatcher> {
        &self.watcher
    }

    pub fn presence(&self) -> &Arc<presence::Presence> {
        &self.presence
    }

    pub fn navigation(&self) -> &Arc<navigation::Navigation> {
        &self.navigation
    }

    pub fn connection(&self) -> &Arc<core_connection::Connection> {
        &self.connection
    }

    pub fn paths(&self) -> &app_paths::AppPaths {
        &self.paths
    }

    pub fn sidecar(&self) -> &Arc<sidecar::SidecarState> {
        &self.sidecar
    }

    pub fn login_item(&self, app: tauri::AppHandle) -> windows::preferences::PluginLoginItem {
        windows::preferences::PluginLoginItem(app)
    }

    pub fn stop(&self) {
        self.watcher().stop();
        self.sidecar().stop();
    }
}
