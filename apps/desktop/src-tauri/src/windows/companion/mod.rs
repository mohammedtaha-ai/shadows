pub mod input;
mod no_activate;
mod placement;
pub mod state;
mod surface;

pub use surface::{EditorContext, SurfaceAppearance, SurfacePresence, SurfacePreview};

use state::{CompanionState, Model};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct Store {
    model: Mutex<Model>,
    origin: Mutex<Option<(i32, i32)>>,
    input: Mutex<input::Placement>,
    toggle: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    rebuilding: Mutex<bool>,
    visibility_intent: std::sync::atomic::AtomicU64,
    editor_context: OnceLock<Arc<dyn Fn() -> Option<EditorContext> + Send + Sync>>,
}
impl Store {
    pub fn set_editor_context(
        &self,
        context: Arc<dyn Fn() -> Option<EditorContext> + Send + Sync>,
    ) {
        assert!(
            self.editor_context.set(context).is_ok(),
            "editor context wired once"
        );
    }
}
fn model<T>(app: &tauri::AppHandle, f: impl FnOnce(&mut Model) -> T) -> T {
    f(&mut app
        .state::<std::sync::Arc<crate::app::DesktopApp>>()
        .companion()
        .model
        .lock()
        .expect("companion state poisoned"))
}
pub fn snapshot(app: &tauri::AppHandle) -> CompanionState {
    model(app, |m| m.view.clone())
}
fn publish(app: &tauri::AppHandle) -> CompanionState {
    let state = snapshot(app);
    for label in ["main", "companion"] {
        let _ = app.emit_to(label, "companion-state", &state);
    }
    let toggle = app
        .state::<std::sync::Arc<crate::app::DesktopApp>>()
        .companion()
        .toggle
        .lock()
        .unwrap()
        .clone();
    if let Some(item) = toggle {
        let _ = item.set_text(if state.visible {
            "Hide AYLUL"
        } else {
            "Show AYLUL"
        });
    }
    state
}
pub fn attach_toggle(app: &tauri::AppHandle, item: tauri::menu::MenuItem<tauri::Wry>) {
    *app.state::<std::sync::Arc<crate::app::DesktopApp>>()
        .companion()
        .toggle
        .lock()
        .unwrap() = Some(item);
    publish(app);
}
fn page_started(app: &tauri::AppHandle) {
    let _ = input::set_input(app, false);
    let _ = input::refresh(app);
    let generation = model(app, |m| m.begin_document());
    publish(app);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let owner = app.clone();
        let _ = app.run_on_main_thread(move || {
            model(&owner, |m| m.expire(generation));
            publish(&owner);
        });
    });
}
use input::fit;
pub fn setup_companion(app: &tauri::AppHandle) -> Result<(), String> {
    let result = (|| {
        if let Some(old) = app.get_webview_window("companion") {
            model(app, |m| {
                m.initialized = false;
                m.finish(false, None);
            });
            publish(app);
            *app.state::<std::sync::Arc<crate::app::DesktopApp>>()
                .companion()
                .rebuilding
                .lock()
                .unwrap() = true;
            old.destroy().map_err(|_| "companion_unavailable")?;
            return Ok(());
        }
        model(app, |m| {
            m.initialized = false;
            m.deny_render();
        });
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
        .on_page_load(|window, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Started {
                let app = window.app_handle().clone();
                let owner = app.clone();
                let _ = app.run_on_main_thread(move || page_started(&owner));
            }
        });
        if let Some(directory) = app
            .state::<std::sync::Arc<crate::app::DesktopApp>>()
            .paths()
            .webview("companion")
        {
            builder = builder.data_directory(directory);
        }
        if let Some(args) = app
            .state::<std::sync::Arc<crate::app::DesktopApp>>()
            .paths()
            .browser_args("companion")
        {
            builder = builder.additional_browser_args(&args);
        }
        let w = builder.build().map_err(|_| "companion_unavailable")?;
        let owner = app.clone();
        w.on_window_event(move |event| match event {
            tauri::WindowEvent::Destroyed => {
                let rebuild = std::mem::take(
                    &mut *owner
                        .state::<std::sync::Arc<crate::app::DesktopApp>>()
                        .companion()
                        .rebuilding
                        .lock()
                        .unwrap(),
                );
                if rebuild {
                    // Destroy removes the label after event dispatch. Queue from another
                    // thread so reconstruction runs after that removal, never inline here.
                    let app = owner.clone();
                    tauri::async_runtime::spawn(async move {
                        let next = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            let _ = setup_companion(&next);
                        });
                    });
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = set_visible(&owner, false);
            }
            tauri::WindowEvent::Moved(_)
            | tauri::WindowEvent::Resized(_)
            | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                let _ = fit(&owner, false);
            }
            _ => (),
        });
        fit(app, true)?;
        Ok(())
    })();
    if result.is_err() {
        model(app, |m| {
            m.initialized = false;
            m.finish(false, Some("companion_unavailable".into()));
        });
    }
    publish(app);
    result
}
fn apply_visibility(app: &tauri::AppHandle) -> Result<CompanionState, String> {
    let (requested, initialized) = model(app, |m| (m.requested, m.initialized));
    let Some(w) = app.get_webview_window("companion") else {
        return Err("companion_unavailable".into());
    };
    let result = if requested && initialized {
        fit(app, false).and_then(|_| no_activate::show(&w))
    } else {
        model(app, |m| m.deny_render());
        publish(app);
        no_activate::hide(&w)
    };
    let actual = w.is_visible().unwrap_or(false);
    model(app, |m| m.finish(actual, result.as_ref().err().cloned()));
    let state = publish(app);
    result.map(|_| state)
}
pub fn set_visible(app: &tauri::AppHandle, visible: bool) -> Result<CompanionState, String> {
    app.state::<std::sync::Arc<crate::app::DesktopApp>>()
        .companion()
        .visibility_intent
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if !visible {
        input::set_input(app, false)?;
    }
    model(app, |m| m.request_visible(visible));
    let retry = visible
        && (app.get_webview_window("companion").is_none()
            || model(app, |m| !m.initialized && m.view.error.is_some()));
    if retry {
        setup_companion(app)?;
        return Ok(snapshot(app));
    }
    apply_visibility(app)
}
pub fn ready(app: &tauri::AppHandle) -> Result<CompanionState, String> {
    model(app, |m| {
        m.initialize(m.generation);
    });
    apply_visibility(app)
}
pub fn failed(app: &tauri::AppHandle) -> Result<CompanionState, String> {
    let visible = app
        .get_webview_window("companion")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    let _ = input::set_input(app, false);
    model(app, |m| m.fail(visible));
    Ok(publish(app))
}
pub fn set_paused(app: &tauri::AppHandle, paused: bool) -> Result<CompanionState, String> {
    model(app, |m| m.pause(paused));
    Ok(publish(app))
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

pub fn visibility_token(app: &tauri::AppHandle) -> u64 {
    app.state::<std::sync::Arc<crate::app::DesktopApp>>()
        .companion()
        .visibility_intent
        .load(std::sync::atomic::Ordering::SeqCst)
}
pub fn show_for_selection(app: &tauri::AppHandle, token: u64) -> Result<(), String> {
    if visibility_token(app) == token {
        set_visible(app, true)?;
    }
    Ok(())
}

pub fn republish(app: &tauri::AppHandle) {
    publish(app);
}
pub fn present(
    app: &tauri::AppHandle,
    presence: SurfacePresence,
    appearance: Option<SurfaceAppearance>,
) {
    model(app, |model| model.present(presence, appearance));
    publish(app);
}
