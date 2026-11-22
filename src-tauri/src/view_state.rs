//! The snapshot of what the person was doing in the main window (route, scroll, tabs, sections, drafts), kept in memory for the window that is made again after a memory release.
//!
//! The page keeps the snapshot up to date while it is used (inject-viewstate.js: `panel_state_save`); when the window is let go (mem_adaptive.rs) the store is armed, and the next main window made
//! carries the snapshot in `window.__PANEL_RESTORE__` (an initialization script, so the router has it before its first view) and applies it. The snapshot lives in memory only: a draft never
//! reaches the disk, and a window made for any other reason (the first start, a restart) gets nothing.

use std::sync::Mutex;

/// The script that records and restores (the same file is put into every framed Web UI page by the proxy).
pub const SCRIPT: &str = include_str!("../inject-viewstate.js");

const MAX_BYTES: usize = 256 * 1024;

#[derive(Default)]
struct Store {
    snapshot: Option<String>,
    /// the snapshot the last window was made with: a window made ahead and let go again before anyone saw it saved nothing new, and the next one starts from this
    carry: Option<String>,
    armed: bool,
}

static STORE: Mutex<Store> = Mutex::new(Store { snapshot: None, carry: None, armed: false });

fn store() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The page reports its state (the latest one wins). Only the main window, only a JSON object of a sane size.
#[tauri::command]
pub fn panel_state_save(window: tauri::WebviewWindow, state: String) {
    if window.label() != "main" {
        return;
    }
    keep(state);
}

fn keep(state: String) {
    if state.len() > MAX_BYTES || !matches!(serde_json::from_str::<serde_json::Value>(&state), Ok(serde_json::Value::Object(_))) {
        return;
    }
    store().snapshot = Some(state);
}

/// The main window is about to be let go: the snapshot it left is for the next window.
pub fn arm_restore() {
    store().armed = true;
}

/// The release was called off (the window could not be closed): the next window is not a reopen.
pub fn disarm_restore() {
    store().armed = false;
}

/// The person has the window in front of them: its page is the authority on its state from now on.
pub fn clear_carry() {
    store().carry = None;
}

fn take() -> Option<String> {
    let mut store = store();
    if !std::mem::take(&mut store.armed) {
        return None;
    }
    let taken = store.snapshot.take().or_else(|| store.carry.take());
    store.carry = taken.clone();
    taken
}

/// What the main window's initialization script is: the snapshot assignment when a reopen after a release has one, then the recorder.
pub fn main_window_script() -> String {
    script_with(take().as_deref())
}

fn script_with(snapshot: Option<&str>) -> String {
    match snapshot {
        Some(json) => format!("window.__PANEL_RESTORE__ = {json};\n{SCRIPT}"),
        None => SCRIPT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The store is one per process and the tests run in threads: one at a time.
    static LOCK: Mutex<()> = Mutex::new(());

    fn reset() {
        let mut s = store();
        s.snapshot = None;
        s.carry = None;
        s.armed = false;
    }

    #[test]
    fn a_window_made_for_any_other_reason_gets_nothing() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset();
        keep(r#"{"v":1}"#.into());
        assert_eq!(take(), None, "not armed: the first start and a restart start fresh");
        assert_eq!(script_with(None), SCRIPT);
        reset();
    }

    #[test]
    fn a_released_window_gets_its_snapshot_once() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset();
        keep(r#"{"v":1,"shell":{"scroll":[]}}"#.into());
        arm_restore();
        let script = main_window_script();
        assert!(script.starts_with(r#"window.__PANEL_RESTORE__ = {"v":1,"shell":{"scroll":[]}};"#));
        assert!(script.ends_with(SCRIPT) || script.contains("__panelViewState"));
        assert_eq!(take(), None, "taken once");
        reset();
    }

    #[test]
    fn only_a_json_object_of_sane_size_is_kept() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset();
        keep("not json".into());
        keep("[1,2]".into());
        keep("\"text\"".into());
        keep(format!("{{\"x\":\"{}\"}}", "a".repeat(MAX_BYTES)));
        arm_restore();
        assert_eq!(take(), None);
        reset();
    }

    #[test]
    fn a_called_off_release_makes_the_next_window_a_plain_one() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset();
        keep(r#"{"v":1}"#.into());
        arm_restore();
        disarm_restore();
        assert_eq!(take(), None);
        reset();
    }

    #[test]
    fn a_window_made_ahead_and_let_go_unseen_hands_the_snapshot_on() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset();
        keep(r#"{"v":1,"nav":"a"}"#.into());
        arm_restore();
        assert!(take().is_some(), "the made-ahead window gets it");
        // nobody saw that window and it saved nothing; it is let go again
        arm_restore();
        assert_eq!(take().as_deref(), Some(r#"{"v":1,"nav":"a"}"#), "the next one starts from the same state");
        // once the person has seen a window, the page is the authority
        clear_carry();
        arm_restore();
        assert_eq!(take(), None);
        reset();
    }

    #[test]
    fn the_recorder_never_keeps_secrets_in_its_source() {
        let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // the sensitive-field rule is part of the script that ships
        assert!(SCRIPT.contains("password") && SCRIPT.contains("hidden") && SCRIPT.contains("one-time-code"));
    }
}
