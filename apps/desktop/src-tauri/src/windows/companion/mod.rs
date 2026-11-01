pub mod input;
mod no_activate;
mod placement;
pub mod state;
mod surface;

pub use surface::{
    EditorContext, SurfaceAgent, SurfaceAppearance, SurfacePanel, SurfacePresence, SurfacePreview,
};

pub use state::CompanionState;
use state::Model;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

type WebviewOptions = (Option<std::path::PathBuf>, Option<String>);

#[derive(Clone, Default)]
pub struct Store {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    model: Mutex<Model>,
    origin: Mutex<Option<(i32, i32)>>,
    input: Mutex<input::Placement>,
    toggle: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    rebuilding: Mutex<bool>,
    visibility_intent: std::sync::atomic::AtomicU64,
    webview_options: OnceLock<WebviewOptions>,
    editor_context: OnceLock<Arc<dyn Fn() -> Option<EditorContext> + Send + Sync>>,
}

// Queued window work shares one surface state without exposing storage to commands.
#[derive(Clone)]
pub struct Handle {
    store: Store,
}

impl Store {
    pub fn handle(&self) -> Handle {
        Handle {
            store: self.clone(),
        }
    }

    pub fn set_webview_options(&self, directory: Option<std::path::PathBuf>, args: Option<String>) {
        assert!(
            self.shared.webview_options.set((directory, args)).is_ok(),
            "webview options wired once"
        );
    }

