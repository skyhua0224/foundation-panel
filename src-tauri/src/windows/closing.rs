//! The close of a panel window, step by step, and the guarantee that a window that is to be closed never shows again.
//!
//! A window that is destroyed while it is visible shows what is left of it: tao creates every window with WS_CAPTION and hides the frame only by answering
//! WM_NCCALCSIZE, so as soon as the window's thread does not answer (the WebView2 teardown, the exit of the process, a stall of the main thread) the system draws
//! the window from its style bits: a flat client area under the old light title bar with its three square buttons, the title and the icon, and it stays.
//! The window is therefore hidden in the same event that asks for the close, before anything is torn down; a watchdog destroys it again if the destruction does
//! not come, and a window that is closing cannot be shown by any other path (tray, second launch, a pending show) until its `Destroyed` event has come.
//! Every step is a line of panel.log, so a window that did not go can be followed: requested, hidden, destroyed (with the time), forced, given up.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use tauri::{AppHandle, Manager, Runtime, Window};

/// After this long a window that has not been destroyed is destroyed again.
pub(crate) const FORCE_AFTER: Duration = Duration::from_millis(1500);
/// After this long the window is given up: it stays hidden, and the log says so.
pub(crate) const GIVE_UP_AFTER: Duration = Duration::from_millis(5000);
const WATCH_TICK: Duration = Duration::from_millis(100);

/// What the watchdog does at a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Not (or no longer) closing: the watchdog ends.
    Done,
    Wait,
    /// Hide it again and destroy it (once).
    Force,
    /// Say it in the log and end the watchdog; the window stays hidden and stays "closing" (once).
    GiveUp,
}

struct Entry {
    since: Instant,
    forced: bool,
    gave_up: bool,
}

#[derive(Default)]
pub(crate) struct Tracker {
    closing: HashMap<String, Entry>,
}

impl Tracker {
    /// The close of `label` starts. False when it is already closing (a second request changes nothing).
    pub(crate) fn begin(&mut self, label: &str, now: Instant) -> bool {
        if self.closing.contains_key(label) {
            return false;
        }
        self.closing.insert(label.to_owned(), Entry { since: now, forced: false, gave_up: false });
        true
    }

    pub(crate) fn is_closing(&self, label: &str) -> bool {
        self.closing.contains_key(label)
    }

    /// The window is gone. How long the close took, if one was going.
    pub(crate) fn finished(&mut self, label: &str, now: Instant) -> Option<Duration> {
        self.closing.remove(label).map(|entry| now.duration_since(entry.since))
    }

    /// The watchdog's decision at `now`: forcing and giving up each come once, in that order.
    pub(crate) fn check(&mut self, label: &str, now: Instant) -> Step {
        let Some(entry) = self.closing.get_mut(label) else { return Step::Done };
        let age = now.duration_since(entry.since);
        if entry.gave_up {
            Step::Done
        } else if age >= GIVE_UP_AFTER {
            entry.gave_up = true;
            Step::GiveUp
        } else if age >= FORCE_AFTER && !entry.forced {
            entry.forced = true;
            Step::Force
        } else {
            Step::Wait
        }
    }
}

static TRACKER: Mutex<Option<Tracker>> = Mutex::new(None);

fn tracker<T>(f: impl FnOnce(&mut Tracker) -> T) -> T {
    let mut guard = TRACKER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(guard.get_or_insert_with(Tracker::default))
}

/// A window under this label is closing: nothing shows it, and a new one is made only after its `Destroyed`.
pub(crate) fn is_closing(label: &str) -> bool {
    tracker(|t| t.is_closing(label))
}

/// The window event `CloseRequested` (the title bar, Alt+F4, the taskbar, `close()` of the page or of the host): hidden at once, then the destruction is watched.
pub(crate) fn close_requested<R: Runtime>(window: &Window<R>) {
    let label = window.label().to_owned();
    if !tracker(|t| t.begin(&label, Instant::now())) {
        info!("[close] {label}: close requested again, already closing");
        return;
    }
    info!("[close] {label}: close requested");
    match window.hide() {
        Ok(()) => info!("[close] {label}: hidden"),
        Err(e) => warn!("[close] {label}: could not be hidden: {e}"),
    }
    watch(window.app_handle().clone(), label);
}

/// The window event `Destroyed`.
pub(crate) fn destroyed(label: &str) {
    match tracker(|t| t.finished(label, Instant::now())) {
        Some(took) => info!("[close] {label}: destroyed after {} ms", took.as_millis()),
        None => info!("[close] {label}: destroyed (not through a close request)"),
    }
}

/// Closes the window `label` as the host's `--close-desktop` does: `close()` goes through `close_requested`, so the window is hidden before it is torn down.
pub(crate) fn close_window<R: Runtime>(app: &AppHandle<R>, label: &str, reason: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else { return false };
    info!("[close] {label}: closing ({reason})");
    if let Err(e) = window.close() {
        warn!("[close] {label}: close() failed: {e}");
        return false;
    }
    true
}

