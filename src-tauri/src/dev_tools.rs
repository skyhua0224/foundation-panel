//! Tools of the 开发者选项 page that only the panel's own process can give: the overrides of the window behaviour that are otherwise environment variables,
//! a scale change on the real window, DevTools, the cache of the WebView and the reset of the settings of this window.
//! Every command is local-only (build.rs LOCAL_ONLY_COMMANDS, granted to the main window in capabilities/main-developer.json): the Web UI of the host never reaches them.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const FILE: &str = "developer.json";
/// Same names as SUNSHINE_GUI_WEBVIEW_DPI_COVER takes (dpi_cover::mode_of).
pub(crate) const COVER_MODES: [&str; 6] = ["off", "fast", "fixed", "adaptive", "snapshot", "warm"];

/// What the person chose on the page. None = "as the environment and the defaults say".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Overrides {
    pub cover: Option<String>,
    pub cold_cloak: Option<bool>,
}

/// The file may be damaged or written by another version: whatever is not understood is dropped, never an error.
pub(crate) fn parse_overrides(text: &str) -> Overrides {
    let mut value: Overrides = serde_json::from_str(text).unwrap_or_default();
    value.cover = value.cover.filter(|mode| COVER_MODES.contains(&mode.as_str()));
    value
}

/// A mode the page sends: one of the known names, anything else is refused.
pub(crate) fn valid_cover(mode: &str) -> Option<&'static str> {
    COVER_MODES.iter().copied().find(|known| known.eq_ignore_ascii_case(mode.trim()))
}

/// The mode that counts: the override of the page, then the variable, then the default (the caller's).
pub(crate) fn effective<'a>(override_value: Option<&'a str>, env_value: Option<&'a str>) -> Option<&'a str> {
    override_value.or(env_value)
}

/// The scale the window is pushed to: a percentage of the real scale (100 = back to the real one, which is `real_dpi`). Refused outside 50 – 400.
pub(crate) fn simulated_dpi(real_dpi: u32, percent: u32) -> Option<u32> {
    (50..=400).contains(&percent).then(|| (u64::from(real_dpi) * u64::from(percent) / 100).max(1) as u32)
}

static OVERRIDES: Mutex<Option<Overrides>> = Mutex::new(None);

fn path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|dir| dir.join("Sunshine GUI").join(FILE))
}

fn current() -> Overrides {
    let mut guard = OVERRIDES.lock().unwrap();
    guard.get_or_insert_with(|| path().and_then(|p| std::fs::read_to_string(p).ok()).map(|text| parse_overrides(&text)).unwrap_or_default()).clone()
}

fn store(next: Overrides) -> Result<(), String> {
    let file = path().ok_or("no config directory")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    *OVERRIDES.lock().unwrap() = Some(next);
    Ok(())
}

/// Read where the cover mode is decided (windows/lifecycle/dpi_cover.rs).
pub(crate) fn cover_override() -> Option<String> {
    current().cover
}

