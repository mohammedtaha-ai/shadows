//! Native owner assembly and lifetime.
use crate::{
    app_paths, auth, core_connection, desktop_preferences, navigation, presence, sidecar, windows,
};
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub struct DesktopApp {
    desktop_preferences: desktop_preferences::DesktopPreferences,
    account: Arc<auth::AuthController>,
    companion_window: windows::companion::Store,
    main_window: windows::MainWindow,
    companion: Arc<presence::Presence>,
    navigation: Arc<navigation::Navigation>,
    core_connection: Arc<core_connection::Connection>,
    app_paths: app_paths::AppPaths,
    activity: Arc<sidecar::SidecarState>,
}

impl DesktopApp {
    pub fn start(app: &mut tauri::App) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let paths = app_paths::AppPaths::resolve(app)?;
        let preferences = desktop_preferences::DesktopPreferences::new(
            paths.root().to_path_buf(),
            desktop_preferences::PluginLoginItem(app.handle().clone()),
        );
        let auth = auth::AuthController::new(paths.auth());
        let companion = windows::companion::Store::default();
        let connection = core_connection::Connection::new(paths.root().to_path_buf());
        let presence = presence::Presence::new(paths.root().to_path_buf(), connection.clone());
        let editor_presence = Arc::downgrade(&presence);
        companion.set_editor_context(Arc::new(move || {
            let owner = editor_presence.upgrade()?;
            owner.editor_context()
        }));
        let publisher = app.handle().clone();
        let publishing_presence = Arc::downgrade(&presence);
        presence.start_watcher(Arc::new(move |update| {
            let app = publisher.clone();
            let presence = publishing_presence.clone();
            let _ = publisher.run_on_main_thread(move || {
                if let Some(presence) = presence.upgrade() {
                    presence.publish(update, app.try_state::<Arc<DesktopApp>>().is_some());
                }
            });
        }));
        let navigation = Arc::new(navigation::Navigation::default());
        let dispatch = app.handle().clone();
        let visibility = app.handle().clone();
        let republisher = app.handle().clone();
        let presenter = app.handle().clone();
        presence.attach_workflows(
            navigation.clone(),
            presence::WindowActions {
                on_ui: Arc::new(move |action| {
                    Box::pin(windows::companion::on_ui(dispatch.clone(), action))
                }),
                visibility_token: Arc::new(move || {
                    windows::companion::visibility_token(&visibility)
                }),
                show_for_selection: windows::companion::show_for_selection,
                republish: Arc::new(move || windows::companion::republish(&republisher)),
                present: Arc::new(move |projection, appearance| {
                    windows::companion::present(&presenter, projection, appearance)
                }),
            },
        );
        let binding_presence = Arc::downgrade(&presence);
        let binding_connection = std::sync::Arc::downgrade(&connection);
        let emitter = app.handle().clone();
        presence.set_notifier(std::sync::Arc::new(move |view| {
            if let Some(presence) = binding_presence.upgrade() {
                presence.sync_watcher(
                    view.clone(),
                    binding_connection
                        .upgrade()
                        .and_then(|connection| connection.ready()),
                );
            }
            let _ = emitter.emit_to("main", "companion-binding", view);
            let owner = emitter.clone();
            let _ = emitter.run_on_main_thread(move || {
                let _ = windows::companion::input::refresh(&owner);
            });
        }));
        let binding = std::sync::Arc::downgrade(&presence);
        let owner = app.handle().clone();
        connection.set_notifier(std::sync::Arc::new(move |view| {
            if let Some(presence) = binding.upgrade() {
                presence.configuration_changed(view.configuration_id.clone());
                presence.sync_watcher(presence.view(), presence.connection.ready());
            }
            let _ = owner.emit_to("main", "desktop-connection", view);
        }));
        let origin_connection = connection.clone();
        let main_window = windows::MainWindow::new(
            Arc::new(move || origin_connection.configured().map(|config| config.origin)),
            paths.webview("main"),
            paths.browser_args("main"),
        );
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
            desktop_preferences: preferences,
            account: auth,
            companion_window: companion,
            main_window,
            companion: presence,
            navigation,
            core_connection: connection,
            app_paths: paths,
            activity: sidecar,
        }))
    }

    pub fn preferences(&self) -> &desktop_preferences::DesktopPreferences {
        &self.desktop_preferences
    }

    pub fn auth(&self) -> &Arc<auth::AuthController> {
        &self.account
    }

    pub fn companion(&self) -> &windows::companion::Store {
        &self.companion_window
    }

    pub fn main_window(&self) -> &windows::MainWindow {
        &self.main_window
    }

    pub fn presence(&self) -> &Arc<presence::Presence> {
        &self.companion
    }

    pub fn navigation(&self) -> &Arc<navigation::Navigation> {
        &self.navigation
    }

    pub fn connection(&self) -> &Arc<core_connection::Connection> {
        &self.core_connection
    }

    pub fn paths(&self) -> &app_paths::AppPaths {
        &self.app_paths
    }

    pub fn sidecar(&self) -> &Arc<sidecar::SidecarState> {
        &self.activity
    }

    pub fn stop(&self) {
        self.presence().stop();
        self.sidecar().stop();
    }
}
