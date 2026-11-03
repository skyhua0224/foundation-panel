//! Native display events feed the same Panel state as window visibility, and move the WebView with the window the moment its monitor changes.
//!
//! A change of the monitor's scale (WM_DPICHANGED: another monitor, the Settings scale, the resolution of a display whose scale follows it) used to reach the
//! page in pieces: tao resized the window when the message came, the page then laid out at its old rasterization scale (a 200 % page in a 125 % window:
//! 400 x 250 CSS pixels instead of 640 x 400) for another 170 to 200 ms, and the right scale arrived only with the debounced refresh below (0.3 to 0.4 s after
//! the change; a hidden panel kept the stale layout until it was shown). WebView2 detects monitor scale changes on its own and late; the panel now turns that
//! detection off and sets the scale itself, in the message, just before the window is resized, and sets the bounds to the client rectangle right after.
//! https://learn.microsoft.com/windows/win32/hidpi/wm-dpichanged
//! https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2controller3#put_shouldDetectMonitorScaleChanges
//!
//! Two paths. The default one gives the WebView DIP bounds (BoundsMode UseRasterizationScale): its size in pixels is its bounds times the scale, so a change
//! of the scale resizes it in the same step as the scale itself and there is no moment with the new scale and the old size. For that the panel takes over wry's
//! WM_SIZE handling of this window: wry sets the bounds in physical pixels, which DIP bounds would scale a second time, so a WM_SIZE goes straight to the
//! window's own procedure (tao's, from the class) and the WebView and its host window are sized from the client rectangle here, in the same message.
//! https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2controller3#put_boundsmode
//! The other path (SUNSHINE_GUI_WEBVIEW_BOUNDS=raw) leaves the bounds in physical pixels and wry's handler in place, and sets the scale and the bounds together.
use super::dpi_cover::{self, Mode};
use super::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex as StdMutex};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_BOUNDS_MODE_USE_RASTERIZATION_SCALE, ICoreWebView2Controller, ICoreWebView2Controller3,
};
use wv2_windows_core::Interface;

/// The WebView2 controller, only ever used on the thread of the window (the message loop).
struct MainThreadController(ICoreWebView2Controller);
unsafe impl Send for MainThreadController {}

struct Hook<R: Runtime> {
    window: WebviewWindow<R>,
    panel: Panel,
    controller: Arc<StdMutex<Option<MainThreadController>>>,
    /// the WebView has DIP bounds and the panel sizes it (set once the controller is switched over)
    dip: Arc<AtomicBool>,
    /// the window's own procedure, from its class: where a WM_SIZE goes when wry's handler must not see it
    class_proc: usize,
    /// the page is hidden while WebView2 settles after a change of scale (mode Fixed)
    covered: AtomicBool,
    /// the DPI of the window, to tell a change of scale from a move between monitors of the same scale
    last_dpi: AtomicU32,
    next_token: AtomicU64,
    /// the cover window of the modes Adaptive and Snapshot
    cover: StdMutex<Cover>,
}

#[derive(Default)]
struct Cover {
    /// HWND of the cover window, 0 for none
    overlay: isize,
    /// which cover the page's report is about; 0 once it is over
    token: u64,
    began: Option<Instant>,
    fading: Option<Instant>,
}

/// Which bounds the WebView of a panel gets: physical pixels and wry's handler (the default), or DIPs with the panel's own WM_SIZE handling
/// (SUNSHINE_GUI_WEBVIEW_BOUNDS = dip). Measured on the Skyindows panel the two give the same layouts after a change of scale.
fn dip_wanted() -> bool {
    std::env::var("SUNSHINE_GUI_WEBVIEW_BOUNDS").is_ok_and(|value| value.eq_ignore_ascii_case("dip"))
}

/// While WebView2 settles after a change of scale the page is covered (SUNSHINE_GUI_WEBVIEW_DPI_COVER = off | fixed | adaptive | snapshot, see dpi_cover):
/// for 70 to 170 ms after the first right layout, WebView2 laid the page out for the old size at the new scale, whatever bounds mode or order of calls was used.
const COVER_TIMER: usize = 0x5353_5743;
const FADE_TIMER: usize = 0x5353_5744;
/// the page tells that its layout matches the window (posted by `panel_dpi_settled`)
const WM_SETTLED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 0x53;
/// a snapshot of the page is ready (wparam: the cover's token, lparam: a boxed `dpi_cover::Frame`, or 0 when there is none)
const WM_SNAPSHOT: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 0x54;

