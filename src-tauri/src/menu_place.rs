//! Where the menu window of the context menu goes (ctx_popup.rs): the arithmetic only, in physical pixels of the virtual desktop, so that it is tested without a window.
//!
//! A native menu opens with its top-left corner at the pointer; when it does not fit it opens to the left of the pointer and/or above it (its bottom-right corner at the pointer); a menu that fits in
//! neither direction ends at the edge, and one that is taller than the work area scrolls inside it. The edge is that of the WORK AREA of the monitor under the pointer (the screen without the taskbar),
//! not that of the window the menu belongs to: the menu may extend beyond its window. This is the same rule as `placeMenu` of the in-page menu (foundation-ui src/contextmenu/place.js), which keeps
//! the viewport of a page as its edge.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
    /// The rectangle grown by `by` on every side.
    pub fn grow(&self, by: i32) -> Self {
        Self::new(self.x - by, self.y - by, self.w + 2 * by, self.h + 2 * by)
    }
    fn contains(&self, p: (i32, i32)) -> bool {
        p.0 >= self.x && p.0 < self.right() && p.1 >= self.y && p.1 < self.bottom()
    }
    /// Squared distance from a point to the rectangle (0 inside).
    fn distance2(&self, p: (i32, i32)) -> i64 {
        let dx = (self.x - p.0).max(0).max(p.0 - (self.right() - 1)) as i64;
        let dy = (self.y - p.1).max(0).max(p.1 - (self.bottom() - 1)) as i64;
        dx * dx + dy * dy
    }
}

/// One monitor: the work area (physical pixels of the virtual desktop) and the scale factor that maps CSS pixels to physical ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorInfo {
    pub work: Rect,
    pub scale: f64,
}

/// The monitor of a point: the one whose work area holds it; for a point in the taskbar, in a gap between monitors or outside every one, the nearest. None only when there is no monitor.
pub fn pick_monitor(monitors: &[MonitorInfo], point: (i32, i32)) -> Option<usize> {
    if let Some(i) = monitors.iter().position(|m| m.work.contains(point)) {
        return Some(i);
    }
    monitors
        .iter()
        .enumerate()
        .min_by_key(|(_, m)| m.work.distance2(point))
        .map(|(i, _)| i)
}

/// A scale factor that can be used: finite and positive, else 1.
pub fn sane_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }
}

/// CSS pixels to physical pixels, rounded up (a menu is never cut by a fraction of a pixel).
pub fn css_to_physical(css: f64, scale: f64) -> i32 {
    (css * sane_scale(scale)).ceil() as i32
}

/// Physical pixels to CSS pixels, rounded down.
pub fn physical_to_css(physical: i32, scale: f64) -> f64 {
    (physical as f64 / sane_scale(scale)).floor()
}

/// One axis: the start of a menu of `size` for a pointer at `point` inside [start, start + extent), with `margin` kept free at both ends. Returns (start, size, capped).
fn axis(point: i32, size: i32, start: i32, extent: i32, margin: i32) -> (i32, i32, bool) {
    let low = start + margin;
    let high = start + extent - margin; // exclusive end of the free range
    let free = high - low;
    if size >= free {
        return (low, free.max(0), true);
    }
    // below / right of the pointer when it fits there; else above / left of it when it fits there; else flush with the edge it overflows
    if point + size <= high {
        return (point.max(low), size, false);
    }
    if point - size >= low {
        return ((point - size).min(high - size), size, false);
    }
    ((high - size).clamp(low, start + extent), size, false)
}

/// The menu surface, placed. `size` is the surface as measured (physical pixels), `anchor` the pointer, `work` the work area of its monitor, `margin` the room kept to the edge of the work area.
/// The height is cut to the work area when the menu is taller than it (`capped`): the menu scrolls inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub rect: Rect,
    pub capped: bool,
}

pub fn place_surface(anchor: (i32, i32), size: (i32, i32), work: Rect, margin: i32) -> Placed {
    let (x, w, _) = axis(anchor.0, size.0.max(1), work.x, work.w, margin);
    let (y, h, capped) = axis(anchor.1, size.1.max(1), work.y, work.h, margin);
    Placed { rect: Rect::new(x, y, w, h), capped }
}