/// Runs `then` when no window of `label` is closing any more (at once if none is); gives up after `GIVE_UP_AFTER` and says so. A window cannot be made under a label whose
/// old window is still in the registry, and the old one must not be shown in the meantime.
pub(crate) fn after_closed<R: Runtime>(app: &AppHandle<R>, label: &'static str, then: impl FnOnce(AppHandle<R>) + Send + 'static) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        while is_closing(label) {
            if started.elapsed() > GIVE_UP_AFTER + FORCE_AFTER {
                error!("[close] {label}: still closing after {} ms, the request that waited for it is dropped", started.elapsed().as_millis());
                return;
            }
            tokio::time::sleep(WATCH_TICK).await;
        }
        then(app);
    });
}

fn watch<R: Runtime>(app: AppHandle<R>, label: String) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(WATCH_TICK).await;
            let step = tracker(|t| t.check(&label, Instant::now()));
            match step {
                Step::Done => return,
                Step::Wait => {}
                Step::Force => {
                    warn!("[close] {label}: not destroyed after {} ms, destroying it", FORCE_AFTER.as_millis());
                    match app.get_webview_window(&label) {
                        Some(window) => {
                            let _ = window.hide();
                            match window.destroy() {
                                Ok(()) => info!("[close] {label}: destroy requested"),
                                Err(e) => warn!("[close] {label}: destroy failed: {e}"),
                            }
                        }
                        // gone from the registry without its event: nothing is left to wait for
                        None => {
                            warn!("[close] {label}: no window under the label any more");
                            tracker(|t| t.finished(&label, Instant::now()));
                            return;
                        }
                    }
                }
                Step::GiveUp => {
                    error!("[close] {label}: still not destroyed after {} ms; it stays hidden and is not shown again", GIVE_UP_AFTER.as_millis());
                    if let Some(window) = app.get_webview_window(&label) {
                        let _ = window.hide();
                    }
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn a_close_runs_requested_then_destroyed_and_the_label_is_free_again() {
        let (mut t, base) = (Tracker::default(), Instant::now());
        assert!(!t.is_closing("main"));
        assert!(t.begin("main", base));
        assert!(t.is_closing("main"));
        assert!(!t.begin("main", at(base, 5)), "a second request changes nothing");
        assert_eq!(t.check("main", at(base, 100)), Step::Wait);
        assert_eq!(t.finished("main", at(base, 180)), Some(Duration::from_millis(180)));
        assert!(!t.is_closing("main"));
        assert_eq!(t.check("main", at(base, 2000)), Step::Done, "a destroyed window needs no watchdog");
        assert!(t.begin("main", at(base, 3000)), "the next window under the label closes the same way");
    }

    #[test]
    fn a_stalled_teardown_is_forced_once_then_given_up_once_and_stays_closing() {
        let (mut t, base) = (Tracker::default(), Instant::now());
        t.begin("desktop", base);
        assert_eq!(t.check("desktop", at(base, FORCE_AFTER.as_millis() as u64 - 1)), Step::Wait);
        assert_eq!(t.check("desktop", at(base, FORCE_AFTER.as_millis() as u64)), Step::Force);
        assert_eq!(t.check("desktop", at(base, FORCE_AFTER.as_millis() as u64 + 100)), Step::Wait, "forced once");
        assert_eq!(t.check("desktop", at(base, GIVE_UP_AFTER.as_millis() as u64)), Step::GiveUp);
        assert_eq!(t.check("desktop", at(base, GIVE_UP_AFTER.as_millis() as u64 + 100)), Step::Done, "given up once");
        assert!(t.is_closing("desktop"), "a window that could not be destroyed is never shown again");
        assert_eq!(t.finished("desktop", at(base, 9000)), Some(Duration::from_millis(9000)), "a late Destroyed still frees the label");
        assert!(!t.is_closing("desktop"));
    }

    #[test]
    fn windows_are_tracked_apart_and_an_unknown_one_is_not_closing() {
        let (mut t, base) = (Tracker::default(), Instant::now());
        t.begin("main", base);
        assert!(!t.is_closing("desktop"));
        assert_eq!(t.check("desktop", at(base, 9000)), Step::Done);
        assert_eq!(t.finished("desktop", base), None);
        assert!(t.is_closing("main"));
    }

    /// Every path that shows a window passes the closing check: a path added later that does not is a window that comes back after its close.
    #[test]
    fn the_show_paths_ask_whether_the_window_is_closing() {
        let source = include_str!("../windows.rs");
        let code: String = source.lines().filter(|line| !line.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        let shown = code.split("pub fn show_and_activate_window").nth(1).and_then(|rest| rest.split("\n}\n").next()).expect("show_and_activate_window");
        assert!(shown.contains("closing::is_closing"), "show_and_activate_window must refuse a closing window");
        let uncloaked = code.split("fn uncloak_cold").nth(1).and_then(|rest| rest.split("\n}\n").next()).expect("uncloak_cold");
        assert!(uncloaked.contains("closing::is_closing"), "uncloak_cold must not focus a closing window");
        for entry in ["fn request_main_window", "pub fn open_desktop_window", "pub fn open_main_panel_page"] {
            let body = code.split(entry).nth(1).and_then(|rest| rest.split("\n}\n").next()).unwrap_or_else(|| panic!("{entry}"));
            assert!(body.contains("closing::after_closed"), "{entry} must wait for a closing window instead of showing it");
        }
    }
}
