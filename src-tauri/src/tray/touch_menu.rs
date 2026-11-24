//! Touch support for the tray icon's right-click menu.
//!
//! A touch long-press on a notification icon does not produce the WM_RBUTTONUP that the
//! tray-icon crate listens for: the shell sends WM_CONTEXTMENU, and tray-icon (0.26.1 and older)
//! has no arm for it and emits no event, so the gesture never opened the Material menu on a
//! touch screen (a mouse right-click still went through WM_RBUTTONUP as usual). This module
//! subclasses tray-icon's message window (class tray_icon_app, this process only) and turns
//! WM_CONTEXTMENU into the tray callback message with WM_RBUTTONUP — the exact shape a real
//! mouse right-click delivers — so the usual event path (popup_input::on_icon_event) runs
//! unchanged.

use log::{debug, info};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU8, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, FindWindowExW, GWLP_WNDPROC, GetWindowThreadProcessId,
    PostMessageW, SetWindowLongPtrW, WM_CONTEXTMENU, WM_LBUTTONUP, WM_RBUTTONUP,
};
use windows::core::{PCWSTR, w};

/// The taskbar callback message tray-icon registers for its icons. The constant is private in
/// the crate but fixed there (6002) and pinned by Cargo.lock; a change upstream needs the same
/// change here.
const TRAY_CALLBACK_MSG: u32 = 6002;
const TOUCH_IDLE: u8 = 0;
const TOUCH_WAITING_FOR_REPLAY: u8 = 1;
const TOUCH_SUPPRESSING_RELEASE: u8 = 2;
const TOUCH_RELEASE_WINDOW_MS: u64 = 1000;

/// The message window this process has hooked, 0 when none.
static HOOKED: AtomicIsize = AtomicIsize::new(0);
/// The window procedure that was on the window before the hook.
static ORIGINAL: AtomicIsize = AtomicIsize::new(0);
/// The touch context-menu gesture is waiting for our replay, or suppressing the physical release
/// that follows it. Mouse clicks never change this state.
static TOUCH_STATE: AtomicU8 = AtomicU8::new(TOUCH_IDLE);
static TOUCH_SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
/// Whether the gesture's own release was consumed while waiting for the replay; if it
/// was, nothing follows the replay and the next click must reach tray-icon untouched.
static TOUCH_LEFT_EATEN: AtomicBool = AtomicBool::new(false);

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Installs the WM_CONTEXTMENU hook on the tray message window when it is not hooked yet. Call it
/// after each tray-icon build so a newly created message window is covered too.
pub fn ensure_installed() {
    let Some(hwnd) = find_tray_message_window() else {
        return;
    };
    if HOOKED.load(Ordering::Relaxed) == hwnd.0 as isize {
        return;
    }
    let previous =
        unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, hook_proc as *const () as isize) };
    if previous == 0 {
        return;
    }
    ORIGINAL.store(previous, Ordering::Relaxed);
    HOOKED.store(hwnd.0 as isize, Ordering::Relaxed);
    info!("[tray-touch] WM_CONTEXTMENU hook installed on the tray message window");
}

/// The message window of this process's tray icons: every application built on the crate makes
/// a tray_icon_app window, so the search filters by process id.
fn find_tray_message_window() -> Option<HWND> {
    let mut hwnd = HWND::default();
    for _ in 0..64 {
        hwnd = unsafe { FindWindowExW(None, Some(hwnd), w!("tray_icon_app"), PCWSTR::null()) }
            .unwrap_or_default();
        if hwnd.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == std::process::id() {
            return Some(hwnd);
        }
    }
    None
}

unsafe extern "system" fn hook_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == TRAY_CALLBACK_MSG {
            let callback_kind = lparam.0 as u32;
            let state = TOUCH_STATE.load(Ordering::Acquire);
            if state != TOUCH_IDLE || callback_kind == WM_RBUTTONUP || callback_kind == WM_LBUTTONUP {
                debug!("[tray-touch] callback {:#x} state {} ({})", callback_kind, state, super::input_source::describe_message_source());
            }
            if state == TOUCH_WAITING_FOR_REPLAY {
                if callback_kind == WM_RBUTTONUP {
                    if TOUCH_LEFT_EATEN.swap(false, Ordering::AcqRel) {
                        // The gesture's own release already arrived and was eaten, so nothing
                        // follows this replay: leave the next click for tray-icon.
                        TOUCH_STATE.store(TOUCH_IDLE, Ordering::Release);
                    } else {
                        // This is our replay, or a physical right-up that arrived before it. Let
                        // one right-up through to tray-icon so the menu opens, then guard the
                        // release that may follow it.
                        TOUCH_STATE.store(TOUCH_SUPPRESSING_RELEASE, Ordering::Release);
                    }
                } else if callback_kind == WM_LBUTTONUP {
                    // The touch release can arrive before the queued replay. It is part of the
                    // context-menu gesture and must not become a second tray click.
                    TOUCH_LEFT_EATEN.store(true, Ordering::Release);
                    return LRESULT(0);
                }
            } else if state == TOUCH_SUPPRESSING_RELEASE {
                if now_millis() > TOUCH_SUPPRESS_UNTIL_MS.load(Ordering::Acquire) {
                    TOUCH_STATE.store(TOUCH_IDLE, Ordering::Release);
                } else if callback_kind == WM_LBUTTONUP || callback_kind == WM_RBUTTONUP {
                    TOUCH_STATE.store(TOUCH_IDLE, Ordering::Release);
                    return LRESULT(0);
                }
            }
        }
        if message == WM_CONTEXTMENU {
            debug!("[tray-touch] WM_CONTEXTMENU wparam {:#x} lparam {:#x} ({})", wparam.0, lparam.0, super::input_source::describe_message_source());
            // Replay the gesture as the tray callback a real mouse right-click delivers. The
            // touch release can be delivered before or after this queued message, so the callback
            // state above consumes that one release and leaves mouse clicks untouched.
            TOUCH_SUPPRESS_UNTIL_MS
                .store(now_millis() + TOUCH_RELEASE_WINDOW_MS, Ordering::Release);
            TOUCH_LEFT_EATEN.store(false, Ordering::Release);
            TOUCH_STATE.store(TOUCH_WAITING_FOR_REPLAY, Ordering::Release);
            if PostMessageW(
                Some(hwnd),
                TRAY_CALLBACK_MSG,
                WPARAM(0),
                LPARAM(WM_RBUTTONUP as isize),
            )
            .is_err()
            {
                TOUCH_STATE.store(TOUCH_IDLE, Ordering::Release);
            }
            return LRESULT(0);
        }
        let original = ORIGINAL.load(Ordering::Relaxed);
        if original == 0 {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        CallWindowProcW(
            Some(std::mem::transmute::<
                isize,
                unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
            >(original)),
            hwnd,
            message,
            wparam,
            lparam,
        )
    }
}
