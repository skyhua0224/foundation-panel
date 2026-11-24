//! The setup UI process of the Foundation Sunshine installer.
//!
//! The installer shell starts this exe with its stdin and stdout on pipes:
//!   stdin  (shell -> UI): one JSON object per line: {"type":"view","view":InstallerView} | {"type":"minimise"} |
//!                         {"type":"focus"}; end of stream = close the window and exit.
//!   stdout (UI -> shell): {"type":"hello","hwnd":N} | {"type":"loaded"} | {"type":"action","action":{...}}
//! The page reaches it with invoke('installer_view'), invoke('installer_action', {action}) and the
//! 'installer://view' event (foundation-ui src/vue/installer/bridge.js).
//!
//! Started without a shell (stdin is not a pipe from the shell) it still opens; the page then sees no view and the
//! bridge's mock is not used (Tauri is present), so this is for development only.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const WINDOW_ID: &str = "main";
/// Logical size of the installer surface, the same as the native shell (tools/installer_d2d.h kW, kH). The window is
/// transparent and larger by ROOM on every side: the page (foundation-ui MInstaller, windowed) draws the rounded surface
/// and its shadow in that room (--inst-room in installer.css).
const WIDTH: f64 = 820.0;
const HEIGHT: f64 = 560.0;
const ROOM: f64 = 12.0;
/// The smallest surface the pages are laid out for (installer.css container queries).
const MIN_WIDTH: f64 = 480.0;
const MIN_HEIGHT: f64 = 360.0;

struct Host {
    last: Mutex<Value>,
    out: Mutex<std::io::Stdout>,
    shown: AtomicBool,
}

/// One line to the shell, "type" first. serde_json orders keys alphabetically, so {"action":{"type":..},"type":..}
/// would put the nested "type" first, and a shell that reads the first "type" (every shell before
/// installer_json.h) dropped every action.
fn wire_line(message: Value) -> String {
    match message {
        Value::Object(mut map) => match map.remove("type") {
            Some(kind) => {
                let whole = Value::Object(map).to_string();
                let rest = whole.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or("");
                if rest.is_empty() { format!("{{\"type\":{kind}}}") } else { format!("{{\"type\":{kind},{rest}}}") }
            }
            None => Value::Object(map).to_string(),
        },
        other => other.to_string(),
    }
}