/// Read where the cold-start cloak is decided (windows.rs).
pub(crate) fn cold_cloak_override() -> Option<bool> {
    current().cold_cloak
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevState {
    pub cover: String,
    pub cover_from: &'static str,
    pub cold_cloak: bool,
    pub cold_cloak_from: &'static str,
    pub dpi: u32,
}

#[tauri::command]
pub fn dev_get_state(window: tauri::WebviewWindow) -> DevState {
    let own = current();
    let env_cover = std::env::var("SUNSHINE_GUI_WEBVIEW_DPI_COVER").ok();
    let env_cloak = std::env::var("SUNSHINE_GUI_COLD_CLOAK").ok();
    DevState {
        cover: effective(own.cover.as_deref(), env_cover.as_deref()).and_then(valid_cover).unwrap_or("fast").to_string(),
        cover_from: if own.cover.is_some() { "page" } else if env_cover.is_some() { "environment" } else { "default" },
        cold_cloak: own.cold_cloak.unwrap_or_else(|| env_cloak.as_deref() == Some("on")),
        cold_cloak_from: if own.cold_cloak.is_some() { "page" } else if env_cloak.is_some() { "environment" } else { "default" },
        dpi: real_dpi(&window),
    }
}

/// `mode` = "" gives the choice back to the environment and the default.
#[tauri::command]
pub fn dev_set_cover_mode(mode: String) -> Result<(), String> {
    let mut next = current();
    next.cover = if mode.trim().is_empty() { None } else { Some(valid_cover(&mode).ok_or_else(|| format!("unknown cover mode: {mode}"))?.to_string()) };
    store(next)
}

/// Takes effect at the next cold start of the window (the cloak is made when the window is first shown).
#[tauri::command]
pub fn dev_set_cold_cloak(on: Option<bool>) -> Result<(), String> {
    let mut next = current();
    next.cold_cloak = on;
    store(next)
}

#[cfg(target_os = "windows")]
fn real_dpi(window: &tauri::WebviewWindow) -> u32 {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    window.hwnd().map(|hwnd| unsafe { GetDpiForWindow(HWND(hwnd.0)) }).unwrap_or(96)
}

#[cfg(not(target_os = "windows"))]
fn real_dpi(_window: &tauri::WebviewWindow) -> u32 {
    96
}

/// Pushes the real window through the path a scale change takes (the cover, the layout, the reveal), without the system's scale changing. `percent` 100 goes back.
#[tauri::command]
pub fn dev_simulate_dpi(window: tauri::WebviewWindow, percent: u32) -> Result<(), String> {
    let dpi = simulated_dpi(real_dpi(&window), percent).ok_or("the scale must be between 50 and 400 percent")?;
    log::info!("[developer] simulated scale change to {dpi} dpi ({percent}%)");
    crate::windows::send_synthetic_dpi(&window, dpi)
}

/// DevTools of the main window. The embedded Web UI is a frame of the same page: pick its context in the console or the frame in Elements.
#[tauri::command]
pub fn dev_open_devtools(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let window = app.get_webview_window("main").ok_or("the main window is not there")?;
    window.open_devtools();
    Ok(())
}

/// The HTTP cache of the WebView (not its storage: the settings of the window stay). Asks WebView2 through the DevTools protocol.
#[tauri::command]
pub async fn dev_clear_webview_cache(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        let tx = std::sync::Mutex::new(Some(tx));
        window
            .with_webview(move |webview| unsafe {
                use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
                use windows::core::w;
                let done = move |result: Result<(), String>| {
                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(result);
                    }
                };
                let report = done;
                let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, _| {
                    report(result.map_err(|e| e.to_string()));
                    Ok(())
                }));
                let started = webview.controller().CoreWebView2().and_then(|core| core.CallDevToolsProtocolMethod(w!("Network.clearBrowserCache"), w!("{}"), &handler));
                if let Err(error) = started {
                    log::warn!("[developer] clearing the WebView cache failed to start: {error}");
                }
            })
            .map_err(|e| e.to_string())?;
        tokio::time::timeout(std::time::Duration::from_secs(5), rx).await.map_err(|_| "no answer from WebView2".to_string())?.map_err(|_| "the request was dropped".to_string())?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        Err("windows only".to_string())
    }
}

/// The `data` of a `Page.captureScreenshot` answer (the PNG in base64); an answer without it is an error.
fn capture_data(json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    value.get("data").and_then(|d| d.as_str()).filter(|d| !d.is_empty()).map(str::to_string).ok_or_else(|| "the capture has no image".to_string())
}

