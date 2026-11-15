//! What the panel does when a WebView2 process fails, so the window does not stay blank.
//!
//! Until now a failure was only written to the crash report: the renderer of the window died (`render_exited`, the most frequent failure of the
//! first release: twelve reports from one day) and the page stayed empty until the user closed and opened the panel again.
//!
//! * `render_exited`, `frame_render_exited`: the page is gone, the browser is alive: reload it. At most three times in five minutes; a renderer that
//!   dies again and again (a driver fault) is left alone instead of looping.
//! * `browser_exited`: the whole WebView2 is unusable (the runtime was updated or killed): start the panel again, at most twice in a row.
//! * `render_unresponsive`: the runtime raises `render_exited` itself when it gives up on the process, so there is nothing to do yet.
//! * `gpu_exited`, `utility_exited`, `sandbox_helper_exited`: Chromium starts these again by itself.
//!
//! The decision is a plain function of the failure kind and the recent history: the part that touches the window is in `win.rs`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Reloads allowed in `RELOAD_WINDOW`.
const MAX_RELOADS: usize = 3;
const RELOAD_WINDOW: Duration = Duration::from_secs(5 * 60);
/// Restarts in a row (the count is carried to the new process in `RESTARTS_ENV`).
const MAX_RESTARTS: u32 = 2;
pub(crate) const RESTARTS_ENV: &str = "SUNSHINE_PANEL_WEBVIEW_RESTARTS";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Action {
    Nothing,
    Reload,
    Restart,
}

#[derive(Default)]
pub(crate) struct History {
    reloads: VecDeque<Instant>,
}

/// The value of `RESTARTS_ENV` of this process (0 when it is not set or not a number).
pub(crate) fn restarts_so_far(value: Option<&str>) -> u32 {
    value.and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// `kind` is the name the crash report uses (`render_exited`, `browser_exited`, ...).
pub(crate) fn decide(kind: &str, now: Instant, history: &mut History, restarts: u32) -> Action {
    match kind {
        "render_exited" | "frame_render_exited" => {
            while history.reloads.front().is_some_and(|t| now.duration_since(*t) > RELOAD_WINDOW) {
                history.reloads.pop_front();
            }
            if history.reloads.len() >= MAX_RELOADS {
                return Action::Nothing;
            }
            history.reloads.push_back(now);
            Action::Reload
        }
        "browser_exited" if restarts < MAX_RESTARTS => Action::Restart,
        _ => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dead_renderer_is_reloaded_three_times_in_five_minutes() {
        let mut history = History::default();
        let t0 = Instant::now();
        for i in 0..3 {
            assert_eq!(decide("render_exited", t0 + Duration::from_secs(i * 10), &mut history, 0), Action::Reload);
        }
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(40), &mut history, 0), Action::Nothing);
        assert_eq!(decide("frame_render_exited", t0 + Duration::from_secs(60), &mut history, 0), Action::Nothing);
        // after the window the first one has aged out
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(5 * 60 + 5), &mut history, 0), Action::Reload);
    }

    #[test]
    fn a_dead_browser_restarts_the_panel_twice_in_a_row() {
        let mut history = History::default();
        let now = Instant::now();
        assert_eq!(decide("browser_exited", now, &mut history, 0), Action::Restart);
        assert_eq!(decide("browser_exited", now, &mut history, 1), Action::Restart);
        assert_eq!(decide("browser_exited", now, &mut history, 2), Action::Nothing);
    }

    #[test]
    fn what_chromium_restarts_by_itself_is_left_alone() {
        let mut history = History::default();
        let now = Instant::now();
        for kind in ["gpu_exited", "utility_exited", "sandbox_helper_exited", "render_unresponsive", "ppapi_plugin_exited", "unknown_exited"] {
            assert_eq!(decide(kind, now, &mut history, 0), Action::Nothing, "{kind}");
        }
    }

    #[test]
    fn the_restart_count_comes_from_the_environment() {
        assert_eq!(restarts_so_far(None), 0);
        assert_eq!(restarts_so_far(Some("1")), 1);
        assert_eq!(restarts_so_far(Some(" 2 ")), 2);
        assert_eq!(restarts_so_far(Some("many")), 0);
    }
}