impl Host {
    fn send(&self, message: Value) {
        let line = wire_line(message);
        if let Ok(mut out) = self.out.lock() {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    }
}

// Both commands are async: they run on the async runtime, never on the main thread, so nothing the window does on the
// main thread (a focus change, a modal loop) can hold an action back.
#[tauri::command]
async fn installer_view(host: tauri::State<'_, Host>) -> Result<Value, String> {
    Ok(host.last.lock().map(|v| v.clone()).unwrap_or(Value::Null))
}

#[tauri::command]
async fn installer_action(window: WebviewWindow, host: tauri::State<'_, Host>, action: Value) -> Result<(), String> {
    // The shell hears the action first: nothing below may delay or swallow it.
    host.send(json!({ "type": "action", "action": action }));
    let kind = action.get("type").and_then(Value::as_str).unwrap_or("").to_string();
    if kind == "minimise" {
        let _ = window.minimize();
    }
    // 'ready', and any action at all (the page is live, whatever happened to 'ready'), shows the window.
    reveal(&window, &host, if kind == "ready" { "ready" } else { "action" });
    Ok(())
}

/// The window is shown cloaked from the start (WebView2 renders only for a shown window) and uncloaked when the page
/// says it has painted ('ready'), on its first action, or 1.5 s after the page loaded (watchdog), whichever is first.
/// Every reveal is logged to the shell (%TEMP%\FoundationSunshine-setup-web.log).
fn reveal(window: &WebviewWindow, host: &Host, reason: &str) {
    if host.shown.swap(true, Ordering::SeqCst) {
        return;
    }
    host.send(json!({ "type": "log", "text": format!("reveal by {reason}") }));
    #[cfg(windows)]
    {
        set_cloak(window, false);
        // the WebView renders and takes input for a visible controller: say so explicitly after the uncloak
        let _ = window.with_webview(|webview| unsafe {
            let _ = webview.controller().SetIsVisible(true);
        });
        if !test_offscreen() {
            // The shell allowed this process to come to the front (AllowSetForegroundWindow). A plain
            // SetForegroundWindow, not tao's set_focus (which synthesises an Alt key press).
            use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
            if let Some(hwnd) = hwnd_of(window) {
                let _ = unsafe { SetForegroundWindow(hwnd) };
            }
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// Test switch, as the Panel's SUNSHINE_GUI_TEST_OFFSCREEN: FOUNDATION_SETUP_TEST_OFFSCREEN=1 opens the window at
/// 6000,100 (outside every monitor), shown, never activated, so a guarded measurement instance in a person's session
/// never covers their screen or takes their focus. Never set by the installer.
fn test_offscreen() -> bool {
    std::env::var_os("FOUNDATION_SETUP_TEST_OFFSCREEN").is_some_and(|v| v == "1")
}

/// Test instance only: a physical position right of every monitor of the desktop (whatever their arrangement and
/// scale), so the window is on none of them. 6000,100 in logical pixels was not enough: it landed on a monitor.
fn offscreen_position(app: &tauri::App) -> tauri::PhysicalPosition<i32> {
    let monitors = app.available_monitors().unwrap_or_default();
    let right = monitors.iter().map(|m| m.position().x + m.size().width as i32).max().unwrap_or(0);
    let top = monitors.iter().map(|m| m.position().y).min().unwrap_or(0);
    tauri::PhysicalPosition::new(right + 4000, top)
}

/// Test instance only: a window that is on any monitor ends the process at once (it must never be seen).
/// MonitorFromWindow with MONITOR_DEFAULTTONULL is null for a window that intersects no monitor.
#[cfg(windows)]
fn guard_offscreen(window: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONULL};
    let Some(hwnd) = hwnd_of(window) else { std::process::exit(4) };
    let raw = hwnd.0 as isize;
    let on_screen = move || unsafe { !MonitorFromWindow(HWND(raw as *mut _), MONITOR_DEFAULTTONULL).is_invalid() };
    if on_screen() {
        eprintln!("test instance would be on a monitor: exiting");
        std::process::exit(4);
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        if on_screen() {
            eprintln!("test instance moved onto a monitor: exiting");
            std::process::exit(4);
        }
    });
}

/// Evidence for tests (never set in a real install): FOUNDATION_SETUP_UI_SHOTS=<dir> writes a PNG of the page
/// (WebView2 CapturePreview, the page as rendered, without the DWM frame) 900 ms after each view;
/// FOUNDATION_SETUP_UI_DRIVE=a,b,toggle-component:usbip sends those actions one per shot, as the page would.
struct Evidence {
    dir: PathBuf,
    drive: Mutex<std::collections::VecDeque<String>>,
    count: std::sync::atomic::AtomicU32,
}

fn evidence() -> Option<&'static Evidence> {
    static E: std::sync::OnceLock<Option<Evidence>> = std::sync::OnceLock::new();
    E.get_or_init(|| {
        let dir = PathBuf::from(std::env::var_os("FOUNDATION_SETUP_UI_SHOTS")?);
        let _ = std::fs::create_dir_all(&dir);
        let drive = std::env::var("FOUNDATION_SETUP_UI_DRIVE").unwrap_or_default();
        Some(Evidence {
            dir,
            drive: Mutex::new(drive.split(',').filter(|a| !a.is_empty()).map(String::from).collect()),
            count: std::sync::atomic::AtomicU32::new(0),
        })
    })
    .as_ref()
}

fn evidence_after_view(window: &WebviewWindow, page: &str) {
    let Some(e) = evidence() else { return };
    let n = e.count.fetch_add(1, Ordering::SeqCst);
    let _path = e.dir.join(format!("{n:02}-{page}.png"));
    let w = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(900));
        #[cfg(windows)]
        capture_page(&w, _path);
        std::thread::sleep(Duration::from_millis(400));
        let next = e.drive.lock().ok().and_then(|mut q| q.pop_front());
        if let Some(next) = next {
            let action = match next.split_once(':') {
                Some((t, id)) => json!({ "type": t, "id": id }),
                None => json!({ "type": next }),
            };
            w.state::<Host>().send(json!({ "type": "action", "action": action }));
        }
    });
}

