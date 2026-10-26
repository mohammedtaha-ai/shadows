use std::{path::PathBuf, sync::Arc};
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
pub mod companion;
mod main_policy;
mod placement;

pub struct MainWindow {
    configured_origin: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    data_directory: Option<PathBuf>,
    browser_args: Option<String>,
}

impl MainWindow {
    pub fn new(
        configured_origin: Arc<dyn Fn() -> Option<String> + Send + Sync>,
        data_directory: Option<PathBuf>,
        browser_args: Option<String>,
    ) -> Self {
        Self {
            configured_origin,
            data_directory,
            browser_args,
        }
    }

    fn build(&self, app: &tauri::App) -> Result<tauri::WebviewWindow, tauri::Error> {
        main_policy::build(
            app,
            self.configured_origin.clone(),
            self.data_directory.clone(),
            self.browser_args.as_deref(),
        )
    }

    // Foreground activation is reserved for explicit tray, shortcut and launch actions.
    // Opening reads only the live "main" window, so it needs no managed-state lookup.
    pub fn open(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
        let w = app
            .get_webview_window("main")
            .expect("main exists for app lifetime");
        w.unminimize()?;
        w.show()?;
        // A display change can strand a previously placed window. Recover only if it
        // no longer intersects a current monitor. Physical cases still need evidence.
        let pos = w.outer_position()?;
        let mut size = w.outer_size()?;
        let monitors = w.available_monitors()?;
        if let Some(monitor) = w.current_monitor()?.or(w.primary_monitor()?) {
            let area = monitor.work_area();
            let inner = w.inner_size()?;
            let fitted = placement::fit_inner(
                (inner.width, inner.height),
                (size.width, size.height),
                (area.size.width, area.size.height),
            );
            if fitted != (inner.width, inner.height) {
                w.set_size(tauri::PhysicalSize::new(fitted.0, fitted.1))?;
                size = w.outer_size()?;
            }
        }
        let areas: Vec<_> = monitors
            .iter()
            .map(|m| {
                let a = m.work_area();
                (a.position.x, a.position.y, a.size.width, a.size.height)
            })
            .collect();
        if let Some((x, y)) =
            placement::recover_origin((pos.x, pos.y, size.width, size.height), &areas)
        {
            w.set_position(tauri::PhysicalPosition::new(x, y))?;
        }
        w.set_focus()
    }
}

pub fn open_main(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
    MainWindow::open(app)
}

pub struct CompanionActions {
    pub attach_toggle: Box<dyn FnOnce(tauri::menu::MenuItem<tauri::Wry>)>,
    pub toggle: Arc<dyn Fn(&tauri::AppHandle) + Send + Sync>,
    pub setup: Box<dyn FnOnce(&tauri::AppHandle)>,
}

pub fn setup(
    app: &mut tauri::App,
    main: &MainWindow,
    close_action: Arc<dyn Fn() -> crate::desktop_preferences::CloseAction + Send + Sync>,
    actions: CompanionActions,
) -> Result<(), Box<dyn std::error::Error>> {
    let w = main.build(app)?;
    let h = w.clone();
    w.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            match close_action() {
                crate::desktop_preferences::CloseAction::Hide => {
                    let _ = h.hide();
                }
                crate::desktop_preferences::CloseAction::Exit => h.app_handle().exit(0),
            }
        }
    });
    let show = tauri::menu::MenuItem::with_id(app, "show", "Open NYLRA", true, None::<&str>)?;
    let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit NYLRA", true, None::<&str>)?;
    let toggle =
        tauri::menu::MenuItem::with_id(app, "companion-toggle", "Show AYLUL", true, None::<&str>)?;
    (actions.attach_toggle)(toggle.clone());
    let menu = tauri::menu::Menu::with_items(app, &[&show, &toggle, &quit])?;
    // Neutral application icon, not substitute character artwork.
    let mut pixels = vec![0u8; 16 * 16 * 4];
    for p in pixels.as_chunks_mut::<4>().0 {
        p.copy_from_slice(&[168, 116, 225, 255]);
    }
    tauri::tray::TrayIconBuilder::with_id("nylra")
        .icon(tauri::image::Image::new_owned(pixels, 16, 16))
        .tooltip("NYLRA")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let _ = open_main(tray.app_handle());
            }
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                let _ = open_main(app);
            }
            "quit" => app.exit(0),
            "companion-toggle" => {
                (actions.toggle)(app);
            }
            _ => (),
        })
        .build(app)?;
    app.global_shortcut().register("Ctrl+Alt+Shift+F12")?;
    open_main(app.handle())?;
    (actions.setup)(app.handle());
    Ok(())
}