/// The room a menu may have (CSS pixels): the height of the work area less the margin at both ends, and the same for the width.
pub fn room_css(work: Rect, scale: f64, margin_css: f64) -> (f64, f64) {
    (
        (physical_to_css(work.w, scale) - 2.0 * margin_css).max(0.0),
        (physical_to_css(work.h, scale) - 2.0 * margin_css).max(0.0),
    )
}

/// The screen position of a point of a window's page that was given in CSS pixels (a menu opened by the keyboard has no pointer): the window's client origin plus the point times the page's
/// pixel ratio.
pub fn client_to_screen(origin: (i32, i32), client: (f64, f64), ratio: f64) -> (i32, i32) {
    let r = sane_scale(ratio);
    (origin.0 + (client.0 * r).round() as i32, origin.1 + (client.1 * r).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect::new(0, 0, 1920, 1040); // a 1080 monitor with a 40 px taskbar at the bottom
    const M: i32 = 8;

    fn at(anchor: (i32, i32), size: (i32, i32)) -> Rect {
        place_surface(anchor, size, WORK, M).rect
    }

    #[test]
    fn a_menu_that_fits_opens_at_the_pointer() {
        assert_eq!(at((300, 200), (240, 400)), Rect::new(300, 200, 240, 400));
    }

    #[test]
    fn at_the_right_edge_it_opens_to_the_left_of_the_pointer() {
        // 1900 + 240 > 1920: its right edge is at the pointer
        assert_eq!(at((1900, 200), (240, 400)), Rect::new(1660, 200, 240, 400));
    }

    #[test]
    fn at_the_bottom_edge_it_opens_above_the_pointer() {
        assert_eq!(at((300, 900), (240, 400)), Rect::new(300, 500, 240, 400));
    }

    #[test]
    fn in_the_corner_it_opens_up_and_to_the_left() {
        assert_eq!(at((1919, 1039), (240, 400)), Rect::new(1672, 632, 240, 400)); // the margin keeps it 8 px from both edges
    }

    #[test]
    fn the_edge_is_the_work_area_not_the_screen() {
        // the taskbar takes the bottom 40 px: a pointer at y = 1030 is in the work area's last rows and the menu goes above it, never under the taskbar
        let r = at((300, 1030), (240, 400));
        assert!(r.y + r.h <= WORK.h - M + 1, "ends above the taskbar: {r:?}");
    }

    #[test]
    fn it_fits_neither_way_so_it_ends_at_the_edge() {
        // 700 high in a 1040 work area with the pointer in the middle (520): below does not fit (520+700 > 1032), above does not fit (520-700 < 8)
        assert_eq!(at((300, 520), (240, 700)), Rect::new(300, 332, 240, 700));
    }

    #[test]
    fn a_menu_taller_than_the_work_area_is_cut_and_scrolls() {
        let p = place_surface((300, 500), (240, 2000), WORK, M);
        assert!(p.capped);
        assert_eq!(p.rect, Rect::new(300, M, 240, 1040 - 2 * M));
    }

    #[test]
    fn a_pointer_at_the_left_or_top_edge_keeps_the_margin() {
        assert_eq!(at((0, 0), (240, 400)), Rect::new(8, 8, 240, 400));
    }

    #[test]
    fn it_never_leaves_the_work_area_wherever_the_pointer_is() {
        for (w, h) in [(112, 48), (240, 400), (280, 1000), (300, 5000)] {
            for x in (0..1920).step_by(97) {
                for y in (0..1040).step_by(89) {
                    let p = place_surface((x, y), (w, h), WORK, M);
                    let r = p.rect;
                    assert!(r.x >= WORK.x + M && r.y >= WORK.y + M, "{x},{y} {w}x{h} -> {r:?}");
                    assert!(r.x + r.w <= WORK.right() - M, "{x},{y} {w}x{h} -> {r:?}");
                    assert!(r.y + r.h <= WORK.bottom() - M, "{x},{y} {w}x{h} -> {r:?}");
                }
            }
        }
    }

    // two monitors: a 4K one at 200 % on the left of the primary (negative x), the primary 1080p at 100 % (work area without a 40 px taskbar), a 1440p one at 150 % above it
    fn desk() -> [MonitorInfo; 3] {
        [
            MonitorInfo { work: Rect::new(-3840, 0, 3840, 2120), scale: 2.0 },
            MonitorInfo { work: Rect::new(0, 0, 1920, 1040), scale: 1.0 },
            MonitorInfo { work: Rect::new(0, -1440, 2560, 1400), scale: 1.5 },
        ]
    }

    #[test]
    fn the_monitor_is_the_one_under_the_pointer_also_with_negative_coordinates() {
        let d = desk();
        assert_eq!(pick_monitor(&d, (-100, 500)), Some(0));
        assert_eq!(pick_monitor(&d, (100, 500)), Some(1));
        assert_eq!(pick_monitor(&d, (100, -500)), Some(2));
        assert_eq!(pick_monitor(&d, (-3840, 0)), Some(0));
        assert_eq!(pick_monitor(&d, (0, 0)), Some(1));
    }

    #[test]
    fn a_pointer_in_the_taskbar_or_between_monitors_takes_the_nearest() {
        let d = desk();
        assert_eq!(pick_monitor(&d, (500, 1060)), Some(1), "the taskbar of the primary is not in its work area");
        assert_eq!(pick_monitor(&d, (-3000, 2200)), Some(0));
        assert_eq!(pick_monitor(&d, (2600, -1420)), Some(2));
        assert_eq!(pick_monitor(&[], (0, 0)), None);
    }

    #[test]
    fn a_menu_on_the_left_monitor_is_cut_at_that_monitors_edge_not_the_primary() {
        let d = desk();
        let m = d[0];
        // 200 %: a 280 x 360 CSS menu is 560 x 720 physical, the pointer 20 px from the right edge of the left monitor (x = -20): it opens to the LEFT of the pointer, still on that monitor
        let (w, h) = (css_to_physical(280.0, m.scale), css_to_physical(360.0, m.scale));
        let margin = css_to_physical(8.0, m.scale);
        let r = place_surface((-20, 600), (w, h), m.work, margin).rect;
        assert_eq!(r, Rect::new(-20 - 560, 600, 560, 720));
        assert!(r.x >= m.work.x && r.x + r.w <= m.work.x + m.work.w);
    }

    #[test]
    fn sizes_follow_the_scale_of_the_monitor_the_menu_opens_on() {
        assert_eq!(css_to_physical(280.0, 1.0), 280);
        assert_eq!(css_to_physical(280.0, 1.25), 350);
        assert_eq!(css_to_physical(281.0, 1.5), 422); // 421.5 rounds up
        assert_eq!(css_to_physical(100.0, 0.0), 100, "a broken scale is 1");
        assert_eq!(css_to_physical(100.0, f64::NAN), 100);
        assert_eq!(physical_to_css(1040, 1.5), 693.0);
    }

    #[test]
    fn the_room_of_a_menu_is_the_work_area_less_the_margins_in_css_pixels() {
        let d = desk();
        assert_eq!(room_css(d[1].work, d[1].scale, 8.0), (1904.0, 1024.0));
        assert_eq!(room_css(d[0].work, d[0].scale, 8.0), (1904.0, 1044.0));
        assert_eq!(room_css(Rect::new(0, 0, 10, 10), 1.0, 8.0), (0.0, 0.0));
    }

    #[test]
    fn a_menu_opened_by_the_keyboard_is_placed_from_the_window_origin_and_the_pixel_ratio() {
        assert_eq!(client_to_screen((100, 50), (10.0, 20.0), 1.0), (110, 70));
        assert_eq!(client_to_screen((-1920, 100), (100.5, 40.0), 1.5), (-1769, 160));
        assert_eq!(client_to_screen((0, 0), (10.0, 10.0), 0.0), (10, 10));
    }

    #[test]
    fn the_window_is_the_surface_grown_by_the_room_for_its_shadow() {
        let s = place_surface((300, 200), (240, 400), WORK, M).rect;
        assert_eq!(s.grow(16), Rect::new(284, 184, 272, 432));
    }
}