#[cfg(windows)]
fn capture_page(window: &WebviewWindow, path: PathBuf) {
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
        use windows::core::{HSTRING, Interface};
        use windows::Win32::System::Com::{IStream, STGM_CREATE, STGM_WRITE};
        use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
        let Ok(core) = webview.controller().CoreWebView2() else { return };
        let Ok(stream) = SHCreateStreamOnFileEx(&HSTRING::from(path.as_os_str()), (STGM_CREATE | STGM_WRITE).0, 0x80, true, None::<&IStream>) else { return };
        let keep = stream.clone();
        let handler = webview2_com::CapturePreviewCompletedHandler::create(Box::new(move |_result| {
            let _ = keep.cast::<IStream>().map(|s| s.Commit(windows::Win32::System::Com::STGC_DEFAULT));
            Ok(())
        }));
        let _ = core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &handler);
    });
}

/// The window has a fixed size. Its drag region would maximise it on a double click: Windows does that for a
/// caption hit (WM_NCLBUTTONDBLCLK, then WM_SYSCOMMAND SC_MAXIMIZE), and Tauri's drag-region script asks for it
/// (internal_toggle_maximize, also not granted in capabilities/default.json). The subclass drops both
/// (installer-ms-setwindowsubclass, installer-ms-wm-syscommand).
#[cfg(windows)]
fn keep_fixed_size(window: &WebviewWindow) {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, STYLESTRUCT,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION,
        WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };
    const NON_CLIENT_STYLE: u32 =
        WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_THICKFRAME.0;
    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        const WM_NCLBUTTONDBLCLK: u32 = 0x00A3;
        const WM_SYSCOMMAND: u32 = 0x0112;
        const WM_STYLECHANGING: u32 = 0x007C;
        const SC_MAXIMIZE: usize = 0xF030;
        if msg == WM_NCLBUTTONDBLCLK || (msg == WM_SYSCOMMAND && (wparam.0 & 0xFFF0) == SC_MAXIMIZE)
        {
            return LRESULT(0);
        }
        // The window is transparent (DWM blur-behind with an empty region) and has no frame of its own: whatever the
        // system paints into its non-client area, or erases its background with, shows as a stray strip over the
        // transparent margin. None of it is painted (WM_NCACTIVATE with lParam -1 stops the default handler from
        // redrawing the frame).
        const WM_ERASEBKGND: u32 = 0x0014;
        const WM_NCPAINT: u32 = 0x0085;
        const WM_NCACTIVATE: u32 = 0x0086;
        if msg == WM_NCPAINT {
            return LRESULT(0);
        }
        if msg == WM_ERASEBKGND {
            return LRESULT(1);
        }
        if msg == WM_NCACTIVATE {
            return unsafe { DefSubclassProc(hwnd, msg, wparam, LPARAM(-1)) };
        }
        // The page owns the entire surface, including its title bar. Keep tao/WebView2 from restoring any
        // system caption or frame after the window has been created.
        if msg == WM_STYLECHANGING && wparam.0 as i32 == GWL_STYLE.0 {
            let change = lparam.0 as *mut STYLESTRUCT;
            if !change.is_null() {
                unsafe { (*change).styleNew &= !NON_CLIENT_STYLE };
            }
        }
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(proc), 0x5e7, 0);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(NON_CLIENT_STYLE as isize));
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

