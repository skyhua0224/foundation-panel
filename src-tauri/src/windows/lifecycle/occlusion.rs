//! Whether the window is covered by other windows.
//!
//! WebView2 does not report a covered window as hidden (its page keeps `visibilityState` "visible" and its timers run), so the panel
//! looks for itself: a few points of the window are asked which top-level window is at that spot. The window counts as covered when
//! none of them is its own; one point that shows the window is enough to keep it running. A window nothing can be asked about (off every
//! screen) is not covered, and a window that lets clicks through (an overlay) is not seen by `WindowFromPoint`, so it covers nothing.
//! Covered for two samples in a row pauses the page, any sample that shows the window, or the window being activated, resumes it.

/// Samples in a row that say "covered" before the page is paused.
pub(super) const COVERED_SAMPLES: u8 = 2;

/// The step from one sample to the next: `Some(true)` when the page is to be paused, `Some(false)` when it is to run again.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Occlusion {
    covered_in_a_row: u8,
    pub(super) occluded: bool,
}

impl Occlusion {
    pub(super) fn observe(&mut self, covered: bool) -> Option<bool> {
        if covered {
            self.covered_in_a_row = (self.covered_in_a_row + 1).min(COVERED_SAMPLES);
            if self.covered_in_a_row >= COVERED_SAMPLES && !self.occluded {
                self.occluded = true;
                return Some(true);
            }
            None
        } else {
            self.covered_in_a_row = 0;
            if self.occluded {
                self.occluded = false;
                return Some(false);
            }
            None
        }
    }

    /// The window was activated: it is on top of everything.
    pub(super) fn clear(&mut self) -> bool {
        self.covered_in_a_row = 0;
        std::mem::take(&mut self.occluded)
    }
}

/// The 5 x 5 points asked, inside the window's rectangle (a margin keeps the shadow and the resize border out).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(super) fn sample_points(left: i32, top: i32, right: i32, bottom: i32) -> Vec<(i32, i32)> {
    const MARGIN: i32 = 8;
    const GRID: i32 = 5;
    let (left, top, right, bottom) = (left + MARGIN, top + MARGIN, right - MARGIN, bottom - MARGIN);
    if right - left < 40 || bottom - top < 40 {
        return Vec::new();
    }
    (0..GRID)
        .flat_map(|row| {
            (0..GRID).map(move |column| {
                (
                    left + (right - left) * (2 * column + 1) / (2 * GRID),
                    top + (bottom - top) * (2 * row + 1) / (2 * GRID),
                )
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
pub(super) fn covered<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> bool {
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetWindowRect, WindowFromPoint};

    let Ok(raw) = window.hwnd() else {
        return false;
    };
    let hwnd = HWND(raw.0);
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return false;
    }
    let points = sample_points(rect.left, rect.top, rect.right, rect.bottom);
    if points.is_empty() {
        return false;
    }
    points.into_iter().all(|(x, y)| {
        let hit = unsafe { WindowFromPoint(POINT { x, y }) };
        // nothing there (off every screen), or the window itself (or a child of it) shows at this point
        !hit.is_invalid() && unsafe { GetAncestor(hit, GA_ROOT) } != hwnd
    })
}

#[cfg(not(target_os = "windows"))]
pub(super) fn covered<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_covered_samples_in_a_row_pause_and_one_visible_sample_resumes() {
        let mut state = Occlusion::default();
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(false), None, "one covered sample is not enough");
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(true), Some(true));
        assert_eq!(state.observe(true), None, "already paused");
        assert_eq!(state.observe(false), Some(false));
        assert_eq!(state.observe(false), None);
    }

    #[test]
    fn activating_the_window_resumes_at_once() {
        let mut state = Occlusion::default();
        state.observe(true);
        state.observe(true);
        assert!(state.occluded);
        assert!(state.clear());
        assert!(!state.occluded);
        assert!(!state.clear(), "nothing to resume the second time");
        // and the count starts over
        assert_eq!(state.observe(true), None);
    }

    #[test]
    fn the_sample_points_lie_inside_the_window_and_a_tiny_window_has_none() {
        let points = sample_points(100, 200, 1380, 1000);
        assert_eq!(points.len(), 25);
        assert!(points.iter().all(|&(x, y)| (108..1372).contains(&x) && (208..992).contains(&y)));
        let columns: std::collections::BTreeSet<_> = points.iter().map(|p| p.0).collect();
        assert_eq!(columns.len(), 5);
        assert!(sample_points(0, 0, 50, 50).is_empty());
    }
}
