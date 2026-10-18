// Physical geometry only. Device removal/mixed DPI still need hardware evidence.
pub fn fit_inner(inner: (u32, u32), outer: (u32, u32), area: (u32, u32)) -> (u32, u32) {
    (
        inner
            .0
            .min(area.0.saturating_sub(64 + outer.0.saturating_sub(inner.0)))
            .max(1),
        inner
            .1
            .min(area.1.saturating_sub(64 + outer.1.saturating_sub(inner.1)))
            .max(1),
    )
}
pub fn recover_origin(
    window: (i32, i32, u32, u32),
    areas: &[(i32, i32, u32, u32)],
) -> Option<(i32, i32)> {
    let (x, y, w, h) = window;
    let x = x as i64;
    let y = y as i64;
    let usable = areas.iter().any(|&(ax, ay, aw, ah)| {
        let left = x.max(ax as i64);
        let right = (x + w as i64).min(ax as i64 + aw as i64);
        right - left >= 120.min(w) as i64 && y >= ay as i64 && y + 32 <= ay as i64 + ah as i64
    });
    if usable {
        return None;
    }
    let &(ax, ay, aw, ah) = areas.first()?;
    if aw == 0 || ah == 0 {
        return None;
    }
    let left = ax as i64 + 32.min(aw / 2) as i64;
    let top = ay as i64 + 32.min(ah / 2) as i64;
    let right = (ax as i64 + aw as i64 - w as i64 - 32).max(left);
    let bottom = (ay as i64 + ah as i64 - h as i64 - 32).max(top);
    Some((x.clamp(left, right) as i32, y.clamp(top, bottom) as i32))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dpi_enlarged_window_fits_physical_work_area() {
        assert_eq!(
            fit_inner((1080, 960), (1102, 1016), (1920, 1008)),
            (1080, 888)
        );
        assert_eq!(
            fit_inner((1440, 1280), (1472, 1358), (1920, 984)),
            (1440, 842)
        );
    }
    #[test]
    fn lost_window_and_one_pixel_sliver_recover_a_usable_title_bar() {
        let areas = [(0, 0, 1920, 1032)];
        assert_eq!(
            recover_origin((4000, 200, 720, 640), &areas),
            Some((1168, 200))
        );
        assert_eq!(
            recover_origin((1919, 200, 720, 640), &areas),
            Some((1168, 200))
        );
    }
    #[test]
    fn accessible_window_on_negative_monitor_stays_put() {
        assert_eq!(
            recover_origin(
                (-1500, 100, 720, 640),
                &[(-1920, 0, 1920, 1080), (0, 0, 1920, 1032)]
            ),
            None
        );
    }
}
