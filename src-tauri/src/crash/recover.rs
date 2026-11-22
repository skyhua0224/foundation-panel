//! What the panel does when a WebView2 process fails, so the window does not stay blank.
//!
//! A failure used to be only written to the crash report: the renderer of the window died (`render_exited`, the most frequent failure of the
//! first release: twelve reports from one day) and the page stayed empty until the user closed and opened the panel again. Then the page was
//! reloaded, three times; a renderer that dies again and again (the window flashed white every second and stayed white) was left alone.
//!
//! * `render_exited`, `frame_render_exited`: the page is gone, the browser is alive: reload it (Microsoft's guidance for
//!   `RenderProcessExited`). A renderer that dies a third time within five minutes is not a one-off: the panel starts again with GPU
//!   acceleration off (software rendering), which is where a driver fault stops mattering (`fallback.rs` keeps that choice for the
//!   next runs). If it still dies with GPU acceleration off, the person is told in a native dialog, since the window cannot.
//! * `gpu_exited`: Chromium starts the GPU process again by itself (Microsoft: "auto-recoverable"); three exits in five minutes is a GPU
//!   process that cannot stay up, and the same switch to software rendering is made before the page dies.
//! * `browser_exited`: the whole WebView2 is unusable (the runtime was updated or killed): start the panel again, at most twice in a row.
//! * `render_unresponsive`: the runtime raises `render_exited` itself when it gives up on the process, so there is nothing to do yet.
//! * `utility_exited`, `sandbox_helper_exited`: Chromium starts these again by itself.
//!
//! The decision is a plain function of the failure kind and the recent history: the part that touches the window is in `win.rs`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Renderer failures within `WINDOW` after which a reload is not enough: the third one switches to software rendering.
const SOFTWARE_AFTER_EXITS: usize = 3;
/// GPU process failures within `WINDOW` after which the GPU is left out.
const SOFTWARE_AFTER_GPU_EXITS: usize = 3;
/// GPU process failures within `WINDOW` from which a crash report is worth sending: one is the usual self-healing blip (Chromium
/// starts the process again and the window never notices), two in five minutes mean the process is not stable.
const REPORT_AFTER_GPU_EXITS: usize = 2;
const WINDOW: Duration = Duration::from_secs(5 * 60);
/// Restarts in a row (the count is carried to the new process in `RESTARTS_ENV`).
const MAX_RESTARTS: u32 = 2;
pub(crate) const RESTARTS_ENV: &str = "SUNSHINE_PANEL_WEBVIEW_RESTARTS";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Action {
    Nothing,
    Reload,
    Restart,
    /// Start the panel again with GPU acceleration off, and keep that for the next runs.
    SoftwareRestart,
    /// Nothing the panel can do any more: tell the person (the window cannot).
    GiveUp,
}

#[derive(Default)]
pub(crate) struct History {
    exits: VecDeque<Instant>,
    gpu: VecDeque<Instant>,
}

fn prune(queue: &mut VecDeque<Instant>, now: Instant) {
    while queue.front().is_some_and(|t| now.duration_since(*t) > WINDOW) {
        queue.pop_front();
    }
}

/// The value of `RESTARTS_ENV` of this process (0 when it is not set or not a number).
pub(crate) fn restarts_so_far(value: Option<&str>) -> u32 {
    value.and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// How the window renders in this run: with the GPU (the default), with GPU acceleration already off, or with the GPU kept on by the
/// person (`SUNSHINE_PANEL_GPU=1`): that choice is never overridden, so the last step after repeated failures is to tell them.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum RenderMode {
    Gpu,
    Software,
    GpuForced,
}

/// `kind` is the name the crash report uses (`render_exited`, `browser_exited`, ...).
pub(crate) fn decide(kind: &str, now: Instant, history: &mut History, restarts: u32, mode: RenderMode) -> Action {
    match kind {
        "render_exited" | "frame_render_exited" => {
            prune(&mut history.exits, now);
            history.exits.push_back(now);
            if history.exits.len() >= SOFTWARE_AFTER_EXITS {
                return if mode == RenderMode::Gpu { Action::SoftwareRestart } else { Action::GiveUp };
            }
            Action::Reload
        }
        "gpu_exited" => {
            prune(&mut history.gpu, now);
            history.gpu.push_back(now);
            if history.gpu.len() >= SOFTWARE_AFTER_GPU_EXITS && mode == RenderMode::Gpu {
                Action::SoftwareRestart
            } else {
                Action::Nothing
            }
        }
        "browser_exited" if restarts < MAX_RESTARTS => Action::Restart,
        _ => Action::Nothing,
    }
}