/// It is an application window, never a browser: no context menu, no DevTools (release), no browser keys (reload,
/// find, print, view source, zoom keys and Ctrl+wheel), no zoom or pinch, no swipe navigation, no status bubble, no
/// built-in error page, no autofill or password prompts; a new window or a navigation away from the app opens http(s)
/// in the system browser instead and anything else is dropped; no download bubble. As the Panel's
/// harden_browser_surface (src-tauri/src/windows.rs). Authority: installer-ms-icorewebview2settings*.
#[cfg(windows)]
fn harden_webview(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2Settings3, ICoreWebView2Settings4, ICoreWebView2Settings5, ICoreWebView2Settings6, ICoreWebView2_4,
        };
        use webview2_com::{DownloadStartingEventHandler, NavigationStartingEventHandler, NewWindowRequestedEventHandler, take_pwstr};
        use windows::core::{Interface, PWSTR};
        let Ok(core) = webview.controller().CoreWebView2() else { return };
        if let Ok(settings) = core.Settings() {
            let _ = settings.SetAreDefaultContextMenusEnabled(false);
            let _ = settings.SetAreDevToolsEnabled(cfg!(debug_assertions));
            let _ = settings.SetIsStatusBarEnabled(false);
            let _ = settings.SetIsZoomControlEnabled(false);
            let _ = settings.SetIsBuiltInErrorPageEnabled(false);
            if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
                let _ = s3.SetAreBrowserAcceleratorKeysEnabled(cfg!(debug_assertions));
            }
            if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
                let _ = s4.SetIsGeneralAutofillEnabled(false);
                let _ = s4.SetIsPasswordAutosaveEnabled(false);
            }
            if let Ok(s5) = settings.cast::<ICoreWebView2Settings5>() {
                let _ = s5.SetIsPinchZoomEnabled(false);
            }
            if let Ok(s6) = settings.cast::<ICoreWebView2Settings6>() {
                let _ = s6.SetIsSwipeNavigationEnabled(false);
            }
        }
        let new_window = NewWindowRequestedEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args {
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let target = take_pwstr(uri);
                args.SetHandled(true)?;
                open_external(&target);
            }
            Ok(())
        }));
        let mut token = 0;
        let _ = core.add_NewWindowRequested(&new_window, &mut token);
        let navigation = NavigationStartingEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args {
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let target = take_pwstr(uri);
                if !is_app_url(&target) {
                    args.SetCancel(true)?;
                    open_external(&target);
                }
            }
            Ok(())
        }));
        let _ = core.add_NavigationStarting(&navigation, &mut token);
        if let Ok(core4) = core.cast::<ICoreWebView2_4>() {
            let download = DownloadStartingEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    args.SetCancel(true)?;
                    args.SetHandled(true)?;
                }
                Ok(())
            }));
            let _ = core4.add_DownloadStarting(&download, &mut token);
        }
    });
}

/// The pages of this app: tauri.localhost (Tauri 2 on Windows) and about:blank.
fn is_app_url(url: &str) -> bool {
    url.starts_with("http://tauri.localhost") || url.starts_with("https://tauri.localhost") || url.starts_with("tauri://") || url == "about:blank"
}

/// An http(s) address in the system browser (the person's own browser, unelevated like this process); nothing else.
#[cfg(windows)]
fn open_external(url: &str) {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if !(url.starts_with("https://") || url.starts_with("http://")) || test_offscreen() {
        return;
    }
    unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(url), windows::core::PCWSTR::null(), windows::core::PCWSTR::null(), SW_SHOWNORMAL);
    }
}

fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

#[cfg(windows)]
fn hwnd_of(window: &WebviewWindow) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(windows::Win32::Foundation::HWND(h.hwnd.get() as *mut _)),
        _ => None,
    }
}

/// DWMWA_CLOAK (13), as the Panel's cold start (src-tauri/src/windows.rs set_cloak).
#[cfg(windows)]
fn set_cloak(window: &WebviewWindow, cloaked: bool) {
    use windows::core::BOOL;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    if let Some(hwnd) = hwnd_of(window) {
        let value = BOOL::from(cloaked);
        let _ = unsafe { DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(13), (&value as *const BOOL).cast(), std::mem::size_of::<BOOL>() as u32) };
    }
}

