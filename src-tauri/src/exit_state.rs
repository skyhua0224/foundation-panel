//! The panel's own exit, as one fact the WebView2 failure handler can ask about.
//!
//! A quit from the tray, the handoff to the update helper, a restart with administrator rights, the restart
//! after a dead browser process: from the moment one of those paths decides to leave, the WebView2 processes
//! end with this one. WebView2 still reports those endings through `ProcessFailed` — the event exists for
//! "diagnostics and recovery from failures in the WebView2 processes" (Microsoft Learn,
//! panel-ms-webview2-process-events-20261121) — and the recovery the guidance asks for (reload the page,
//! recreate the controls, start again) is exactly wrong while the process is going down: those endings are
//! not failures of the machine. The state is therefore set at the start of every quit and restart path
//! (`begin`), never cleared (no path that sets it continues the run), and asked before a failure is
//! recorded or acted on (`crash::win`). `AppHandle::exit` and `AppHandle::restart` both trigger
//! `RunEvent::ExitRequested` before `RunEvent::Exit` (docs.rs, tauri 2.12.4:
//! panel-tauri-apphandle-exit-20261121, panel-tauri-runevent-20261121), so the two arms in `main.rs`
//! are the net every other exit path falls through.

use log::info;
use std::sync::OnceLock;

/// The exit of one run: the reason is the state, and the first reason wins (what began the exit).
pub(crate) struct Shutdown {
    reason: OnceLock<&'static str>,
}

impl Shutdown {
    pub(crate) const fn new() -> Self {
        Self {
            reason: OnceLock::new(),
        }
    }

    /// Marks this run as exiting. False when it already was: a later marker changes nothing.
    pub(crate) fn begin(&self, reason: &'static str) -> bool {
        self.reason.set(reason).is_ok()
    }

    pub(crate) fn is_exiting(&self) -> bool {
        self.reason.get().is_some()
    }

    /// What began the exit, for the log and the failure handler.
    pub(crate) fn reason(&self) -> Option<&'static str> {
        self.reason.get().copied()
    }
}

static SHUTDOWN: Shutdown = Shutdown::new();

/// The panel has started its own shutdown or restart; `reason` names the path that began it.
pub(crate) fn begin(reason: &'static str) {
    if SHUTDOWN.begin(reason) {
        info!("[exit] the panel is on its way out: {reason}");
    }
}

/// Whether the panel is shutting down or restarting right now.
pub(crate) fn is_exiting() -> bool {
    SHUTDOWN.is_exiting()
}

/// The path that began the exit.
pub(crate) fn reason() -> Option<&'static str> {
    SHUTDOWN.reason()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_is_not_exiting_until_a_path_marks_it() {
        let shutdown = Shutdown::new();
        assert!(!shutdown.is_exiting());
        assert_eq!(shutdown.reason(), None);
        assert!(shutdown.begin("tray quit"));
        assert!(shutdown.is_exiting());
        assert_eq!(shutdown.reason(), Some("tray quit"));
    }

    #[test]
    fn the_path_that_begins_the_exit_keeps_the_reason() {
        let shutdown = Shutdown::new();
        assert!(shutdown.begin("update handoff"));
        assert!(
            !shutdown.begin("exit requested"),
            "a later marker changes nothing"
        );
        assert_eq!(shutdown.reason(), Some("update handoff"));
        assert!(
            shutdown.is_exiting(),
            "the state never goes back: no path that marks it continues the run"
        );
    }

    /// Every quit and restart path marks the exit: the tray quit, the update handoff, the elevated restart,
    /// the webview restart, and the run-event arms every `AppHandle::exit` falls through. A path that
    /// loses its marker fails here, so an exit-time failure cannot be reported as a crash again unnoticed.
    #[test]
    fn every_quit_and_restart_path_marks_the_exit() {
        for (file, source) in [
            ("main.rs", include_str!("main.rs")),
            ("tray/actions.rs", include_str!("tray/actions.rs")),
            ("update.rs", include_str!("update.rs")),
            ("utils.rs", include_str!("utils.rs")),
            ("windows.rs", include_str!("windows.rs")),
        ] {
            assert!(
                source.contains("exit_state::begin("),
                "{file} lost its exit marker"
            );
        }
    }
}