/// The size in DIPs whose size in pixels at `scale` covers `pixels` (the host window clips the pixel that rounding may add).
pub(super) fn dip_extent(pixels: i32, scale: f64) -> i32 {
    ((f64::from(pixels.max(1)) / scale.max(0.01)).ceil() as i32).max(1)
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
        let dip = Arc::new(AtomicBool::new(false));
        let dip_slot = dip.clone();
        let hwnd_address = hwnd.0 as usize;
        let class_proc = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetClassLongPtrW(
                windows::Win32::Foundation::HWND(hwnd.0),
                windows::Win32::UI::WindowsAndMessaging::GCLP_WNDPROC,
            )
        };
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
                    if dip_wanted() && class_proc != 0 && controller3.SetBoundsMode(COREWEBVIEW2_BOUNDS_MODE_USE_RASTERIZATION_SCALE).is_ok() {
                        dip_slot.store(true, AtomicOrdering::SeqCst);
                    }
                }
            }
            if dip_slot.load(AtomicOrdering::SeqCst) {
                // wry set its bounds in pixels before this: they are DIPs from now on
                sync_to_client(&controller, windows::Win32::Foundation::HWND(hwnd_address as *mut _));
            }
            *slot.lock().unwrap() = Some(MainThreadController(controller));
        });
        let context = Box::into_raw(Box::new(Hook { window, panel, controller, dip, class_proc, covered: AtomicBool::new(false), last_dpi: AtomicU32::new(dpi_of_window), next_token: AtomicU64::new(0), cover: StdMutex::new(Cover::default()) }));
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
    if message == windows::Win32::UI::WindowsAndMessaging::WM_SIZE
        && wparam.0 != windows::Win32::UI::WindowsAndMessaging::SIZE_MINIMIZED as usize
        && context.dip.load(AtomicOrdering::SeqCst)
    {
        // not through wry's handler (it sets pixel bounds): the window's own procedure, then the WebView from the client rectangle
        let result = unsafe {
            let proc: windows::Win32::UI::WindowsAndMessaging::WNDPROC = std::mem::transmute(context.class_proc);
            windows::Win32::UI::WindowsAndMessaging::CallWindowProcW(proc, hwnd, message, wparam, lparam)
        };
        sync_webview_to_client(hwnd, context);
        fit_cover(hwnd, context);
        return result;
    }
    if message == WM_DPICHANGED {
        let dpi = u32::from((wparam.0 & 0xFFFF) as u16);
        let from = context.last_dpi.swap(dpi, AtomicOrdering::SeqCst);
        begin_cover(hwnd, context, from, dpi);
        apply_new_dpi(hwnd, context, wparam, lparam);
    }
    match message {
        windows::Win32::UI::WindowsAndMessaging::WM_TIMER if wparam.0 == COVER_TIMER => {
            if dpi_cover::mode() == Mode::Fixed {
                end_cover(hwnd, context);
            } else {
                start_fade(hwnd, context, "cap");
            }
            return windows::Win32::Foundation::LRESULT(0);
        }
        windows::Win32::UI::WindowsAndMessaging::WM_TIMER if wparam.0 == FADE_TIMER => {
            fade_tick(hwnd, context);
            return windows::Win32::Foundation::LRESULT(0);
        }
        WM_SETTLED => {
            if context.cover.lock().unwrap().token == wparam.0 as u64 {
                start_fade(hwnd, context, "page settled");
            }
            return windows::Win32::Foundation::LRESULT(0);
        }
        WM_SNAPSHOT => {
            let frame = (lparam.0 != 0).then(|| unsafe { Box::from_raw(lparam.0 as *mut dpi_cover::Frame) });
            snapshot_ready(hwnd, context, wparam.0 as u64, frame.as_deref());
            return windows::Win32::Foundation::LRESULT(0);
        }
        WM_SHOWWINDOW if wparam.0 == 0 => abort_cover(hwnd, context),
        _ => {}
    }
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == windows::Win32::UI::WindowsAndMessaging::WM_SIZE {
        if wparam.0 == windows::Win32::UI::WindowsAndMessaging::SIZE_MINIMIZED as usize {
            abort_cover(hwnd, context);
        } else {
            fit_cover(hwnd, context);
        }
    }
    if matches!(message, WM_DISPLAYCHANGE | WM_DPICHANGED) {
        // a new resolution or scale moved or resized the window: the WebView follows in the same message
        sync_webview_to_client(hwnd, context);
        if message == WM_DPICHANGED {
            fit_cover(hwnd, context);
            announce_cover(hwnd, context);
        }
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

/// The page is covered from the change of scale until WebView2 has settled: hidden for a fixed time (its window shows the surface colour), or under a cover
/// window that fades out when the page says its layout matches the window.
fn begin_cover<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>, from_dpi: u32, to_dpi: u32) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible, SetTimer};
    let mode = dpi_cover::mode();
    // a move between monitors of the same scale changes nothing for the page
    if mode == Mode::Off || from_dpi == to_dpi || !unsafe { IsWindowVisible(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool() {
        return;
    }
    if context.panel.state.lock().unwrap().phase != Phase::Active {
        return;
    }
    log::info!("[dpi-cover] {mode:?} begin, dpi {from_dpi} -> {to_dpi}");
    if mode == Mode::Fixed {
        if let Some(controller) = context.controller.lock().unwrap().as_ref() {
            if unsafe { controller.0.SetIsVisible(false) }.is_ok() {
                context.covered.store(true, AtomicOrdering::SeqCst);
                unsafe { SetTimer(Some(hwnd), COVER_TIMER, dpi_cover::CAP_MS, None) };
            }
        }
        return;
    }
    let token = context.next_token.fetch_add(1, AtomicOrdering::SeqCst) + 1;
    {
        let mut cover = context.cover.lock().unwrap();
        drop_overlay(&mut cover);
        *cover = Cover { overlay: 0, token, began: Some(Instant::now()), fading: None };
    }
    unsafe { SetTimer(Some(hwnd), COVER_TIMER, dpi_cover::CAP_MS, None) };
    if mode == Mode::Adaptive {
        let overlay = dpi_cover::create(hwnd, crate::windows::surface::current(), None);
        context.cover.lock().unwrap().overlay = overlay.map_or(0, |overlay| overlay.0 as isize);
        return;
    }
    // Snapshot: the cover is put up when the frame is ready; the page shows until then
    let address = hwnd.0 as usize;
    let asked = Instant::now();
    let captured = context.controller.lock().unwrap().as_ref().map(|controller| {
        dpi_cover::capture(&controller.0, move |frame| {
            log::info!("[dpi-cover] snapshot {} after {} ms", frame.as_ref().map_or("failed".to_string(), |frame| format!("{}x{}", frame.width, frame.height)), asked.elapsed().as_millis());
            let pointer = frame.map_or(0, |frame| Box::into_raw(Box::new(frame)) as isize);
            let posted = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(windows::Win32::Foundation::HWND(address as *mut _)),
                    WM_SNAPSHOT,
                    windows::Win32::Foundation::WPARAM(token as usize),
                    windows::Win32::Foundation::LPARAM(pointer),
                )
            };
            if posted.is_err() && pointer != 0 {
                drop(unsafe { Box::from_raw(pointer as *mut dpi_cover::Frame) });
            }
        })
    });
    if !matches!(captured, Some(Ok(()))) {
        // no frame to show: the surface colour covers
        snapshot_ready(hwnd, context, token, None);
    }
}