/// The page draws the window's rounded shape (radius = card radius + inset) on a transparent window, so DWM must not
/// clip it with its own 8 px corner: DWMWA_WINDOW_CORNER_PREFERENCE (33) = DWMWCP_DONOTROUND (1).
/// Authority: installer-ms-dwmwindowattribute.
#[cfg(windows)]
fn apply_dwm_corners(window: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    if let Some(hwnd) = hwnd_of(window) {
        let preference: u32 = 1;
        let _ = unsafe { DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(33), (&preference as *const u32).cast(), std::mem::size_of::<u32>() as u32) };
    }
}

/// How long the process has to end once the shell's stream has ended. The exit tears the WebView2 and the runtime down; a teardown that does not finish
/// leaves the window as the system draws a window that nobody answers for (a flat surface under the old light title bar with its three square buttons, the
/// title and the icon) for as long as the process lives. So the window is hidden first, and the process is ended hard if it has not ended by itself.
const EXIT_WATCHDOG: Duration = Duration::from_millis(2500);

/// Ends the process now, without the teardown (TerminateProcess; the exit code is not read by the shell).
fn end_process() -> ! {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        let _ = TerminateProcess(GetCurrentProcess(), 0);
    }
    std::process::exit(0)
}

/// The shell is done with us: out of sight first (cloaked, hidden), a watchdog that ends the process, then the exit. Each step is a line of the shell's log
/// (%TEMP%\FoundationSunshine-setup-web.log), so a window that stayed can be followed.
fn end_of_stream(handle: &tauri::AppHandle) {
    let say = |text: &str| handle.state::<Host>().send(json!({ "type": "log", "text": format!("exit: {text}") }));
    say("the shell's stream ended");
    let watchdog = handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(EXIT_WATCHDOG);
        watchdog.state::<Host>().send(json!({ "type": "log", "text": format!("exit: still running {} ms after the end of the stream, ending the process", EXIT_WATCHDOG.as_millis()) }));
        end_process();
    });
    if let Some(window) = handle.get_webview_window(WINDOW_ID) {
        #[cfg(windows)]
        set_cloak(&window, true);
        let _ = window.hide();
        say("window hidden");
    }
    handle.exit(0);
    say("exit requested");
}

