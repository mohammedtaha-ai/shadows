mod no_activate;
mod placement;
pub mod state;

use state::{CompanionState, Model};
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct Store {
    model: Mutex<Model>,
    origin: Mutex<Option<(i32, i32)>>,
    toggle: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    rebuilding: Mutex<bool>,
}
fn model<T>(app: &tauri::AppHandle, f: impl FnOnce(&mut Model) -> T) -> T {
    f(&mut app
        .state::<Store>()
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
    let toggle = app.state::<Store>().toggle.lock().unwrap().clone();
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
    *app.state::<Store>().toggle.lock().unwrap() = Some(item);
    publish(app);
}
fn page_started(app: &tauri::AppHandle) {
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
fn fit(app: &tauri::AppHandle, initial: bool) -> Result<(), String> {
    let w = app
        .get_webview_window("companion")
        .ok_or("companion_unavailable")?;
    let size = w.outer_size().map_err(|_| "companion_position_failed")?;
    let mut monitors = w
        .available_monitors()
        .map_err(|_| "companion_position_failed")?;
    if let Ok(Some(primary)) = w.primary_monitor() {
        let position = primary.position();
        monitors.sort_by_key(|m| m.position() != position);
    }
    let areas: Vec<_> = monitors
        .iter()
        .map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width, a.size.height)
        })
        .collect();
    let origin = if initial {
        *app.state::<Store>().origin.lock().unwrap()
    } else {
        w.outer_position().ok().map(|p| (p.x, p.y))
    };
    if let Some((x, y, width, height)) =
        placement::fit_companion((size.width, size.height), origin, &areas)
    {
        if (width, height) != (size.width, size.height) {
            w.set_size(tauri::PhysicalSize::new(width, height))
                .map_err(|_| "companion_position_failed")?;
        }
        if w.outer_position().ok().map(|p| (p.x, p.y)) != Some((x, y)) {
            w.set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(|_| "companion_position_failed")?;
        }
        *app.state::<Store>().origin.lock().unwrap() = Some((x, y));
    }
    Ok(())
}
pub fn setup_companion(app: &tauri::AppHandle) -> Result<(), String> {
    let result = (|| {
        if let Some(old) = app.get_webview_window("companion") {
            model(app, |m| {
                m.initialized = false;
                m.finish(false, None);
            });
            publish(app);
            *app.state::<Store>().rebuilding.lock().unwrap() = true;
            old.destroy().map_err(|_| "companion_unavailable")?;
            return Ok(());
        }
        model(app, |m| {
            m.initialized = false;
            m.deny_render();
        });
        let w =
            WebviewWindowBuilder::new(app, "companion", WebviewUrl::App("companion.html".into()))
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
                })
                .build()
                .map_err(|_| "companion_unavailable")?;
        let owner = app.clone();
        w.on_window_event(move |event| match event {
            tauri::WindowEvent::Destroyed => {
                let rebuild =
                    std::mem::take(&mut *owner.state::<Store>().rebuilding.lock().unwrap());
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