/// The frame of the page is ready (or there is none and the surface colour covers): the cover goes up if that cover is still the one wanted.
fn snapshot_ready<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>, token: u64, frame: Option<&dpi_cover::Frame>) {
    let mut cover = context.cover.lock().unwrap();
    if cover.token != token || cover.overlay != 0 || cover.fading.is_some() {
        return;
    }
    let overlay = dpi_cover::create(hwnd, crate::windows::surface::current(), frame);
    cover.overlay = overlay.map_or(0, |overlay| overlay.0 as isize);
    if let Some(began) = cover.began {
        log::info!("[dpi-cover] cover up {} ms after the change", began.elapsed().as_millis());
    }
}

fn drop_overlay(cover: &mut Cover) {
    if cover.overlay != 0 {
        dpi_cover::destroy(windows::Win32::Foundation::HWND(cover.overlay as *mut _));
        cover.overlay = 0;
    }
}

/// The cover goes at once (the window was hidden or minimized).
fn abort_cover<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;
    let mut cover = context.cover.lock().unwrap();
    if cover.token == 0 && cover.overlay == 0 {
        return;
    }
    drop_overlay(&mut cover);
    *cover = Cover::default();
    unsafe {
        let _ = KillTimer(Some(hwnd), COVER_TIMER);
        let _ = KillTimer(Some(hwnd), FADE_TIMER);
    }
}