fn main() {
    let data_dir = arg_value("--data-dir").map(PathBuf::from);
    tauri::Builder::default()
        .manage(Host { last: Mutex::new(Value::Null), out: Mutex::new(std::io::stdout()), shown: AtomicBool::new(false) })
        .invoke_handler(tauri::generate_handler![installer_view, installer_action])
        .setup(move |app| {
            let mut builder = WebviewWindowBuilder::new(app, WINDOW_ID, WebviewUrl::App("index.html".into()))
                .title("Foundation Sunshine")
                .inner_size(WIDTH + 2.0 * ROOM, HEIGHT + 2.0 * ROOM)
                .min_inner_size(MIN_WIDTH + 2.0 * ROOM, MIN_HEIGHT + 2.0 * ROOM)
                .resizable(false)
                .maximizable(false)
                // No OS caption or frame at all: the page draws the title bar, its buttons, the rounded surface and the
                // shadow on a transparent window (as the Panel's popup windows, ctx_popup.rs).
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .visible(false)
                .skip_taskbar(test_offscreen())
                .focused(!test_offscreen())
                .disable_drag_drop_handler()
                // wry's own switches, plus: a window that is cloaked (or covered) keeps rendering, so the page's first
                // frame and its 'ready' never wait for the window to be shown (Chromium native window occlusion)
                .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion --disable-backgrounding-occluded-windows")
                .on_page_load(|window, payload| {
                    if payload.event() == tauri::webview::PageLoadEvent::Finished {
                        let host = window.state::<Host>();
                        host.send(json!({ "type": "loaded" }));
                        let w = window.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(1500));
                            let host = w.state::<Host>();
                            reveal(&w, &host, "watchdog (no ready 1.5 s after load)");
                        });
                    }
                });
            builder = if test_offscreen() { builder } else { builder.center() };
            if let Some(dir) = data_dir.clone() {
                builder = builder.data_directory(dir);
            }
            let window = builder.build()?;
            if test_offscreen() {
                let _ = window.set_position(offscreen_position(app));
                #[cfg(windows)]
                guard_offscreen(&window);
            }
            #[cfg(windows)]
            {
                harden_webview(&window);
                apply_dwm_corners(&window);
                keep_fixed_size(&window);
                set_cloak(&window, true);
            }
            // A test instance is shown without activation (Tauri show() activates even with focused(false)): it must
            // never take the focus of the person whose session it runs in.
            #[cfg(windows)]
            if test_offscreen() {
                use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
                if let Some(hwnd) = hwnd_of(&window) {
                    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
                }
            } else {
                let _ = window.show();
            }
            #[cfg(not(windows))]
            let _ = window.show();
            #[cfg(windows)]
            if let Some(hwnd) = hwnd_of(&window) {
                app.state::<Host>().send(json!({ "type": "hello", "hwnd": hwnd.0 as isize }));
            }

            // Lines from the shell. End of stream: the shell is done with us.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    let Ok(line) = line else { break };
                    let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
                    let Some(window) = handle.get_webview_window(WINDOW_ID) else { continue };
                    match message.get("type").and_then(Value::as_str).unwrap_or("") {
                        "view" => {
                            let view = message.get("view").cloned().unwrap_or(Value::Null);
                            if let Ok(mut last) = handle.state::<Host>().last.lock() {
                                *last = view.clone();
                            }
                            let page = view.get("page").and_then(Value::as_str).unwrap_or("view").to_string();
                            let _ = window.emit("installer://view", view);
                            evidence_after_view(&window, &page);
                        }
                        "minimise" => {
                            let _ = window.minimize();
                        }
                        "focus" if !test_offscreen() => {
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        _ => {}
                    }
                }
                end_of_stream(&handle);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Alt+F4 and the taskbar's Close go to the shell like the title bar's close button: it decides
            // (exit, or minimise while the worker runs).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.state::<Host>().send(json!({ "type": "action", "action": { "type": "exit" } }));
            }
        })
        .run(tauri::generate_context!())
        .expect("the setup UI could not start");
}

#[cfg(test)]
mod tests {
    use super::wire_line;
    use serde_json::{json, Value};

    /// The end of the shell's stream: the watchdog that ends the process is armed before anything that can wait on the window, and the window is hidden
    /// before the exit is requested (a window that is torn down while it is visible stays as a flat surface in the system's old frame).
    #[test]
    fn the_end_of_the_stream_arms_the_watchdog_then_hides_then_exits() {
        let source = include_str!("main.rs");
        let body = source.split("fn end_of_stream").nth(1).and_then(|rest| rest.split("\n}\n").next()).expect("end_of_stream");
        let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle} is missing from end_of_stream"));
        assert!(at("std::thread::spawn") < at("window.hide()"), "the watchdog is armed before the window is touched");
        assert!(at("window.hide()") < at("handle.exit(0)"), "the window is hidden before the exit");
        assert!(at("end_process()") < at("window.hide()"), "the watchdog thread ends the process");
    }

    #[test]
    fn the_type_is_the_first_key_and_the_line_is_the_same_json() {
        for v in [
            json!({"type": "action", "action": {"type": "toggle-component", "id": "usbip"}}),
            json!({"type": "action", "action": {"type": "ready"}}),
            json!({"type": "loaded"}),
            json!({"type": "hello", "hwnd": 12}),
            json!({"type": "log", "text": "a \"b\" {c}"}),
        ] {
            let line = wire_line(v.clone());
            assert!(line.starts_with(&format!("{{\"type\":{}", v["type"])), "{line}");
            assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), v);
        }
    }
}