/// A PNG of the whole window as WebView2 draws it (the embedded Web UI frame included), in base64: the report sheet attaches it, taken before the sheet is drawn. Asks WebView2 through the DevTools protocol.
#[tauri::command]
pub async fn dev_capture_window(window: tauri::WebviewWindow) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
        let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
        let on_start = tx.clone();
        window
            .with_webview(move |webview| unsafe {
                use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
                use windows::core::w;
                let answer = tx.clone();
                let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, json| {
                    let out = result.map_err(|e| e.to_string()).and_then(|_| capture_data(&json));
                    if let Some(tx) = answer.lock().unwrap().take() {
                        let _ = tx.send(out);
                    }
                    Ok(())
                }));
                let started = webview.controller().CoreWebView2().and_then(|core| core.CallDevToolsProtocolMethod(w!("Page.captureScreenshot"), w!("{\"format\":\"png\"}"), &handler));
                if let Err(error) = started {
                    log::warn!("[developer] the window capture failed to start: {error}");
                    if let Some(tx) = on_start.lock().unwrap().take() {
                        let _ = tx.send(Err(error.to_string()));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        tokio::time::timeout(std::time::Duration::from_secs(5), rx).await.map_err(|_| "no answer from WebView2".to_string())?.map_err(|_| "the request was dropped".to_string())?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        Err("windows only".to_string())
    }
}

/// The settings file of the window goes back to its defaults (the page then clears what the window itself stores and reloads).
#[tauri::command]
pub async fn dev_reset_desktop_settings(app: tauri::AppHandle) -> Result<(), String> {
    crate::desktop_settings::save_desktop_settings(app, crate::desktop_settings::DesktopSettings::default()).await.map(|_| ())?;
    store(Overrides::default())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevEnvironment {
    pub app_version: &'static str,
    pub webview: Option<String>,
    pub target: &'static str,
}

/// What `get_system_info`, `get_gpus` and the host do not know: the WebView2 runtime this window runs on.
#[tauri::command]
pub fn dev_environment() -> DevEnvironment {
    DevEnvironment { app_version: env!("CARGO_PKG_VERSION"), webview: tauri::webview_version().ok(), target: std::env::consts::ARCH }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capture_answer_gives_its_image_or_an_error() {
        assert_eq!(capture_data(r#"{"data":"iVBORw0KGgo="}"#), Ok("iVBORw0KGgo=".to_string()));
        assert!(capture_data("{}").is_err());
        assert!(capture_data(r#"{"data":""}"#).is_err());
        assert!(capture_data("not json").is_err());
    }

    #[test]
    fn a_damaged_file_gives_the_defaults() {
        assert_eq!(parse_overrides("not json"), Overrides::default());
        assert_eq!(parse_overrides(""), Overrides::default());
    }

    #[test]
    fn an_unknown_mode_is_dropped_and_a_known_one_kept() {
        assert_eq!(parse_overrides(r#"{"cover":"sparkle"}"#).cover, None);
        assert_eq!(parse_overrides(r#"{"cover":"adaptive","coldCloak":true}"#), Overrides { cover: Some("adaptive".into()), cold_cloak: Some(true) });
    }

    #[test]
    fn the_page_wins_over_the_environment() {
        assert_eq!(effective(Some("off"), Some("fast")), Some("off"));
        assert_eq!(effective(None, Some("fast")), Some("fast"));
        assert_eq!(effective(None, None), None);
    }

    #[test]
    fn the_mode_names_are_the_ones_dpi_cover_knows() {
        for mode in COVER_MODES {
            assert_eq!(valid_cover(mode), Some(mode));
        }
        assert_eq!(valid_cover(" Warm "), Some("warm"));
        assert_eq!(valid_cover("on"), None);
    }

    #[test]
    fn a_simulated_scale_is_a_share_of_the_real_one() {
        assert_eq!(simulated_dpi(96, 200), Some(192));
        assert_eq!(simulated_dpi(192, 100), Some(192));
        assert_eq!(simulated_dpi(144, 50), Some(72));
        assert_eq!(simulated_dpi(96, 10), None);
        assert_eq!(simulated_dpi(96, 900), None);
    }
}