fn fit_cover<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    let cover = context.cover.lock().unwrap();
    if cover.overlay != 0 {
        dpi_cover::fit(windows::Win32::Foundation::HWND(cover.overlay as *mut _), hwnd);
    }
}

/// Tells the page which size its layout has to match, and which cover the report is about.
fn announce_cover<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    use tauri::Emitter;
    let token = {
        let cover = context.cover.lock().unwrap();
        if cover.began.is_none() || cover.fading.is_some() {
            return;
        }
        cover.token
    };
    let mut client = windows::Win32::Foundation::RECT::default();
    if unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client) }.is_ok() {
        let _ = context.window.emit_to(
            context.window.label().to_string(),
            "panel-dpi-cover",
            serde_json::json!({ "width": client.right, "height": client.bottom, "token": token }),
        );
    }
}

fn start_fade<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>, reason: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};
    let mut cover = context.cover.lock().unwrap();
    let _ = unsafe { KillTimer(Some(hwnd), COVER_TIMER) };
    if cover.fading.is_some() {
        return;
    }
    if let Some(began) = cover.began {
        log::info!("[dpi-cover] {reason} {} ms after the change, fading", began.elapsed().as_millis());
    }
    if cover.overlay == 0 {
        // the frame never came: nothing to fade, and a late one is not wanted
        *cover = Cover::default();
        return;
    }
    cover.fading = Some(Instant::now());
    unsafe { SetTimer(Some(hwnd), FADE_TIMER, 10, None) };
}

fn fade_tick<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;
    let mut cover = context.cover.lock().unwrap();
    let Some(started) = cover.fading else {
        let _ = unsafe { KillTimer(Some(hwnd), FADE_TIMER) };
        return;
    };
    let elapsed = started.elapsed().as_millis();
    if cover.overlay != 0 {
        dpi_cover::set_opacity(windows::Win32::Foundation::HWND(cover.overlay as *mut _), dpi_cover::fade_alpha(elapsed));
    }
    if elapsed >= dpi_cover::FADE_MS {
        let total = cover.began.map(|began| began.elapsed().as_millis());
        drop_overlay(&mut cover);
        *cover = Cover::default();
        let _ = unsafe { KillTimer(Some(hwnd), FADE_TIMER) };
        log::info!("[dpi-cover] cover gone {} ms after the change", total.unwrap_or(0));
    }
}

/// The page's report reaches the window thread through a message.
pub(super) fn post_settled<R: Runtime>(window: &WebviewWindow<R>, token: u64) {
    if let Ok(hwnd) = window.hwnd() {
        let _ = unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(windows::Win32::Foundation::HWND(hwnd.0)),
                WM_SETTLED,
                windows::Win32::Foundation::WPARAM(token as usize),
                windows::Win32::Foundation::LPARAM(0),
            )
        };
    }
}

