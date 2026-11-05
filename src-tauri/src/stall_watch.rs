//! A watch on the stalls of the panel: every half second a thread asks the main (UI) thread of the host to answer, and measures both how late the
//! thread itself woke up and how long the main thread took. A stall of the page is read against these lines: the thread late means the process or the
//! machine was starved, the main thread late with the thread on time means the host's UI thread was busy (a blocking command, a modal call), and neither
//! means the stall was inside the page or the WebView2 processes. Nothing is written for answers under the limit.

use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime};

const TICK: Duration = Duration::from_millis(500);
const LIMIT_MS: u64 = 700;
const GIVE_UP: Duration = Duration::from_secs(60);

/// The line for one tick, or none when both the wake-up and the main thread's answer were prompt.
pub fn describe(woke_late_ms: u64, main_ms: Option<u64>) -> Option<String> {
    let main_late = main_ms.map_or(true, |ms| ms >= LIMIT_MS);
    if woke_late_ms < LIMIT_MS && !main_late {
        return None;
    }
    let main = main_ms.map_or("no answer".to_string(), |ms| format!("{ms} ms"));
    let reading = match (woke_late_ms >= LIMIT_MS, main_late) {
        (true, _) => "the watcher thread itself woke late: the process or the machine was starved",
        (false, true) => "the UI thread of the host answered late while the watcher was on time: the UI thread was busy",
        _ => "",
    };
    Some(format!("[stall] watcher woke {woke_late_ms} ms late, main thread answered in {main}: {reading}"))
}

pub fn start<R: Runtime>(app: AppHandle<R>) {
    let _ = std::thread::Builder::new().name("stall-watch".into()).spawn(move || loop {
        let before = Instant::now();
        std::thread::sleep(TICK);
        let woke_late = before.elapsed().saturating_sub(TICK).as_millis() as u64;
        let (tx, rx) = mpsc::channel();
        let asked = Instant::now();
        if app.run_on_main_thread(move || { let _ = tx.send(()); }).is_err() {
            return;
        }
        let main_ms = rx.recv_timeout(GIVE_UP).ok().map(|_| asked.elapsed().as_millis() as u64);
        if let Some(line) = describe(woke_late, main_ms) {
            log::warn!("{line}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_ticks_write_nothing() {
        assert_eq!(describe(3, Some(2)), None);
        assert_eq!(describe(699, Some(699)), None);
    }

    #[test]
    fn a_late_main_thread_with_the_watcher_on_time_is_the_ui_thread() {
        let line = describe(4, Some(8200)).unwrap();
        assert!(line.contains("8200 ms") && line.contains("UI thread was busy"), "{line}");
    }

    #[test]
    fn a_late_watcher_is_the_machine_or_the_process() {
        let line = describe(1500, Some(1502)).unwrap();
        assert!(line.contains("woke 1500 ms late") && line.contains("starved"), "{line}");
    }

    #[test]
    fn no_answer_is_reported_as_late() {
        assert!(describe(0, None).unwrap().contains("no answer"));
    }
}
