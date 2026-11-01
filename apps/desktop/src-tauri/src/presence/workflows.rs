//! Companion coordination across the observer, native surface and main navigation.
use super::{BindingView, PermissionAnswerReply, Presence, RefreshAppearanceReply};
use crate::{navigation::Navigation, windows::companion};
use std::{future::Future, pin::Pin, sync::Arc};

type UiAction = Box<dyn FnOnce(&tauri::AppHandle) -> Result<(), String> + Send>;
type UiResult = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
type ShowForSelection = Arc<dyn Fn(&tauri::AppHandle, u64) -> Result<(), String> + Send + Sync>;

type Present =
    Arc<dyn Fn(companion::SurfacePresence, Option<companion::SurfaceAppearance>) + Send + Sync>;
type Panel = Arc<
    dyn Fn(
            &tauri::AppHandle,
            bool,
            Vec<companion::SurfaceAgent>,
        ) -> Result<companion::CompanionState, String>
        + Send
        + Sync,
>;

/// Callbacks keep native surface lifetime and dispatch with its composition adapter.
pub struct WindowActions {
    pub on_ui: Arc<dyn Fn(UiAction) -> UiResult + Send + Sync>,
    pub visibility_token: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub show_for_selection: ShowForSelection,
    pub republish: Arc<dyn Fn() + Send + Sync>,
    pub present: Present,
    pub panel: Panel,
}

pub(super) struct Dependencies {
    navigation: Arc<Navigation>,
    window: WindowActions,
}

impl Presence {
    pub async fn companion_panel(
        self: &Arc<Self>,
        open: bool,
    ) -> Result<companion::CompanionState, String> {
        let ticket = {
            let mut model = self.inner.lock().unwrap();
            model.panel_ticket += 1;
            model.panel_ticket
        };
        let visibility = (self.workflows().window.visibility_token)();
        let binding = if open {
            self.snapshot().map_err(str::to_owned)?
        } else {
            self.view()
        };
        let ready = if open {
            Some(
                self.connection
                    .ready()
                    .ok_or("companion_core_unavailable")?,
            )
        } else {
            None
        };
        let agents = if let Some(ready) = ready.as_ref() {
            let client =
                super::core_client::Client::new(ready.clone(), std::time::Duration::from_secs(10))
                    .map_err(|_| "companion_core_unavailable")?;
            client
                .panel_agents(&binding)
                .await
                .map_err(|_| "companion_core_unavailable")?
        } else {
            vec![]
        };
        let agents = agents
            .into_iter()
            .map(|agent| companion::SurfaceAgent {
                slug: agent.slug,
                name: agent.name,
                description: agent.description,
                color: agent.color,
                outfit: agent.outfit,
                bound: agent.bound,
            })
            .collect();
        let owner = self.clone();
        let panel = self.workflows().window.panel.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        (self.workflows().window.on_ui)(Box::new(move |app| {
            let ticket_matches = owner.inner.lock().unwrap().panel_ticket == ticket;
            let valid = ticket_matches
                && (!open || owner.view().generation == binding.generation)
                && (!open || (owner.workflows().window.visibility_token)() == visibility)
                && ready.as_ref().is_none_or(|captured| {
                    owner.connection.ready().is_some_and(|current| {
                        current.runtime_id == captured.runtime_id
                            && current.configuration_id == captured.configuration_id
                    })
                });
            let result = if valid {
                panel(app, open, agents)
            } else {
                Err("companion_binding_changed".into())
            };
            let _ = sender.send(result);
            Ok(())
        }))
        .await?;
        receiver.await.map_err(|_| "companion_panel_unavailable")?
    }
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
            let show = window.show_for_selection.clone();
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

    /// Only the bound conversation's request is answered, through the one private Core client.
    pub async fn answer_companion_permission(
        self: &Arc<Self>,
        thread_id: String,
        request_id: String,
        option_id: String,
        command_id: String,
    ) -> Result<PermissionAnswerReply, String> {
        let command_id = uuid::Uuid::parse_str(&command_id).map_err(|_| "permission_invalid")?;
        if request_id.is_empty()
            || request_id.len() > 128
            || !request_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || option_id.is_empty()
            || option_id.len() > 200
        {
            return Err("permission_invalid".into());
        }
        let binding = self.snapshot().map_err(str::to_owned)?;
        if binding
            .target
            .as_ref()
            .map(|target| target.thread_id.as_str())
            != Some(&thread_id)
        {
            return Err("companion_binding_changed".into());
        }
        let ready = self
            .connection
            .ready()
            .ok_or("companion_core_unavailable")?;
        tauri::async_runtime::spawn_blocking(move || {
            super::core_client::Blocking::new(&ready).answer_permission(
                &thread_id,
                &request_id,
                &option_id,
                command_id,
            )
        })
        .await
        .map_err(|_| "companion_core_unavailable")?
        .map_err(str::to_owned)?;
        Ok(PermissionAnswerReply { ok: true })
    }

    pub async fn refresh_appearance(&self) -> Result<RefreshAppearanceReply, &'static str> {
        self.watcher().refresh_appearance(&self.view()).await
    }

    pub async fn open_main_from_companion(
        self: &Arc<Self>,
        target: Option<String>,
    ) -> Result<(), String> {
        let target: super::MainTarget = target
            .map(|value| serde_json::from_value(serde_json::Value::String(value)))
            .transpose()
            .map_err(|_| "navigation_invalid")?
            .unwrap_or_default();
        let presence = self.clone();
        let destination = tauri::async_runtime::spawn_blocking(move || {
            if matches!(target, super::MainTarget::Bound) {
                presence.destination()
            } else {
                presence.destination_for(target)
            }
        })
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
        let mut surface = update.projection.clone().into_surface();
        surface.waiting = update.waiting.clone();
        if update.republish_only {
            (window.republish)();
        } else {
            (window.present)(
                surface,
                update
                    .appearance
                    .clone()
                    .map(|appearance| companion::SurfaceAppearance {
                        color: appearance.color,
                        outfit: appearance.outfit,
                    }),
            );
        }
        update.acknowledge();
    }
}
