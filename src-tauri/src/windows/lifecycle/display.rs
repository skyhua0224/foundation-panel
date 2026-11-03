//! Native display events feed the same Panel state as window visibility, and move the WebView with the window the moment its monitor changes.
//!
//! A change of the monitor's scale (WM_DPICHANGED: another monitor, the Settings scale, the resolution of a display whose scale follows it) used to reach the
//! page in pieces: tao resized the window when the message came, the page then laid out at its old rasterization scale (a 200 % page in a 125 % window:
//! 400 x 250 CSS pixels instead of 640 x 400) for another 170 to 200 ms, and the right scale arrived only with the debounced refresh below (0.3 to 0.4 s after
//! the change; a hidden panel kept the stale layout until it was shown). WebView2 detects monitor scale changes on its own and late; the panel now turns that
//! detection off and sets the scale itself, in the message, just before the window is resized, and sets the bounds to the client rectangle right after.
//! https://learn.microsoft.com/windows/win32/hidpi/wm-dpichanged
//! https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2controller3#put_shouldDetectMonitorScaleChanges
//! BoundsMode stays UseRawPixels: wry sets the bounds in physical pixels on every WM_SIZE, and DIP bounds would be scaled a second time.
use super::*;
use std::sync::{Arc, Mutex as StdMutex};
use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Controller, ICoreWebView2Controller3};
use wv2_windows_core::Interface;

/// The WebView2 controller, only ever used on the thread of the window (the message loop).
struct MainThreadController(ICoreWebView2Controller);
unsafe impl Send for MainThreadController {}

struct Hook<R: Runtime> {
    window: WebviewWindow<R>,
    panel: Panel,
    controller: Arc<StdMutex<Option<MainThreadController>>>,
}

/// The rasterization scale of a monitor DPI (96 is 100 %).
fn scale_of_dpi(dpi: u32) -> f64 {
    f64::from(dpi.max(1)) / 96.0
}

pub(super) fn install<R: Runtime>(window: &WebviewWindow<R>, panel: &Panel) {
    // The window's one WebView follows the window through wry's own WM_SIZE handler, which sets the bounds to the client rectangle in physical pixels. Tauri's
    // auto-resize sets them a second time from a logical size and a scale of its own, and during a change of scale it did so with the old physical size.
    let webview: &tauri::Webview<R> = window.as_ref();
    let _ = webview.set_auto_resize(false);
    let (window, panel) = (window.clone(), panel.clone());
    let dispatch = window.clone();
    let _ = dispatch.run_on_main_thread(move || {
        use windows::Win32::UI::Shell::SetWindowSubclass;
        if panel.state.lock().unwrap().phase == Phase::Closed {
            return;
        }
        let Ok(hwnd) = window.hwnd() else { return };
        let controller = Arc::new(StdMutex::new(None));
        let slot = controller.clone();
        let dpi_of_window = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(hwnd.0)) };
        let _ = window.with_webview(move |webview| {
            let controller = webview.controller();
            // the scale is the panel's from here on: the window's own, now and at every WM_DPICHANGED
            if let Ok(controller3) = controller.cast::<ICoreWebView2Controller3>() {
                unsafe {
                    let _ = controller3.SetShouldDetectMonitorScaleChanges(false);
                    if dpi_of_window > 0 {
                        let _ = controller3.SetRasterizationScale(scale_of_dpi(dpi_of_window));
                    }
                }
            }
            *slot.lock().unwrap() = Some(MainThreadController(controller));
        });
        let context = Box::into_raw(Box::new(Hook { window, panel, controller }));
        if !unsafe {
            SetWindowSubclass(
                windows::Win32::Foundation::HWND(hwnd.0),
                Some(callback::<R>),
                ID,
                context as usize,
            )
        }
        .as_bool()
        {
            unsafe {
                drop(Box::from_raw(context));
            }
            log::warn!("Failed to install WebView display subclass");
        }
    });
}

const ID: usize = 0x53535756;

unsafe extern "system" fn callback<R: Runtime>(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass},
        WindowsAndMessaging::{WM_DISPLAYCHANGE, WM_DPICHANGED, WM_NCDESTROY, WM_SHOWWINDOW},
    };
    let context = unsafe { &*(data as *const Hook<R>) };
    if message == WM_NCDESTROY {
        context.panel.close();
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(callback::<R>), ID);
            let result = DefSubclassProc(hwnd, message, wparam, lparam);
            drop(Box::from_raw(data as *mut Hook<R>));
            return result;
        }
    }
    if message == WM_DPICHANGED {
        apply_new_dpi(hwnd, context, wparam, lparam);
    }
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if matches!(message, WM_DISPLAYCHANGE | WM_DPICHANGED) {
        // a new resolution or scale moved or resized the window: the WebView follows in the same message
        sync_webview_to_client(hwnd, context);
    }
    if matches!(message, WM_DISPLAYCHANGE | WM_DPICHANGED) {
        let revision = context
            .panel
            .state
            .lock()
            .unwrap()
            .display_changed(Instant::now());
        if let Some(revision) = revision {
            let (window, panel) = (context.window.clone(), context.panel.clone());
            tauri::async_runtime::spawn(async move {
                // Coalesce display bursts; hidden panels retain the pending
                // refresh until they become visible, without waking up here.
                tokio::time::sleep(DISPLAY_DEBOUNCE).await;
                {
                    let state = panel.state.lock().unwrap();
                    if state.phase == Phase::Closed || state.display_revision != revision {
                        return;
                    }
                }
                reconcile(&window, &panel);
            });
        }
    } else if message == WM_SHOWWINDOW {
        // The HWND's visibility is updated after this notification returns.
        let (window, panel) = (context.window.clone(), context.panel.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            reconcile(&window, &panel);
        });
    }
    result
}

