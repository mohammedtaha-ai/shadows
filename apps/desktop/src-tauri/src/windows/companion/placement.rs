// Whole physical companion rectangle; main's title-bar recovery is a different rule.
pub fn fit_companion(
    size: (u32, u32),
    origin: Option<(i32, i32)>,
    areas: &[(i32, i32, u32, u32)],
) -> Option<(i32, i32, u32, u32)> {
    let valid: Vec<_> = areas
        .iter()
        .copied()
        .filter(|a| a.2 > 0 && a.3 > 0)
        .collect();
    if let Some((x, y)) = origin {
        if valid.iter().any(|&(ax, ay, aw, ah)| {
            x as i64 >= ax as i64
                && y as i64 >= ay as i64
                && x as i64 + size.0 as i64 <= ax as i64 + aw as i64
                && y as i64 + size.1 as i64 <= ay as i64 + ah as i64
        }) {
            return Some((x, y, size.0, size.1));
        }
    }
    let &(ax, ay, aw, ah) = valid.first()?;
    let mx = 16.min(aw.saturating_sub(1) / 2);
    let my = 16.min(ah.saturating_sub(1) / 2);
    let w = size.0.max(1).min(aw - 2 * mx);
    let h = size.1.max(1).min(ah - 2 * my);
    let right = ax as i64 + aw as i64 - mx as i64 - w as i64;
    let bottom = ay as i64 + ah as i64 - my as i64 - h as i64;
    let (x, y) = origin
        .map(|(x, y)| {
            (
                (x as i64).clamp(ax as i64 + mx as i64, right),
                (y as i64).clamp(ay as i64 + my as i64, bottom),
            )
        })
        .unwrap_or((right, bottom));
    Some((x as i32, y as i32, w, h))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companion_initial_position_uses_work_area_margin() {
        assert_eq!(
            fit_companion((240, 240), None, &[(0, 0, 1920, 1040)]),
            Some((1664, 784, 240, 240))
        );
    }
    #[test]
    fn companion_negative_origin_and_small_or_missing_areas() {
        assert_eq!(
            fit_companion((240, 240), Some((-500, 300)), &[(-1920, 0, 1920, 1080)]),
            Some((-500, 300, 240, 240))
        );
        let fitted = fit_companion((240, 240), None, &[(0, 0, 100, 80)]).unwrap();
        assert!(
            fitted.0 >= 0
                && fitted.1 >= 0
                && fitted.0 as u32 + fitted.2 <= 100
                && fitted.1 as u32 + fitted.3 <= 80
        );
        assert_eq!(
            fit_companion(
                (fitted.2, fitted.3),
                Some((fitted.0, fitted.1)),
                &[(0, 0, 100, 80)]
            ),
            Some(fitted)
        );
        assert_eq!(fit_companion((240, 240), None, &[(0, 0, 0, 0)]), None);
    }
}