    pub fn set_editor_context(
        &self,
        context: Arc<dyn Fn() -> Option<EditorContext> + Send + Sync>,
    ) {
        assert!(
            self.shared.editor_context.set(context).is_ok(),
            "editor context wired once"
        );
    }
}
fn model<T>(handle: &Handle, f: impl FnOnce(&mut Model) -> T) -> T {
    f(&mut handle
        .store
        .shared
        .model
        .lock()
        .expect("companion state poisoned"))
}
pub fn snapshot(handle: &Handle) -> CompanionState {
    model(handle, |m| m.view.clone())
}
fn publish(app: &tauri::AppHandle, handle: &Handle) -> CompanionState {
    let state = snapshot(handle);
    for label in ["main", "companion"] {
        let _ = app.emit_to(label, "companion-state", &state);
    }
    let toggle = handle.store.shared.toggle.lock().unwrap().clone();
    if let Some(item) = toggle {
        let _ = item.set_text(if state.visible {
            "Hide AYLUL"
        } else {
            "Show AYLUL"
        });
    }
    state
}
pub fn attach_toggle(
    app: &tauri::AppHandle,
    handle: &Handle,
    item: tauri::menu::MenuItem<tauri::Wry>,
) {
    *handle.store.shared.toggle.lock().unwrap() = Some(item);
    publish(app, handle);
}
fn page_started(app: &tauri::AppHandle, handle: &Handle) {
    let _ = input::set_input(app, handle, false);
    let _ = input::refresh(app, handle);
    let generation = model(handle, |m| m.begin_document());
    publish(app, handle);
    let app = app.clone();
    let handle = handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let owner = app.clone();
        let _ = app.run_on_main_thread(move || {
            model(&handle, |m| m.expire(generation));
            publish(&owner, &handle);
        });
    });
}
use input::fit;
pub fn setup_companion(app: &tauri::AppHandle, handle: &Handle) -> Result<(), String> {
    let result = (|| {
        if let Some(old) = app.get_webview_window("companion") {
            model(handle, |m| {
                m.initialized = false;
                m.finish(false, None);
            });
            publish(app, handle);
            *handle.store.shared.rebuilding.lock().unwrap() = true;
            old.destroy().map_err(|_| "companion_unavailable")?;
            return Ok(());
        }
        model(handle, |m| {
            m.initialized = false;
            m.deny_render();
        });
        let loading_handle = handle.clone();
        let mut builder = WebviewWindowBuilder::new(
            app,
            "companion",
            WebviewUrl::App("companion/companion.html".into()),
        )
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_navigation(|url| {
            (url.scheme() == "tauri"
                || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost")))
                && url.path() == "/companion/companion.html"
        })
        .on_web_resource_request(|request, response| {
            super::main_policy::reject_html_fallback(
                request.uri().path(),
                "/companion/companion.html",
                response,
            );
        })
        .title("AYLUL")
        .inner_size(240., 240.)
        .visible(false)
        .focused(false)
        .focusable(false)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .on_page_load(move |window, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Started {
                let app = window.app_handle().clone();
                let owner = app.clone();
                let handle = loading_handle.clone();
                let _ = app.run_on_main_thread(move || page_started(&owner, &handle));
            }
        });
        if let Some((directory, args)) = handle.store.shared.webview_options.get() {
            if let Some(directory) = directory {
                builder = builder.data_directory(directory.clone());
            }
            if let Some(args) = args {
                builder = builder.additional_browser_args(args);
            }
        }
        let w = builder.build().map_err(|_| "companion_unavailable")?;
        let owner = app.clone();
        let event_handle = handle.clone();
        w.on_window_event(move |event| {
            let handle = &event_handle;
            match event {
                tauri::WindowEvent::Destroyed => {
                    let rebuild =
                        std::mem::take(&mut *handle.store.shared.rebuilding.lock().unwrap());
                    if rebuild {
                        // Destroy removes the label after event dispatch. Queue from another
                        // thread so reconstruction runs after that removal, never inline here.
                        let app = owner.clone();
                        let handle = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let next = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                let _ = setup_companion(&next, &handle);
                            });
                        });
                    }
                }
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = set_visible(&owner, handle, false);
                }
                tauri::WindowEvent::Focused(false) => {
                    let _ = input::set_panel(&owner, handle, false, vec![]);
                }
                tauri::WindowEvent::Moved(_)
                | tauri::WindowEvent::Resized(_)
                | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    let _ = fit(&owner, handle, false);
                }
                _ => (),
            }
        });
        fit(app, handle, true)?;
        Ok(())
    })();
    if result.is_err() {
        model(handle, |m| {
            m.initialized = false;
            m.finish(false, Some("companion_unavailable".into()));
        });
    }
    publish(app, handle);
    result
}
fn apply_visibility(app: &tauri::AppHandle, handle: &Handle) -> Result<CompanionState, String> {
    let (requested, initialized) = model(handle, |m| (m.requested, m.initialized));
    let Some(w) = app.get_webview_window("companion") else {
        return Err("companion_unavailable".into());
    };
    let result = if requested && initialized {
        fit(app, handle, false).and_then(|_| no_activate::show(&w))
    } else {
        model(handle, |m| m.deny_render());
        publish(app, handle);
        no_activate::hide(&w)
    };
    let actual = w.is_visible().unwrap_or(false);
    model(handle, |m| m.finish(actual, result.as_ref().err().cloned()));
    let state = publish(app, handle);
    result.map(|_| state)
}
pub fn set_visible(
    app: &tauri::AppHandle,
    handle: &Handle,
    visible: bool,
) -> Result<CompanionState, String> {
    handle
        .store
        .shared
        .visibility_intent
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if !visible {
        input::set_panel(app, handle, false, vec![])?;
        input::set_input(app, handle, false)?;
    }
    model(handle, |m| m.request_visible(visible));
    let retry = visible
        && (app.get_webview_window("companion").is_none()
            || model(handle, |m| !m.initialized && m.view.error.is_some()));
    if retry {
        setup_companion(app, handle)?;
        return Ok(snapshot(handle));
    }
    apply_visibility(app, handle)
}
pub fn ready(app: &tauri::AppHandle, handle: &Handle) -> Result<CompanionState, String> {
    model(handle, |m| {
        m.initialize(m.generation);
    });
    apply_visibility(app, handle)
}
pub fn failed(app: &tauri::AppHandle, handle: &Handle) -> Result<CompanionState, String> {
    let visible = app
        .get_webview_window("companion")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    let _ = input::set_input(app, handle, false);
    model(handle, |m| m.fail(visible));
    Ok(publish(app, handle))
}
pub fn set_paused(
    app: &tauri::AppHandle,
    handle: &Handle,
    paused: bool,
) -> Result<CompanionState, String> {
    model(handle, |m| m.pause(paused));
    Ok(publish(app, handle))
}
pub fn drag(app: &tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("companion")
        .ok_or("companion_unavailable")?
        .start_dragging()
        .map_err(|_| "companion_drag_failed".into())
}
// Commands use this dispatcher so Win32 calls and state transitions have one UI-thread owner.
pub async fn on_ui<T: Send + 'static>(
    app: tauri::AppHandle,
    work: impl FnOnce(&tauri::AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let owner = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(work(&owner));
    })
    .map_err(|_| "companion_unavailable")?;
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv().map_err(|_| "companion_unavailable".to_string())
    })
    .await
    .map_err(|_| "companion_unavailable".to_string())??
}

pub fn visibility_token(handle: &Handle) -> u64 {
    handle
        .store
        .shared
        .visibility_intent
        .load(std::sync::atomic::Ordering::SeqCst)
}
pub fn show_for_selection(
    app: &tauri::AppHandle,
    handle: &Handle,
    token: u64,
) -> Result<(), String> {
    if visibility_token(handle) == token {
        set_visible(app, handle, true)?;
    }
    Ok(())
}

pub fn republish(app: &tauri::AppHandle, handle: &Handle) {
    publish(app, handle);
}
pub fn set_panel(
    app: &tauri::AppHandle,
    handle: &Handle,
    open: bool,
    agents: Vec<SurfaceAgent>,
) -> Result<CompanionState, String> {
    input::set_panel(app, handle, open, agents)
}
pub fn present(
    app: &tauri::AppHandle,
    handle: &Handle,
    presence: SurfacePresence,
    appearance: Option<SurfaceAppearance>,
) {
    model(handle, |model| model.present(presence, appearance));
    publish(app, handle);
}
