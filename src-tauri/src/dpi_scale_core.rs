//! The arithmetic of the Windows display scale, apart from the calls (dpi_scale.rs), so it is tested without a display.
//!
//! Windows keeps a per-display scale as a step on a fixed list of percentages (`DPI_VALS`; there is no 275). For a display it reports three numbers, all RELATIVE to the step it recommends for that
//! display (DisplayConfigGetDeviceInfo, the undocumented DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE = -3): the lowest step it allows, the one in use, the highest it allows. The recommended step is
//! the one the relative numbers count from, so it sits `-min_rel` steps above the first entry of the list. A step beyond the highest needs custom scaling and a sign-out, so it is not on offer.
//! Setting is the same packet type with a relative step (SET_DPI_SCALE = -4).

/// The percentages Windows can show as a scale, in order.
pub const DPI_VALS: [u32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

/// What a display allows now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleRange {
    /// the scale in use
    pub current: u32,
    /// the step Windows recommends for the display's size and density
    pub recommended: u32,
    /// every step that can be applied without signing out, ascending
    pub options: Vec<u32>,
}

impl ScaleRange {
    pub fn min(&self) -> u32 {
        self.options.first().copied().unwrap_or(self.current)
    }
    pub fn max(&self) -> u32 {
        self.options.last().copied().unwrap_or(self.current)
    }
}

/// The range from the three relative numbers the system reports. None when they do not describe a range (the recommended step would lie outside the list).
pub fn range_from_rel(min_rel: i32, cur_rel: i32, max_rel: i32) -> Option<ScaleRange> {
    let last = DPI_VALS.len() as i32 - 1;
    // `min_rel` is -(index of the recommended step): the lowest allowed step is the first of the list
    let recommended_at = min_rel.checked_neg()?;
    if !(0..=last).contains(&recommended_at) || max_rel < 0 {
        return None;
    }
    let top = (recommended_at + max_rel).min(last);
    let cur_at = (recommended_at + cur_rel.clamp(min_rel, max_rel)).clamp(0, top);
    Some(ScaleRange { current: DPI_VALS[cur_at as usize], recommended: DPI_VALS[recommended_at as usize], options: DPI_VALS[..=top as usize].to_vec() })
}

/// The relative step to send for a percentage on a display whose lowest step is `min_rel` (the recommended step's negated index). None when the percentage is not on the list.
pub fn rel_for(min_rel: i32, percent: u32) -> Option<i32> {
    let at = DPI_VALS.iter().position(|v| *v == percent)? as i32;
    Some(at + min_rel)
}

/// What a request comes to on a display that allows `options`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub requested: u32,
    pub applied: u32,
    /// the request was beyond what the display allows (or not a step at all): `applied` is what is set instead
    pub clamped: bool,
}

/// The step to set for a request: the request itself when the display allows it, else the nearest allowed step (the lower one on a tie). None when the display has no step at all.
pub fn resolve(requested: u32, options: &[u32]) -> Option<Resolved> {
    let applied = options.iter().copied().min_by_key(|o| (o.abs_diff(requested), *o))?;
    Some(Resolved { requested, applied, clamped: applied != requested })
}

/// The answer of the command that sets the scale (the page says it once when the request was clamped).
pub fn outcome_json(resolved: Resolved, max: u32) -> serde_json::Value {
    serde_json::json!({ "requested": resolved.requested, "applied": resolved.applied, "clamped": resolved.clamped, "max": max })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_display_that_recommends_150_and_allows_up_to_200() {
        // 100 125 [150] 175 200: the recommended step is index 2, so min_rel = -2; the highest is two above it
        let r = range_from_rel(-2, 0, 2).unwrap();
        assert_eq!(r.options, vec![100, 125, 150, 175, 200]);
        assert_eq!((r.current, r.recommended, r.min(), r.max()), (150, 150, 100, 200));
    }

    #[test]
    fn a_phone_sized_display_stops_at_200() {
        // recommended 100, allowed up to 200: min_rel 0, max_rel 4
        let r = range_from_rel(0, 3, 4).unwrap();
        assert_eq!(r.options, vec![100, 125, 150, 175, 200]);
        assert_eq!(r.current, 175);
        assert_eq!(r.max(), 200);
        assert!(!r.options.contains(&250), "250 needs custom scaling and a sign-out");
    }

    #[test]
    fn the_list_has_no_275() {
        let r = range_from_rel(-2, 0, 9).unwrap();
        assert!(r.options.contains(&250) && r.options.contains(&300) && !r.options.contains(&275));
        assert_eq!(*r.options.last().unwrap(), 500, "the list ends at its last entry, however high the system says");
    }

    #[test]
    fn the_current_step_is_kept_inside_the_range() {
        assert_eq!(range_from_rel(-2, 5, 2).unwrap().current, 200, "above the highest: the highest");
        assert_eq!(range_from_rel(-2, -5, 2).unwrap().current, 100, "below the lowest: the lowest");
    }

    #[test]
    fn numbers_that_are_not_a_range_give_none() {
        assert!(range_from_rel(1, 0, 2).is_none(), "a recommended step above the list's start has a negative index");
        assert!(range_from_rel(-12, 0, 2).is_none(), "recommended index past the end of the list");
        assert!(range_from_rel(i32::MIN, 0, 2).is_none());
        assert!(range_from_rel(0, 0, -1).is_none());
    }

    #[test]
    fn the_relative_step_counts_from_the_recommended_one() {
        // recommended 150 (index 2, min_rel -2)
        assert_eq!(rel_for(-2, 150), Some(0));
        assert_eq!(rel_for(-2, 100), Some(-2));
        assert_eq!(rel_for(-2, 200), Some(2));
        assert_eq!(rel_for(-2, 300), Some(5));
        assert_eq!(rel_for(0, 125), Some(1));
        assert_eq!(rel_for(-2, 275), None, "not a step Windows has");
        assert_eq!(rel_for(-2, 0), None);
    }

    #[test]
    fn a_request_beyond_the_range_is_clamped_to_its_highest_step() {
        let options = [100, 125, 150, 175, 200];
        let r = resolve(250, &options).unwrap();
        assert_eq!(r, Resolved { requested: 250, applied: 200, clamped: true });
        assert_eq!(resolve(300, &options).unwrap().applied, 200);
        assert_eq!(resolve(75, &options).unwrap().applied, 100);
    }

    #[test]
    fn a_request_inside_the_range_is_kept() {
        let options = [100, 125, 150, 175, 200];
        assert_eq!(resolve(175, &options).unwrap(), Resolved { requested: 175, applied: 175, clamped: false });
    }

    #[test]
    fn a_value_between_two_steps_goes_to_the_nearer_one_and_the_lower_on_a_tie() {
        let options = [100, 125, 150, 175, 200, 225, 250, 300];
        assert_eq!(resolve(275, &options).unwrap().applied, 250, "275 is 25 from 250 and 25 from 300: the lower");
        assert!(resolve(275, &options).unwrap().clamped, "it was not what was asked for");
        assert_eq!(resolve(140, &options).unwrap().applied, 150);
    }

    #[test]
    fn a_display_with_no_step_resolves_nothing() {
        assert!(resolve(100, &[]).is_none());
    }

    #[test]
    fn the_outcome_carries_what_the_page_needs_to_say_it_once() {
        let v = outcome_json(resolve(250, &[100, 125, 150, 175, 200]).unwrap(), 200);
        assert_eq!(v["clamped"], true);
        assert_eq!((v["requested"].as_u64(), v["applied"].as_u64(), v["max"].as_u64()), (Some(250), Some(200), Some(200)));
    }
}