/// The new scale and, with it, the bounds of the size Windows suggests for the window (an undecorated window's client is its whole rectangle): the WebView never has
/// the new scale with the old bounds. Tao resizes the window to the same rectangle right after, and the WebView is made to match the client rectangle after that.
fn apply_new_dpi<R: Runtime>(
    hwnd: windows::Win32::Foundation::HWND,
    context: &Hook<R>,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) {
    use windows::Win32::UI::WindowsAndMessaging::IsZoomed;
    let dpi = u32::from((wparam.0 & 0xFFFF) as u16);
    let guard = context.controller.lock().unwrap();
    let Some(controller) = guard.as_ref() else { return };
    if let Ok(controller3) = controller.0.cast::<ICoreWebView2Controller3>() {
        if let Err(error) = unsafe { controller3.SetRasterizationScale(scale_of_dpi(dpi)) } {
            log::warn!("Failed to set the WebView scale for {dpi} dpi: {error}");
        }
    }
    // a maximized window is sized by the system: its WM_SIZE sets the bounds
    if lparam.0 != 0 && !unsafe { IsZoomed(hwnd) }.as_bool() {
        let suggested = unsafe { *(lparam.0 as *const windows::Win32::Foundation::RECT) };
        let (width, height) = (suggested.right - suggested.left, suggested.bottom - suggested.top);
        if width > 0 && height > 0 {
            let _ = set_bounds_and_host(&controller.0, width, height);
        }
    }
}

/// The bounds of the controller and, in the same call, the size of its host window (wry resizes the host asynchronously 70 ms later; WebView2 laid the page out for
/// the old host size meanwhile).
fn set_bounds_and_host(controller: &ICoreWebView2Controller, width: i32, height: i32) -> windows::core::Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};
    unsafe {
        controller.SetBounds(windows::Win32::Foundation::RECT { left: 0, top: 0, right: width, bottom: height })?;
        let mut host = windows::Win32::Foundation::HWND::default();
        if controller.ParentWindow(&mut host).is_ok() && !host.is_invalid() {
            let _ = SetWindowPos(host, None, 0, 0, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    Ok(())
}

/// The WebView's bounds are the window's client rectangle, and it is told where its window is.
fn sync_webview_to_client<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
    let guard = context.controller.lock().unwrap();
    let Some(controller) = guard.as_ref() else { return };
    let mut client = windows::Win32::Foundation::RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() || client.right <= 0 || client.bottom <= 0 {
        return;
    }
    let result = set_bounds_and_host(&controller.0, client.right, client.bottom);
    if let Err(error) = result {
        log::warn!("Failed to move the WebView with its window: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::scale_of_dpi;

    #[test]
    fn the_scale_of_a_dpi_is_its_share_of_96() {
        assert_eq!(scale_of_dpi(96), 1.0);
        assert_eq!(scale_of_dpi(120), 1.25);
        assert_eq!(scale_of_dpi(144), 1.5);
        assert_eq!(scale_of_dpi(192), 2.0);
        assert_eq!(scale_of_dpi(0), 1.0 / 96.0); // never a zero scale
    }
}

pub(super) fn refresh<R: Runtime>(window: &WebviewWindow<R>, panel: &Panel) {
    let revision = panel.state.lock().unwrap().pending_refresh(Instant::now());
    let Some(revision) = revision else { return };
    let panel = panel.clone();
    let window_handle = window.hwnd().ok().map(|hwnd| hwnd.0 as usize);
    let _ = window.with_webview(move |webview| {
        {
            let state = panel.state.lock().unwrap();
            if state.pending_refresh(Instant::now()) != Some(revision) {
                return;
            }
        }
        let controller = webview.controller();
        let result = unsafe {
            // the bounds are the window's client rectangle, not what the controller last held (that was the old size for 0.1 to 0.2 s after a change of scale)
            let mut client = windows::Win32::Foundation::RECT::default();
            let known = window_handle.is_some_and(|handle| {
                windows::Win32::UI::WindowsAndMessaging::GetClientRect(windows::Win32::Foundation::HWND(handle as *mut _), &mut client).is_ok()
                    && client.right > 0
                    && client.bottom > 0
            });
                let mut bounds = Default::default();
            controller
                .Bounds(&mut bounds)
                .and_then(|()| {
                    if known {
                        controller.SetBounds(windows::Win32::Foundation::RECT { left: 0, top: 0, right: client.right, bottom: client.bottom })
                    } else {
                        controller.SetBounds(bounds)
                    }
                })
                .and_then(|()| controller.NotifyParentWindowPositionChanged())
        };
        panel
            .state
            .lock()
            .unwrap()
            .refreshed(revision, result.is_ok());
        if let Err(error) = result {
            log::warn!("Failed to refresh WebView after display change: {error}");
        }
    });
}
