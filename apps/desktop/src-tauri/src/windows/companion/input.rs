// Native focus and bounds for the explicit compact editor.
use super::{model, placement, publish, snapshot, state::CompanionState, Store};
use tauri::Manager;

#[derive(Default)]
pub(super) struct Placement {
    closed: Option<(i32, i32)>,
    opened: Option<(i32, i32)>,
    return_focus: Option<(isize, u32)>,
}
pub fn open_bounds(closed: (i32, i32), scale: f64) -> (i32, i32, u32, u32) {
    (
        closed.0 - (28.0 * scale).round() as i32,
        closed.1,
        (296.0 * scale).round() as u32,
        (320.0 * scale).round() as u32,
    )
}
pub fn closed_origin(open: (i32, i32), scale: f64) -> (i32, i32) {
    (open.0 + (28.0 * scale).round() as i32, open.1)
}
pub(super) fn fit(app: &tauri::AppHandle, initial: bool) -> Result<(), String> {
    let w = app
        .get_webview_window("companion")
        .ok_or("companion_unavailable")?;
    let size = w.outer_size().map_err(|_| "companion_position_failed")?;
    let mut monitors = w
        .available_monitors()
        .map_err(|_| "companion_position_failed")?;
    let preferred = w
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(preferred) = preferred {
        monitors.sort_by_key(|m| m.position() != preferred.position());
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
pub fn refresh(app: &tauri::AppHandle) -> Result<CompanionState, String> {
    let owner = app.state::<std::sync::Arc<crate::presence::Presence>>();
    let (scope, context) = owner.context();
    let agent = owner.view().agent;
    if owner.context() != (scope.clone(), context.clone()) {
        return Ok(snapshot(app));
    }
    if snapshot(app).composer.scope != scope {
        set_input(app, false)?;
    }
    model(app, |m| m.composer_context(scope, context, agent));
    Ok(publish(app))
}
pub fn set_input(app: &tauri::AppHandle, open: bool) -> Result<CompanionState, String> {
    let store = app.state::<Store>();
    let current = snapshot(app);
    if current.composer.open == open {
        return Ok(current);
    }
    let w = app
        .get_webview_window("companion")
        .ok_or("companion_unavailable")?;
    if open && (!current.visible || !current.render_allowed || current.error.is_some()) {
        return Err("companion_input_unavailable".into());
    }
    let scale = w.scale_factor().map_err(|_| "companion_position_failed")?;
    let pos = w
        .outer_position()
        .map_err(|_| "companion_position_failed")?;
    if open {
        let rect = open_bounds((pos.x, pos.y), scale);
        {
            let mut placement = store.input.lock().unwrap();
            placement.closed = Some((pos.x, pos.y));
            placement.return_focus = super::no_activate::previous_foreground(&w);
        }
        let result = (|| {
            w.set_size(tauri::PhysicalSize::new(rect.2, rect.3))
                .map_err(|_| "companion_position_failed")?;
            w.set_position(tauri::PhysicalPosition::new(rect.0, rect.1))
                .map_err(|_| "companion_position_failed")?;
            fit(app, false)?;
            let fitted = w
                .outer_position()
                .map_err(|_| "companion_position_failed")?;
            store.input.lock().unwrap().opened = Some((fitted.x, fitted.y));
            w.set_focusable(true)
                .map_err(|_| "companion_focus_failed")?;
            // Background show bypasses Tao's cached visibility. Explicit open may
            // synchronize it: otherwise Tao set_focus silently treats us as hidden.
            w.show().map_err(|_| "companion_focus_failed")?;
            w.set_focus().map_err(|_| "companion_focus_failed")?;
            Ok::<_, String>(())
        })();
        if result.is_err() {
            let _ = w.set_focusable(false);
            let _ = w.set_size(tauri::LogicalSize::new(240.0, 240.0));
            let _ = w.set_position(pos);
            return result.map(|_| current);
        }
    } else {
        w.set_focusable(false)
            .map_err(|_| "companion_focus_failed")?;
        let origin = {
            let placement = store.input.lock().unwrap();
            if placement.opened == Some((pos.x, pos.y)) {
                placement
                    .closed
                    .unwrap_or_else(|| closed_origin((pos.x, pos.y), scale))
            } else {
                closed_origin((pos.x, pos.y), scale)
            }
        };
        w.set_size(tauri::LogicalSize::new(240.0, 240.0))
            .map_err(|_| "companion_position_failed")?;
        w.set_position(tauri::PhysicalPosition::new(origin.0, origin.1))
            .map_err(|_| "companion_position_failed")?;
        fit(app, false)?;
        let previous = store.input.lock().unwrap().return_focus.take();
        super::no_activate::restore_foreground(&w, previous);
    }
    model(app, |m| m.input(open));
    Ok(publish(app))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_bounds_centres_the_character() {
        assert_eq!(open_bounds((1664, 784), 1.0), (1636, 784, 296, 320));
        assert_eq!(open_bounds((1664, 784), 1.5), (1622, 784, 444, 480));
    }
    #[test]
    fn open_then_close_restores_exact_origin() {
        for (x, y, scale) in [(1664, 784, 1.0), (-500, 300, 1.25), (0, 0, 2.0)] {
            let rect = open_bounds((x, y), scale);
            assert_eq!(closed_origin((rect.0, rect.1), scale), (x, y));
        }
    }
    #[test]
    fn open_at_bottom_right_edge_fits_whole_rectangle() {
        let rect = open_bounds((1664, 784), 1.0);
        let fitted = super::super::placement::fit_companion(
            (rect.2, rect.3),
            Some((rect.0, rect.1)),
            &[(0, 0, 1920, 1040)],
        )
        .unwrap();
        assert_eq!(fitted, (1608, 704, 296, 320));
        let negative = super::super::placement::fit_companion(
            (296, 320),
            Some((-500, 300)),
            &[(-1920, 0, 1920, 1080)],
        )
        .unwrap();
        assert_eq!(negative, (-500, 300, 296, 320));
    }
}