/// How long a change of the monitor's scale must stay quiet before the window is given its size back (ms): a display that flips between two scales (a virtual display that follows the client's
/// resolution) is laid out once, for the last scale.
pub const KEEP_DIP_DEBOUNCE_MS: u32 = 1500;

/// A size in pixels as DIPs (device independent pixels, 96 dpi) at a monitor DPI.
pub fn dip_of_pixels(pixels: i32, dpi: u32) -> f64 {
    f64::from(pixels.max(0)) * 96.0 / f64::from(dpi.max(1))
}

/// The size in pixels a window of `dip` DIPs has at `dpi`, never larger than the work area of its monitor (`work`, pixels) and never under 1 pixel.
pub fn pixels_keeping_dip(dip: (f64, f64), dpi: u32, work: (i32, i32)) -> (i32, i32) {
    let scale = f64::from(dpi.max(1)) / 96.0;
    let fit = |dips: f64, room: i32| ((dips * scale).round() as i32).clamp(1, room.max(1));
    (fit(dip.0, work.0), fit(dip.1, work.1))
}

/// The top-left corner that keeps a window of `size` pixels inside the work area `[left, top, right, bottom]`, as near to where it is (`at`) as it can.
pub fn corner_inside(at: (i32, i32), size: (i32, i32), work: (i32, i32, i32, i32)) -> (i32, i32) {
    let x = at.0.min(work.2 - size.0).max(work.0);
    let y = at.1.min(work.3 - size.1).max(work.1);
    (x, y)
}

#[cfg(test)]
mod keep_dip_tests {
    use super::*;

    #[test]
    fn a_size_in_dips_survives_a_change_of_scale() {
        // 1100 x 570 DIPs at 175 % is 1925 x 998 pixels; the same DIPs at 300 % are 3300 x 1710
        assert_eq!(pixels_keeping_dip((1100.0, 570.0), 168, (3840, 2160)), (1925, 998));
        assert_eq!(pixels_keeping_dip((1100.0, 570.0), 288, (3840, 2160)), (3300, 1710));
        assert!((dip_of_pixels(1956, 168) - 1117.7).abs() < 0.1);
        assert!((dip_of_pixels(1956, 288) - 652.0).abs() < 0.1);
    }

    #[test]
    fn the_work_area_is_the_limit() {
        assert_eq!(pixels_keeping_dip((1100.0, 570.0), 288, (2560, 1400)), (2560, 1400));
        assert_eq!(pixels_keeping_dip((0.0, 0.0), 96, (1920, 1080)), (1, 1));
        assert_eq!(pixels_keeping_dip((800.0, 600.0), 96, (0, 0)), (1, 1), "a work area that is not known does not make a window of 0 pixels");
    }

    #[test]
    fn the_corner_keeps_the_window_inside() {
        assert_eq!(corner_inside((100, 50), (800, 600), (0, 0, 1920, 1080)), (100, 50));
        assert_eq!(corner_inside((1500, 700), (800, 600), (0, 0, 1920, 1080)), (1120, 480));
        assert_eq!(corner_inside((-50, -20), (800, 600), (0, 0, 1920, 1080)), (0, 0));
    }
}
