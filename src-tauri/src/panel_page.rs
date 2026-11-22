//! Pages another window asks the main window to open (the floating toolbar's menu, the tray's 虚拟显示器). The main window may not be listening
//! yet: it was closed and is being created, or it is reloading. So the request waits in one slot. A `panel-page-pending` ping tells a listening
//! renderer to take it, and the renderer also takes it once its listeners are registered (SunshineFrame on mount). Taking empties the slot, so a
//! request is delivered exactly once whichever of the two comes first; a newer request replaces one that was not taken yet.
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub(crate) struct PendingPage(Mutex<Option<String>>);
impl PendingPage {
    pub(crate) const fn new() -> Self {
        Self(Mutex::new(None))
    }
    pub(crate) fn put(&self, page: &str) {
        *self.0.lock().unwrap() = Some(page.to_string());
    }
    pub(crate) fn take(&self) -> Option<String> {
        self.0.lock().unwrap().take()
    }
}
static PENDING: PendingPage = PendingPage::new();

/// Queue a page for the renderer and ping the main window if it is already present.
pub fn open_page<R: Runtime>(app: &AppHandle<R>, page: &str) {
    PENDING.put(page);
    log::info!("panel page requested: {page}");
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("panel-page-pending", ());
    }
}

/// The renderer takes the waiting page (none when it was taken already).
#[tauri::command]
pub fn take_pending_panel_page() -> Option<String> {
    PENDING.take()
}

#[cfg(test)]
mod tests {
    use super::PendingPage;
    use std::sync::Arc;

    #[test]
    fn a_request_is_taken_exactly_once() {
        let slot = PendingPage::new();
        assert_eq!(slot.take(), None);
        slot.put("desktop-scale");
        assert_eq!(slot.take().as_deref(), Some("desktop-scale"));
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn a_newer_request_replaces_one_not_taken_yet() {
        let slot = PendingPage::new();
        slot.put("vdd");
        slot.put("live-session");
        assert_eq!(slot.take().as_deref(), Some("live-session"));
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn the_ping_and_the_mount_racing_deliver_it_once() {
        // the renderer's mount and its ping handler both take; only one of them gets the page
        for _ in 0..200 {
            let slot = Arc::new(PendingPage::new());
            slot.put("shortcuts");
            let takers: Vec<_> = (0..2).map(|_| { let s = slot.clone(); std::thread::spawn(move || s.take()) }).collect();
            let got: Vec<_> = takers.into_iter().filter_map(|t| t.join().unwrap()).collect();
            assert_eq!(got, vec!["shortcuts".to_string()]);
        }
    }
}
