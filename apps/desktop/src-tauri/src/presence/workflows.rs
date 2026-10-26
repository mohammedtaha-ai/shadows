//! Companion coordination across the observer, native surface and main navigation.
use super::{BindingView, Presence, RefreshAppearanceReply};
use crate::{navigation::Navigation, windows::companion};
use std::{future::Future, pin::Pin, sync::Arc};

type UiAction = Box<dyn FnOnce(&tauri::AppHandle) -> Result<(), String> + Send>;
type UiResult = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
type Present =
    Arc<dyn Fn(companion::SurfacePresence, Option<companion::SurfaceAppearance>) + Send + Sync>;

/// Callbacks keep native surface lifetime and dispatch with its composition adapter.
pub struct WindowActions {
    pub on_ui: Arc<dyn Fn(UiAction) -> UiResult + Send + Sync>,
    pub visibility_token: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub show_for_selection: fn(&tauri::AppHandle, u64) -> Result<(), String>,
    pub republish: Arc<dyn Fn() + Send + Sync>,
    pub present: Present,
}

pub(super) struct Dependencies {
    navigation: Arc<Navigation>,
    window: WindowActions,
}

impl Presence {
    pub fn attach_workflows(&self, navigation: Arc<Navigation>, window: WindowActions) {
        // Publication uses these dependencies on the UI thread.
        assert!(
            self.workflows
                .set(Dependencies { navigation, window })
                .is_ok(),
            "companion workflows are wired once"
        );
    }

    fn workflows(&self) -> &Dependencies {
        self.workflows
            .get()
            .expect("companion workflows are wired before use")
    }

    pub async fn select_companion_agent(
        self: &Arc<Self>,
        agent: Option<String>,
    ) -> Result<BindingView, String> {
        let request = self.request(None).map_err(str::to_owned)?;
        let before = self.view().generation;
        let window = &self.workflows().window;
        let token = (window.visibility_token)();
        let presence = self.clone();
        let result = tauri::async_runtime::spawn_blocking(move || presence.select(request, agent))
            .await
            .map_err(|_| "companion_core_unavailable")?
            .map_err(str::to_owned)?;
        if result.agent.is_some() && result.generation != before {
            let show = window.show_for_selection;
            (window.on_ui)(Box::new(move |app| show(app, token))).await?;
        }
        Ok(result)
    }

    pub async fn target_companion_thread(
        self: &Arc<Self>,
        thread_id: String,
        expected_generation: u64,
    ) -> Result<BindingView, String> {
        let request = self
            .request(Some(expected_generation))
            .map_err(str::to_owned)?;
        let presence = self.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || presence.retarget(request, thread_id))
                .await
                .map_err(|_| "companion_core_unavailable")?
                .map_err(str::to_owned)?;
        self.watcher()
            .wait_ready(&result, std::time::Duration::from_secs(2))
            .await
            .map_err(str::to_owned)?;
        if self.view().generation != result.generation {
            return Err("companion_binding_changed".into());
        }
        Ok(result)
    }

    pub async fn refresh_appearance(&self) -> Result<RefreshAppearanceReply, &'static str> {
        self.watcher().refresh_appearance(&self.view()).await
    }

    pub async fn open_main_from_companion(self: &Arc<Self>) -> Result<(), String> {
        let presence = self.clone();
        let destination = tauri::async_runtime::spawn_blocking(move || presence.destination())
            .await
            .map_err(|_| "main_open_failed")?;
        let dependencies = self.workflows();
        let navigation = dependencies.navigation.clone();
        (dependencies.window.on_ui)(Box::new(move |app| navigation.open_main(app, destination)))
            .await
    }

    pub fn editor_context(&self) -> Option<companion::EditorContext> {
        let (scope, context) = self.context();
        let agent = self.view().agent;
        if self.context() != (scope.clone(), context.clone()) {
            return None;
        }
        Some(companion::EditorContext {
            scope,
            context,
            agent,
        })
    }

    pub fn publish(&self, mut update: super::watcher::Update, registered: bool) {
        if !update.is_current() {
            return;
        }
        if registered && !update.matches_binding(&self.view()) {
            return;
        }
        if let Some(scope) = update.scope.as_ref() {
            if !registered {
                return;
            }
            let binding = self.view();
            let Some(ready) = self.connection.ready() else {
                return;
            };
            if binding.generation != scope.generation
                || binding.configuration_id.as_deref() != Some(scope.configuration_id.as_str())
                || ready.runtime_id != scope.runtime_id
            {
                return;
            }
        }
        let window = &self.workflows().window;
        if update.republish_only {
            (window.republish)();
        } else {
            (window.present)(
                update.projection.clone().into_surface(),
                update
                    .appearance
                    .clone()
                    .map(|appearance| companion::SurfaceAppearance {
                        color: appearance.color,
                    }),
            );
        }
        update.acknowledge();
    }
}
