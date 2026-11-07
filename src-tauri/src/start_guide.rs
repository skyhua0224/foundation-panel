//! `--start-guide` opens the first-run start guide at its first step, whatever progress was saved.
//!
//! A second launch never opens its own window. tauri-plugin-single-instance 2.5.4 hands that
//! process's `std::env::args_os` to the running one, executable path first
//! (panel-tauri-single-instance-20261107; on Windows, panel-tauri-single-instance-win-20261107).
//! The request waits in one slot because the main window may still be loading. The renderer takes
//! it on `start-guide-pending` and once its listeners are registered. Taking empties the slot.

use std::sync::atomic::{AtomicBool, Ordering};

/// One outstanding `--start-guide`. A newer request while one is waiting is the same request.
pub struct StartGuideSlot {
    pending: AtomicBool,
}

impl StartGuideSlot {
    pub const fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
        }
    }

    pub fn queue(&self) {
        self.pending.store(true, Ordering::SeqCst);
    }

    pub fn take(&self) -> bool {
        self.pending.swap(false, Ordering::SeqCst)
    }
}

static PENDING: StartGuideSlot = StartGuideSlot::new();

/// Exact `--start-guide` only. Other arguments stay with their own parsers.
pub fn requests_start_guide<S: AsRef<str>>(args: &[S]) -> bool {
    args.iter().any(|arg| arg.as_ref() == "--start-guide")
}

pub fn queue_start_guide() {
    PENDING.queue();
}

pub fn take_start_guide() -> bool {
    PENDING.take()
}

/// The renderer takes the waiting request. `false` when it was taken already.
#[cfg_attr(not(standalone_test), tauri::command)]
pub fn take_pending_start_guide() -> bool {
    take_start_guide()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn the_flag_is_exact() {
        assert!(requests_start_guide(&["sunshine-gui.exe", "--start-guide"]));
        assert!(!requests_start_guide(&["sunshine-gui.exe"]));
        assert!(!requests_start_guide(&[
            "sunshine-gui.exe",
            "--start-guide=1"
        ]));
        assert!(!requests_start_guide(&["sunshine-gui.exe", "--desktop"]));
        assert!(requests_start_guide(&[
            "sunshine-gui.exe",
            "--minimized",
            "--start-guide"
        ]));
    }

    #[test]
    fn a_request_is_taken_once_and_a_newer_one_is_the_same_request() {
        let slot = StartGuideSlot::new();
        assert!(!slot.take());
        slot.queue();
        slot.queue();
        assert!(slot.take());
        assert!(!slot.take());
    }

    #[test]
    fn the_ping_and_the_mount_deliver_it_once() {
        for _ in 0..200 {
            let slot = Arc::new(StartGuideSlot::new());
            slot.queue();
            let takers: Vec<_> = (0..2)
                .map(|_| {
                    let slot = Arc::clone(&slot);
                    std::thread::spawn(move || slot.take())
                })
                .collect();
            let got = takers
                .into_iter()
                .filter_map(|taker| taker.join().ok())
                .filter(|taken| *taken)
                .count();
            assert_eq!(got, 1);
        }
    }
}