fn end_cover<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible, KillTimer};
    let _ = unsafe { KillTimer(Some(hwnd), COVER_TIMER) };
    if !context.covered.swap(false, AtomicOrdering::SeqCst) {
        return;
    }
    log::info!("[dpi-cover] Fixed end");
    // a panel that went to the background meanwhile stays as the lifecycle left it
    let shown = unsafe { IsWindowVisible(hwnd) }.as_bool() && !unsafe { IsIconic(hwnd) }.as_bool();
    if shown && context.panel.state.lock().unwrap().phase == Phase::Active {
        if let Some(controller) = context.controller.lock().unwrap().as_ref() {
            let _ = unsafe { controller.0.SetIsVisible(true) };
        }
    }
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
    // DIP bounds: the size follows the scale by itself, and the WM_SIZE of the resize that comes next sets the bounds for it
    if context.dip.load(AtomicOrdering::SeqCst) {
        return;
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
        // pixels, or DIPs when the controller is in the mode where its bounds are DIPs
        let (mut right, mut bottom) = (width, height);
        if let Ok(controller3) = controller.cast::<ICoreWebView2Controller3>() {
            let mut mode = Default::default();
            if controller3.BoundsMode(&mut mode).is_ok() && mode == COREWEBVIEW2_BOUNDS_MODE_USE_RASTERIZATION_SCALE {
                let mut scale = 1.0;
                if controller3.RasterizationScale(&mut scale).is_ok() {
                    (right, bottom) = (dip_extent(width, scale), dip_extent(height, scale));
                }
            }
        }
        // The bounds that are already there are not set again: WebView2 laid the page out for the old size at the new scale for 0.1 to 0.2 s after such a call
        // that came with a change of scale (measured: the page jumped to 854 x 534 CSS pixels in a 640 x 400 window).
        let mut current = windows::Win32::Foundation::RECT::default();
        let unchanged = controller.Bounds(&mut current).is_ok() && (current.left, current.top, current.right, current.bottom) == (0, 0, right, bottom);
        if !unchanged {
            controller.SetBounds(windows::Win32::Foundation::RECT { left: 0, top: 0, right, bottom })?;
        }
        let mut host = windows::Win32::Foundation::HWND::default();
        if controller.ParentWindow(&mut host).is_ok() && !host.is_invalid() {
            let _ = SetWindowPos(host, None, 0, 0, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    Ok(())
}

/// The WebView's bounds are the window's client rectangle, and its host window has the same size.
fn sync_webview_to_client<R: Runtime>(hwnd: windows::Win32::Foundation::HWND, context: &Hook<R>) {
    let guard = context.controller.lock().unwrap();
    let Some(controller) = guard.as_ref() else { return };
    sync_to_client(&controller.0, hwnd);
}

fn sync_to_client(controller: &ICoreWebView2Controller, hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
    let mut client = windows::Win32::Foundation::RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() || client.right <= 0 || client.bottom <= 0 {
        return;
    }
    if let Err(error) = set_bounds_and_host(controller, client.right, client.bottom) {
        log::warn!("Failed to move the WebView with its window: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{dip_extent, scale_of_dpi};

    #[test]
    fn dip_bounds_cover_the_pixels_at_every_scale() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0] {
            for pixels in [1, 480, 960, 1279, 1280, 1920, 2564, 3840] {
                let dips = dip_extent(pixels, scale);
                assert!(f64::from(dips) * scale >= f64::from(pixels), "{pixels} px at {scale}: {dips} DIPs");
                assert!(f64::from(dips - 1) * scale < f64::from(pixels), "{pixels} px at {scale}: {dips} DIPs is more than needed");
            }
        }
        assert_eq!(dip_extent(1280, 2.0), 640);
        assert_eq!(dip_extent(960, 1.5), 640);
        assert_eq!(dip_extent(0, 1.5), 1);
    }

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
        // the bounds are the window's client rectangle (DIPs or pixels as the controller has them), not what the controller last held
        let mut client = windows::Win32::Foundation::RECT::default();
        let known = window_handle.is_some_and(|handle| {
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(windows::Win32::Foundation::HWND(handle as *mut _), &mut client) }.is_ok()
                && client.right > 0
                && client.bottom > 0
        });
        let result = if known {
            set_bounds_and_host(&controller, client.right, client.bottom).and_then(|()| unsafe { controller.NotifyParentWindowPositionChanged() })
        } else {
            unsafe { controller.NotifyParentWindowPositionChanged() }
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
