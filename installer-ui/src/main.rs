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
/// Logical size of the installer window, the same as the native shell (tools/installer_d2d.h kW, kH).
const WIDTH: f64 = 820.0;
const HEIGHT: f64 = 560.0;

struct Host {
    last: Mutex<Value>,
    out: Mutex<std::io::Stdout>,
    shown: AtomicBool,
}

impl Host {
    fn send(&self, message: Value) {
        if let Ok(mut out) = self.out.lock() {
            let _ = writeln!(out, "{message}");
            let _ = out.flush();
        }
    }
}

#[tauri::command]
fn installer_view(host: tauri::State<'_, Host>) -> Value {
    host.last.lock().map(|v| v.clone()).unwrap_or(Value::Null)
}

#[tauri::command]
fn installer_action(window: WebviewWindow, host: tauri::State<'_, Host>, action: Value) {
    match action.get("type").and_then(Value::as_str).unwrap_or("") {
        "minimise" => {
            let _ = window.minimize();
        }
        "ready" => reveal(&window, &host),
        _ => {}
    }
    host.send(json!({ "type": "action", "action": action }));
}

/// The window is shown cloaked from the start (WebView2 renders only for a shown window) and uncloaked when the page
/// says it has painted ('ready'), or 3 s after the page loaded, whichever is first.
fn reveal(window: &WebviewWindow, host: &Host) {
    if host.shown.swap(true, Ordering::SeqCst) {
        return;
    }
    #[cfg(windows)]
    set_cloak(window, false);
    let _ = window.set_focus();
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

/// Win11 rounded corners from DWM (DWMWA_WINDOW_CORNER_PREFERENCE 33 = DWMWCP_ROUND 2), exactly as the Panel's
/// main window (src-tauri/src/windows.rs apply_dwm_rounded_corners). No effect before Windows 11.
#[cfg(windows)]
fn apply_dwm_corners(window: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    if let Some(hwnd) = hwnd_of(window) {
        let preference: u32 = 2;
        let _ = unsafe { DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(33), (&preference as *const u32).cast(), std::mem::size_of::<u32>() as u32) };
    }
}

fn main() {
    let data_dir = arg_value("--data-dir").map(PathBuf::from);
    tauri::Builder::default()
        .manage(Host { last: Mutex::new(Value::Null), out: Mutex::new(std::io::stdout()), shown: AtomicBool::new(false) })
        .invoke_handler(tauri::generate_handler![installer_view, installer_action])
        .setup(move |app| {
            let mut builder = WebviewWindowBuilder::new(app, WINDOW_ID, WebviewUrl::App("index.html".into()))
                .title("Foundation Sunshine")
                .inner_size(WIDTH, HEIGHT)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                // As the Panel: opaque WebView2, DWM clips the corners. shadow(true) keeps the DWM drop shadow of an
                // undecorated window (Tauri: "on Windows 11 it will have rounded corners").
                .transparent(false)
                .shadow(true)
                .visible(false)
                .center()
                .disable_drag_drop_handler()
                .on_page_load(|window, payload| {
                    if payload.event() == tauri::webview::PageLoadEvent::Finished {
                        let host = window.state::<Host>();
                        host.send(json!({ "type": "loaded" }));
                        let w = window.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_secs(3));
                            let host = w.state::<Host>();
                            reveal(&w, &host);
                        });
                    }
                });
            if let Some(dir) = data_dir.clone() {
                builder = builder.data_directory(dir);
            }
            let window = builder.build()?;
            #[cfg(windows)]
            {
                apply_dwm_corners(&window);
                set_cloak(&window, true);
            }
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
                        "focus" => {
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        _ => {}
                    }
                }
                handle.exit(0);
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
