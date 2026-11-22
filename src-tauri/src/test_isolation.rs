//! Private profile and side-effect boundaries for interactive UI measurements.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PROFILE: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn configure() -> std::io::Result<()> {
    if !cfg!(windows)
        || std::env::var_os("SUNSHINE_GUI_TEST_ISOLATION").as_deref()
            != Some(std::ffi::OsStr::new("1"))
    {
        return Ok(());
    }
    let profile = std::env::var("SUNSHINE_GUI_TEST_PROFILE")
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    if !valid_profile(&profile) || std::env::args_os().skip(1).any(|arg| arg != "--hidden") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "test isolation needs a named profile and accepts only --hidden",
        ));
    }
    let exe = std::env::current_exe()?.canonicalize()?;
    let exe_dir = exe.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    let root = child(exe_dir, "test-profiles")?;
    let profile = child(&root, &profile)?;
    let config = child(&profile, "config")?;
    let data = child(&profile, "data")?;
    let webview = child(&profile, "webview")?;
    let temp = child(&profile, "temp")?;

    // WebView2 honors this override at environment creation (panel-isolation-ms-webview2,
    // CreateCoreWebView2EnvironmentWithOptions). Set the per-process paths before workers start.
    unsafe {
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", webview);
        std::env::set_var("APPDATA", config);
        std::env::set_var("LOCALAPPDATA", data);
        std::env::set_var("TMP", &temp);
        std::env::set_var("TEMP", temp);
    }
    PROFILE.set(profile).expect("test profile configured once");
    Ok(())
}

fn valid_profile(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn child(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
    let path = dir.join(name);
    match std::fs::create_dir(&path) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error),
    }
    let path = path.canonicalize()?;
    if !path.is_dir() || path.parent() != Some(dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "test directory resolves outside its parent",
        ));
    }
    Ok(path)
}

pub(crate) fn active() -> bool {
    PROFILE.get().is_some()
}

pub(crate) fn path(name: &str) -> Option<PathBuf> {
    PROFILE.get().map(|dir| dir.join(name))
}

// Tauri and dirs resolve the Windows known folders, not the APPDATA environment
// overrides (panel-isolation-ms-knownfolder, hToken; panel-isolation-tauri-path, app_data_dir). Use the private paths at these sites.
pub(crate) fn config_dir() -> Option<PathBuf> {
    path("config").or_else(dirs::config_dir)
}

pub(crate) fn data_local_dir() -> Option<PathBuf> {
    path("data").or_else(dirs::data_local_dir)
}

pub(crate) fn app_data_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<PathBuf> {
    use tauri::Manager;
    path("app").map_or_else(|| app.path().app_data_dir(), Ok)
}

/// The invoke dispatcher checks the command before passing the request to its handler (`panel-isolation-tauri-invoke`, `Invoke::message`).
pub(crate) fn allows_command(command: &str) -> bool {
    matches!(
        command,
        "clipboard_sync_status"
            | "ctx_popup_cancel"
            | "ctx_popup_loaded"
            | "ctx_popup_open"
            | "ctx_popup_ready"
            | "ctx_popup_selftest"
            | "frontend_slow"
            | "get_current_dpi"
            | "get_dpi_scale_info"
            | "get_gpus"
            | "get_proxy_health_check"
            | "get_proxy_url_command"
            | "get_rtss_status"
            | "get_sunshine_build_info"
            | "get_sunshine_service_status"
            | "get_sunshine_version"
            | "get_system_info"
            | "get_tray_locale"
            | "get_vdd_status"
            | "get_vigem_status"
            | "get_vmouse_support"
            | "get_vmouse_status"
            | "launch_splash_frame"
            | "launch_splash_painted"
            | "launch_splash_prepared"
            | "launch_splash_selftest"
            | "main_panel_loading"
            | "main_panel_ready"
            | "panel_activity"
            | "panel_content_ready"
            | "panel_dpi_settled"
            | "panel_first_paint"
            | "panel_set_surface"
            | "panel_state_save"
            | "parse_sunshine_config"
            | "probe_sunshine_host"
            | "system_load"
            | "take_pending_panel_page"
            | "take_pending_start_guide"
            | "toggle_dark_mode"
            | "tray_popup_layout"
            | "tray_popup_theme_set"
            | "tray_prefs_get"
            | "wait_for_proxy_ready"
            | "webview_heartbeat"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_name_is_one_safe_component() {
        assert!(valid_profile("adaptive-3_1"));
        for name in ["", "..", "a/b", r"a\b", "C:root", "one two"] {
            assert!(!valid_profile(name), "{name}");
        }
    }

    #[test]
    fn permits_window_measurements_and_denies_mutations() {
        assert!(allows_command("panel_state_save"));
        assert!(allows_command("get_proxy_health_check"));
        assert!(allows_command("ctx_popup_ready"));
        assert!(allows_command("ctx_popup_selftest"));
        assert!(allows_command("launch_splash_selftest"));
        for name in [
            "update_desktop_settings",
            "set_locale_preferences",
            "tray_popup_act",
            "restart_as_admin",
        ] {
            assert!(!allows_command(name), "{name}");
        }
    }
}
