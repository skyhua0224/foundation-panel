//! The WebView2 environment of the panel: the options it is made with, one environment for all the panel's windows, and what is logged when a window cannot be made.
//!
//! Why this exists (HRESULT 0x8007139F = `HRESULT_FROM_WIN32(ERROR_INVALID_STATE)`, seen when the menu window and the launch splash could not be created):
//!
//! * Microsoft Learn, "Process model" (WebView2): environments that share a user data folder share ONE browser process, and "if you try to create a CoreWebView2Environment using a user data folder which
//!   another CoreWebView2Environment is already using, and you don't configure the two the same ... the second CoreWebView2Environment will fail to create WebView2 objects. This is true for objects
//!   that are in the same process or in different processes."
//! * Microsoft Learn, `CreateCoreWebView2EnvironmentWithOptions` (Win32 reference): `HRESULT_FROM_WIN32(ERROR_INVALID_STATE)` = "Specified options do not match the options of the WebViews that are
//!   currently running in the shared browser process."
//!
//! So the code is returned when the options (`AdditionalBrowserArguments`, `Language`, the extension flag, the scroll bar style ...) of a new environment differ from those of the browser process that is
//! already running for the user data folder. Inside one panel process every window is made with identical options (tauri-runtime-wry 2.12 and wry 0.57 build them from the same attributes), so the
//! difference comes from another process on the same user data folder, or from the same panel started again while the browser process of the one before is still there. The panel itself produced such a
//! difference: `main()` appended its switches to `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on every start, and a panel started by a panel (restart after a crash, the elevated restart, the updater's
//! relaunch) inherits that variable, so its arguments were "ours ours" while the browser process of its parent still ran with "ours". `merge_browser_arguments` makes that composition idempotent.
//!
//! Second line of defence: all of the panel's windows are made from one environment object (`adopt` / `capture`): a window made later reuses the environment of the first one, so it cannot differ from it,
//! and it costs no environment of its own. A window that still fails with this code is retried with a pause (`RETRY_DELAYS`), and every failure logs the stage, the options in force and the WebView2
//! processes that exist (`describe_failure`), which is what tells the next incident apart.

use log::{info, warn};
use std::time::Duration;

use crate::webview_env_core::{is_invalid_state, retry_delay};

/// What is logged for a window that could not be made: the stage, the attempt, the error and, for the invalid-state code, the options in force and the WebView2 processes (a browser process whose parent
/// is not this process belongs to another panel or program on the same user data folder).
pub fn describe_failure(stage: &str, attempt: usize, shared: bool, error: &str) -> String {
    let mut line = format!("[webview-env] {stage}: attempt {attempt} failed (shared environment: {shared}): {error}");
    if is_invalid_state(error) {
        let arguments = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        line.push_str(&format!(" | invalid state: the options differ from the running browser process | arguments in force: [{arguments}] | {}", processes_report()));
    }
    line
}

/// Logs a failure and answers how long to wait before the next attempt (None = give up).
pub fn note_failure(stage: &str, attempt: usize, shared: bool, error: &str) -> Option<Duration> {
    let delay = retry_delay(attempt, error);
    if delay.is_some() {
        warn!("{}", describe_failure(stage, attempt + 1, shared, error));
    } else {
        warn!("{} | no further attempt", describe_failure(stage, attempt + 1, shared, error));
    }
    delay
}

pub fn note_success(stage: &str, attempt: usize, shared: bool, started: std::time::Instant) {
    if attempt > 0 {
        info!("[webview-env] {stage}: made at attempt {} after {} ms (shared environment: {shared})", attempt + 1, started.elapsed().as_millis());
    }
}

#[cfg(target_os = "windows")]
fn processes_report() -> String {
    let own = std::process::id();
    let all = webview2_processes();
    let ours = all.iter().filter(|(_, parent)| *parent == own).count();
    let others: Vec<String> = all.iter().filter(|(_, parent)| *parent != own).map(|(pid, parent)| format!("{pid}<-{parent}")).collect();
    format!("msedgewebview2.exe: {} of this process (pid {own}), others (pid<-parent): [{}]", ours, others.join(" "))
}

#[cfg(not(target_os = "windows"))]
fn processes_report() -> String {
    String::new()
}

/// (pid, parent pid) of every msedgewebview2.exe process.
#[cfg(target_os = "windows")]
fn webview2_processes() -> Vec<(u32, u32)> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    let mut found = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return found };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry.szExeFile.iter().position(|unit| *unit == 0).unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("msedgewebview2.exe") {
                    found.push((entry.th32ProcessID, entry.th32ParentProcessID));
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    found
}

// ---- one environment for all the windows ------------------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod shared {
    use std::sync::Mutex;
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;

    /// The environment object is only handed to window builders (which pass it to the main thread that makes the window), never called from here.
    struct Held(ICoreWebView2Environment);
    unsafe impl Send for Held {}
    unsafe impl Sync for Held {}

    static ENV: Mutex<Option<Held>> = Mutex::new(None);

    fn lock() -> std::sync::MutexGuard<'static, Option<Held>> {
        ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get() -> Option<ICoreWebView2Environment> {
        lock().as_ref().map(|held| held.0.clone())
    }

    pub fn put(environment: ICoreWebView2Environment) {
        let mut slot = lock();
        if slot.is_none() {
            *slot = Some(Held(environment));
        }
    }

    pub fn clear() -> bool {
        lock().take().is_some()
    }
}

/// The builder makes its window from the environment of the first window of the panel when there is one (its options cannot differ from that window's, and no second environment is made).
#[cfg(target_os = "windows")]
pub fn adopt<'a, R: tauri::Runtime, M: tauri::Manager<R>>(builder: tauri::WebviewWindowBuilder<'a, R, M>) -> tauri::WebviewWindowBuilder<'a, R, M> {
    match shared::get() {
        Some(environment) => builder.with_environment(environment),
        None => builder,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn adopt<'a, R: tauri::Runtime, M: tauri::Manager<R>>(builder: tauri::WebviewWindowBuilder<'a, R, M>) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Whether windows are being made from the shared environment now.
pub fn is_shared() -> bool {
    #[cfg(target_os = "windows")]
    {
        shared::get().is_some()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// A window of the panel was made: its environment is kept for the windows made later (the first one wins).
pub fn capture<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    #[cfg(target_os = "windows")]
    if !is_shared() {
        let label = window.label().to_string();
        let result = window.with_webview(move |webview| {
            shared::put(webview.environment());
            info!("[webview-env] the environment of {label} is the one all windows of the panel are made from");
        });
        if let Err(error) = result {
            warn!("[webview-env] the environment of the window could not be kept: {error}");
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
}

/// A window could not be made from the shared environment: the next attempt makes its own (the object may belong to a browser process that is gone).
pub fn forget() {
    #[cfg(target_os = "windows")]
    if shared::clear() {
        warn!("[webview-env] the shared environment is dropped: the next window makes its own");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_failure_line_names_the_stage_and_the_attempt() {
        let line = describe_failure("tray-menu-window", 2, true, "boom");
        assert!(line.contains("tray-menu-window") && line.contains("attempt 2") && line.contains("shared environment: true") && line.contains("boom"));
        assert!(!line.contains("invalid state"), "the process report is for the invalid-state code only");
    }
}