/// Whether a failure that `decide` has just recorded goes into a crash report. Every kind does, except the GPU process exiting once:
/// Chromium starts it again by itself and nobody sees it, which made it the most reported "crash" of the panel (about a quarter of
/// all reports, from nearly as many installs). The second exit within `WINDOW` is reported, so a driver that really keeps failing still is.
pub(crate) fn worth_reporting(kind: &str, history: &History) -> bool {
    match kind {
        "gpu_exited" => history.gpu.len() >= REPORT_AFTER_GPU_EXITS,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dead_renderer_is_reloaded_twice_and_the_third_death_leaves_the_gpu_out() {
        let mut history = History::default();
        let t0 = Instant::now();
        assert_eq!(decide("render_exited", t0, &mut history, 0, RenderMode::Gpu), Action::Reload);
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(10), &mut history, 0, RenderMode::Gpu), Action::Reload);
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(20), &mut history, 0, RenderMode::Gpu), Action::SoftwareRestart);
        // a frame renderer counts the same
        let mut frames = History::default();
        assert_eq!(decide("frame_render_exited", t0, &mut frames, 0, RenderMode::Gpu), Action::Reload);
    }

    #[test]
    fn with_the_gpu_already_off_a_third_death_tells_the_person() {
        let mut history = History::default();
        let t0 = Instant::now();
        for i in 0..2 {
            assert_eq!(decide("render_exited", t0 + Duration::from_secs(i), &mut history, 0, RenderMode::Software), Action::Reload);
        }
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(3), &mut history, 0, RenderMode::Software), Action::GiveUp);
    }

    /// `SUNSHINE_PANEL_GPU=1` is the person's choice: repeated deaths never restart the panel into software rendering, they tell the person.
    #[test]
    fn a_forced_gpu_is_never_switched_off() {
        let mut history = History::default();
        let t0 = Instant::now();
        assert_eq!(decide("render_exited", t0, &mut history, 0, RenderMode::GpuForced), Action::Reload);
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(1), &mut history, 0, RenderMode::GpuForced), Action::Reload);
        assert_eq!(decide("render_exited", t0 + Duration::from_secs(2), &mut history, 0, RenderMode::GpuForced), Action::GiveUp);
        let mut gpu = History::default();
        for i in 0..5 {
            assert_eq!(decide("gpu_exited", t0 + Duration::from_secs(i), &mut gpu, 0, RenderMode::GpuForced), Action::Nothing);
        }
    }

    #[test]
    fn deaths_far_apart_do_not_add_up() {
        let mut history = History::default();
        let t0 = Instant::now();
        for i in 0..6 {
            // one death every ten minutes: every one is a reload, none switches anything off
            assert_eq!(decide("render_exited", t0 + Duration::from_secs(i * 600), &mut history, 0, RenderMode::Gpu), Action::Reload);
        }
    }

    #[test]
    fn a_gpu_process_that_cannot_stay_up_is_left_out_before_the_page_dies() {
        let mut history = History::default();
        let t0 = Instant::now();
        assert_eq!(decide("gpu_exited", t0, &mut history, 0, RenderMode::Gpu), Action::Nothing);
        assert_eq!(decide("gpu_exited", t0 + Duration::from_secs(30), &mut history, 0, RenderMode::Gpu), Action::Nothing);
        assert_eq!(decide("gpu_exited", t0 + Duration::from_secs(60), &mut history, 0, RenderMode::Gpu), Action::SoftwareRestart);
        // already software: nothing to switch
        let mut software = History::default();
        for i in 0..4 {
            assert_eq!(decide("gpu_exited", t0 + Duration::from_secs(i), &mut software, 0, RenderMode::Software), Action::Nothing);
        }
    }

    #[test]
    fn one_gpu_process_exit_is_not_reported_but_a_second_within_five_minutes_is() {
        let mut history = History::default();
        let t0 = Instant::now();
        decide("gpu_exited", t0, &mut history, 0, RenderMode::Gpu);
        assert!(!worth_reporting("gpu_exited", &history));
        decide("gpu_exited", t0 + Duration::from_secs(90), &mut history, 0, RenderMode::Gpu);
        assert!(worth_reporting("gpu_exited", &history));
        // exits far apart never add up
        let mut spread = History::default();
        for i in 0..4 {
            decide("gpu_exited", t0 + Duration::from_secs(i * 600), &mut spread, 0, RenderMode::Gpu);
            assert!(!worth_reporting("gpu_exited", &spread));
        }
    }

    #[test]
    fn every_other_failure_is_reported() {
        let history = History::default();
        for kind in ["render_exited", "frame_render_exited", "browser_exited", "utility_exited", "render_unresponsive", "unknown_exited"] {
            assert!(worth_reporting(kind, &history), "{kind}");
        }
    }

    #[test]
    fn a_dead_browser_restarts_the_panel_twice_in_a_row() {
        let mut history = History::default();
        let now = Instant::now();
        assert_eq!(decide("browser_exited", now, &mut history, 0, RenderMode::Gpu), Action::Restart);
        assert_eq!(decide("browser_exited", now, &mut history, 1, RenderMode::Gpu), Action::Restart);
        assert_eq!(decide("browser_exited", now, &mut history, 2, RenderMode::Gpu), Action::Nothing);
    }

    #[test]
    fn what_chromium_restarts_by_itself_is_left_alone() {
        let mut history = History::default();
        let now = Instant::now();
        for kind in ["utility_exited", "sandbox_helper_exited", "render_unresponsive", "ppapi_plugin_exited", "unknown_exited"] {
            assert_eq!(decide(kind, now, &mut history, 0, RenderMode::Gpu), Action::Nothing, "{kind}");
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
