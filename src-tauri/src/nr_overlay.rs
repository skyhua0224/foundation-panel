mod gamepad;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    nr_shortcut: String,
    overlay_shortcut: String,
    nr_gamepad: String,
    overlay_gamepad: String,
    opacity: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            nr_shortcut: "Ctrl+Alt+KeyN".into(),
            overlay_shortcut: String::new(),
            nr_gamepad: String::new(),
            overlay_gamepad: String::new(),
            opacity: 62,
        }
    }
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    nr_shortcut: Option<String>,
    overlay_shortcut: Option<String>,
    nr_gamepad: Option<String>,
    overlay_gamepad: Option<String>,
    opacity: Option<u32>,
}
#[derive(Default)]
struct OverlayState {
    settings: Settings,
    owned: HashMap<u32, Shortcut>,
    target: Option<u64>,
    capture: Option<(String, std::time::Instant)>,
    // DLSS NR installed and usable (the status the panel reads): only then are the NR and overlay shortcuts held. The stored
    // bindings stay in `settings` either way, so they come back after a reinstall.
    nr_ready: bool,
}
static STATE: LazyLock<Mutex<OverlayState>> = LazyLock::new(|| Mutex::new(OverlayState::default()));
static VISIBLE: AtomicBool = AtomicBool::new(false);
static TOGGLING: AtomicBool = AtomicBool::new(false);
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsStatus {
    settings: Settings,
    nr_registered: bool,
    overlay_registered: bool,
    gamepad_supported: bool,
    capture_active: bool,
    target: Option<u64>,
}
fn snapshot(state: &OverlayState) -> SettingsStatus {
    let registered = |s: &str| {
        s.parse::<Shortcut>()
            .ok()
            .is_some_and(|key| state.owned.contains_key(&key.id()))
    };
    SettingsStatus {
        settings: state.settings.clone(),
        nr_registered: registered(&state.settings.nr_shortcut),
        overlay_registered: registered(&state.settings.overlay_shortcut),
        gamepad_supported: cfg!(target_os = "windows"),
        capture_active: state.capture.is_some(),
        target: state.target,
    }
}
fn path() -> Result<PathBuf, String> {
    crate::test_isolation::config_dir()
        .map(|p| p.join("Sunshine GUI").join("enhancement-controls.json"))
        .ok_or("nr_settings_path".into())
}
fn save(settings: &Settings) -> Result<(), String> {
    let path = path()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Storage::FileSystem::{
                    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
                },
            };
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                MoveFileExW(
                    PCWSTR(from.as_ptr()),
                    PCWSTR(to.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
            .map_err(|e| e.to_string())?;
        }
        #[cfg(not(target_os = "windows"))]
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn normalized_key(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Ok(String::new());
    }
    if value.len() > 100 {
        return Err("nr_shortcut_invalid".into());
    }
    let key: Shortcut = value.parse().map_err(|_| "nr_shortcut_invalid")?;
    if !key
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT)
        || key.mods.contains(Modifiers::SUPER)
    {
        return Err("nr_shortcut_invalid".into());
    }
    if key.id()
        == crate::app::TOOLBAR_SHORTCUT
            .parse::<Shortcut>()
            .unwrap()
            .id()
    {
        return Err("nr_shortcut_conflict".into());
    }
    Ok(key.to_string())
}
fn bindings(settings: &Settings) -> Result<Vec<Shortcut>, String> {
    gamepad::validate(&settings.nr_gamepad, &settings.overlay_gamepad)?;
    let keys: Vec<Shortcut> = [&settings.nr_shortcut, &settings.overlay_shortcut]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().map_err(|_| "nr_shortcut_invalid".to_string()))
        .collect::<Result<_, _>>()?;
    if keys.len() == 2 && keys[0].id() == keys[1].id() {
        return Err("nr_shortcut_duplicate".into());
    }
    Ok(keys)
}
trait Registrar {
    fn register(&self, key: Shortcut) -> Result<(), String>;
    fn unregister(&self, key: Shortcut) -> Result<(), String>;
}
/// Whether the NR toggle and overlay shortcuts may be held, with the reason: only for a DLSS NR component that is installed and
/// usable (installed, the host bridge, an NVIDIA adapter), the status the panel reads (useDlssNrReady). Unknown holds them back:
/// a global hotkey that does nothing for the user still takes the key from every other app.
pub(crate) fn shortcuts_wanted(
    status: Option<&crate::native_components::providers::EnhancementComponentStatus>,
) -> (bool, &'static str) {
    match status {
        None => (false, "DLSS NR status unknown"),
        Some(s) if !s.installed => (false, "DLSS NR not installed"),
        Some(s) if !s.host_supported => (false, "the host has no DLSS NR bridge"),
        Some(s) if !s.adapter_present => (false, "no NVIDIA adapter for DLSS NR"),
        Some(_) => (true, "DLSS NR installed and usable"),
    }
}
/// The bindings to hold now: none while DLSS NR is not ready or a shortcut is being recorded.
fn desired_bindings(state: &OverlayState, settings: &Settings) -> Result<Vec<Shortcut>, String> {
    let keys = bindings(settings)?;
    Ok(if state.nr_ready && state.capture.is_none() { keys } else { Vec::new() })
}
/// One log line for every register and unregister, with the reason.
struct Logged<'a, T: Registrar>(&'a T, &'a str);
impl<T: Registrar> Registrar for Logged<'_, T> {
    fn register(&self, key: Shortcut) -> Result<(), String> {
        let result = self.0.register(key);
        match &result {
            Ok(()) => log::info!("NR shortcut {key} registered: {}", self.1),
            Err(error) => log::warn!("NR shortcut {key} not registered ({error}): {}", self.1),
        }
        result
    }
    fn unregister(&self, key: Shortcut) -> Result<(), String> {
        let result = self.0.unregister(key);
        match &result {
            Ok(()) => log::info!("NR shortcut {key} unregistered: {}", self.1),
            Err(error) => log::warn!("NR shortcut {key} not unregistered ({error}): {}", self.1),
        }
        result
    }
}
/// Hold or release the shortcuts for a new DLSS NR readiness, key by key (a key another app holds does not keep the other one back).
fn set_ready(state: &mut OverlayState, ready: bool, registrar: &impl Registrar, reason: &str) -> bool {
    if state.nr_ready == ready {
        return false;
    }
    state.nr_ready = ready;
    let desired = desired_bindings(state, &state.settings.clone()).unwrap_or_default();
    let logged = Logged(registrar, reason);
    for key in &desired {
        if !state.owned.contains_key(&key.id()) && logged.register(*key).is_ok() {
            state.owned.insert(key.id(), *key);
        }
    }
    let removed: Vec<_> = state.owned.values().filter(|k| !desired.iter().any(|d| d.id() == k.id())).copied().collect();
    for key in removed {
        if logged.unregister(key).is_ok() {
            state.owned.remove(&key.id());
        }
    }
    true
}
/// A DLSS NR component status from startup or from a component command (status, install, uninstall, recover, select): the
/// shortcuts follow it. A failed status read is not handed in, so it does not take the keys away.
pub(crate) fn note_component_status<R: Runtime>(
    app: &AppHandle<R>,
    status: Option<&crate::native_components::providers::EnhancementComponentStatus>,
    cause: &str,
) {
    let (ready, why) = shortcuts_wanted(status);
    let reason = format!("{cause}: {why}");
    let changed = {
        let mut state = STATE.lock().unwrap();
        let first = cause == "startup";
        let changed = set_ready(&mut state, ready, &GlobalRegistrar(app), &reason);
        if first && !ready {
            log::info!("NR shortcuts not registered: {reason}");
        }
        changed.then(|| snapshot(&state))
    };
    if let Some(status) = changed {
        gamepad::wake();
        let _ = app.emit("nr-settings-changed", status);
    }
}
struct GlobalRegistrar<'a, R: Runtime>(&'a AppHandle<R>);
impl<R: Runtime> Registrar for GlobalRegistrar<'_, R> {
    fn register(&self, key: Shortcut) -> Result<(), String> {
        if self.0.global_shortcut().is_registered(key) {
            return Err("nr_shortcut_conflict".into());
        }
        self.0
            .global_shortcut()
            .on_shortcut(key, |app, shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }
                let app = app.clone();
                let id = shortcut.id();
                // Never wait for the settings mutex on the UI thread: registration can
                // itself dispatch to that thread while a settings command holds it.
                tauri::async_runtime::spawn(async move {
                    let action = {
                        let state = STATE.lock().unwrap();
                        if state.capture.is_some() {
                            return;
                        }
                        if state
                            .settings
                            .nr_shortcut
                            .parse::<Shortcut>()
                            .ok()
                            .is_some_and(|k| k.id() == id)
                        {
                            1
                        } else if state
                            .settings
                            .overlay_shortcut
                            .parse::<Shortcut>()
                            .ok()
                            .is_some_and(|k| k.id() == id)
                        {
                            2
                        } else {
                            0
                        }
                    };
                    run_action(&app, action).await;
                });
            })
            .map_err(|_| "nr_shortcut_conflict".into())
    }
    fn unregister(&self, key: Shortcut) -> Result<(), String> {
        self.0
            .global_shortcut()
            .unregister(key)
            .map_err(|_| "nr_shortcut_unregister".into())
    }
}
async fn run_action<R: Runtime>(app: &AppHandle<R>, action: u8) {
    if !STATE.lock().unwrap().nr_ready {
        return;
    }
    let result = match action {
        1 => toggle_nr(app).await,
        2 => {
            let visible = app
                .get_webview_window("nr_overlay")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            set_visible(app, !visible)
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        log::warn!("NR shortcut: {error}");
        let _ = app.emit("nr-action-error", error);
    }
}
// Track each successful OS operation so even a failed rollback is reported honestly.
fn replace_bindings(
    registrar: &impl Registrar,
    owned: &mut HashMap<u32, Shortcut>,
    desired: &[Shortcut],
) -> Result<(), String> {
    for key in desired {
        if !owned.contains_key(&key.id()) {
            registrar.register(*key)?;
            owned.insert(key.id(), *key);
        }
    }
    let removed: Vec<_> = owned
        .values()
        .filter(|key| !desired.iter().any(|new| new.id() == key.id()))
        .copied()
        .collect();
    for key in removed {
        registrar.unregister(key)?;
        owned.remove(&key.id());
    }
    Ok(())
}
fn apply_settings(
    state: &mut OverlayState,
    next: Settings,
    registrar: &impl Registrar,
    persist: impl FnOnce(&Settings) -> Result<(), String>,
) -> Result<(), String> {
    let desired = desired_bindings(state, &next)?;
    let previous: Vec<_> = state.owned.values().copied().collect();
    let registrar = &Logged(registrar, "the shortcut setting changed");
    if let Err(error) =
        replace_bindings(registrar, &mut state.owned, &desired).and_then(|_| persist(&next))
    {
        if replace_bindings(registrar, &mut state.owned, &previous).is_err() {
            return Err("nr_shortcut_restore_failed".into());
        }
        return Err(error);
    }
    state.settings = next;
    gamepad::wake();
    Ok(())
}
pub fn initialize<R: Runtime>(app: &AppHandle<R>) {
    let mut state = STATE.lock().unwrap();
    if let Ok(settings) = path()
        .and_then(|p| fs::read(p).map_err(|e| e.to_string()))
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).map_err(|e| e.to_string()))
    {
        if let (Ok(nr), Ok(overlay)) = (
            normalized_key(&settings.nr_shortcut),
            normalized_key(&settings.overlay_shortcut),
        ) {
            let settings = Settings {
                nr_shortcut: nr,
                overlay_shortcut: overlay,
                nr_gamepad: gamepad::normalize(&settings.nr_gamepad).unwrap_or_default(),
                overlay_gamepad: gamepad::normalize(&settings.overlay_gamepad).unwrap_or_default(),
                opacity: settings.opacity.clamp(35, 95),
            };
            if bindings(&settings).is_ok() {
                state.settings = settings;
            }
        }
    }
    drop(state);
    gamepad::start(app);
    // the keys wait for the DLSS NR status: registered only for an installed and usable component
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The status comes from the host. The panel can start before it (the computer was switched on, the host was reinstalled), so the registration waits for the host and asks again
        // until it answers (about two minutes at most); a host that never answers leaves the shortcuts off, as before.
        let mut status = Err(String::new());
        for attempt in 0..8u32 {
            let _ = crate::proxy_server::host_ready(std::time::Duration::from_secs(20)).await;
            status = crate::native_components::providers::nvidia_dlssnr::component_get_status().await;
            match &status {
                Ok(_) => break,
                Err(error) => {
                    log::info!("DLSS NR status at startup not available yet (try {}): {error}", attempt + 1);
                    crate::proxy_server::host_not_ready();
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
        if let Err(error) = &status {
            log::warn!("DLSS NR status at startup unavailable: {error}");
        }
        note_component_status(&app, status.as_ref().ok(), "startup");
    });
}
// Restore from current settings, including edits made by another manager window.
fn finish_capture(state: &mut OverlayState, registrar: &impl Registrar) -> Result<(), String> {
    state.capture = None;
    gamepad::wake();
    let desired = desired_bindings(state, &state.settings.clone())?;
    replace_bindings(&Logged(registrar, "shortcut recording finished"), &mut state.owned, &desired)
}
#[tauri::command]
pub async fn nr_overlay_capture_shortcut(
    window: tauri::WebviewWindow,
    active: bool,
) -> Result<(), String> {
    let app = window.app_handle().clone();
    let mut state = STATE.lock().unwrap();
    if active {
        let previous: Vec<_> = state.owned.values().copied().collect();
        let recording = Logged(&GlobalRegistrar(&app), "recording a shortcut");
        if let Err(error) = replace_bindings(&recording, &mut state.owned, &[]) {
            replace_bindings(&recording, &mut state.owned, &previous)
                .map_err(|_| "nr_shortcut_restore_failed")?;
            return Err(error);
        }
        let lease = (
            window.label().to_string(),
            std::time::Instant::now() + std::time::Duration::from_secs(30),
        );
        state.capture = Some(lease.clone());
        gamepad::wake();
        let _ = app.emit("nr-settings-changed", snapshot(&state));
        // Backend expiry also restores bindings when the recording window disappears.
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let result = {
                let mut state = STATE.lock().unwrap();
                if state.capture.as_ref() != Some(&lease) {
                    return;
                }
                let result = finish_capture(&mut state, &GlobalRegistrar(&app));
                (result, snapshot(&state))
            };
            let _ = app.emit("nr-settings-changed", result.1);
            if let Err(error) = result.0 {
                let _ = app.emit("nr-action-error", error);
            }
        });
    } else if state
        .capture
        .as_ref()
        .is_some_and(|(label, _)| label == window.label())
    {
        let result = finish_capture(&mut state, &GlobalRegistrar(&app));
        let status = snapshot(&state);
        drop(state);
        let _ = app.emit("nr-settings-changed", status);
        result?;
        return Ok(());
    }
    Ok(())
}
#[tauri::command]
pub async fn nr_overlay_settings() -> SettingsStatus {
    snapshot(&STATE.lock().unwrap())
}
#[tauri::command]
pub async fn nr_overlay_save_settings(
    app: AppHandle,
    patch: SettingsPatch,
) -> Result<SettingsStatus, String> {
    let result = {
        let mut state = STATE.lock().unwrap();
        let mut next = state.settings.clone();
        let update_shortcuts = patch.nr_shortcut.is_some()
            || patch.overlay_shortcut.is_some()
            || patch.nr_gamepad.is_some()
            || patch.overlay_gamepad.is_some();
        if let Some(key) = patch.nr_shortcut {
            next.nr_shortcut = normalized_key(&key)?;
        }
        if let Some(key) = patch.overlay_shortcut {
            next.overlay_shortcut = normalized_key(&key)?;
        }
        if let Some(key) = patch.nr_gamepad {
            next.nr_gamepad = gamepad::normalize(&key)?;
        }
        if let Some(key) = patch.overlay_gamepad {
            next.overlay_gamepad = gamepad::normalize(&key)?;
        }
        if let Some(opacity) = patch.opacity {
            if !(35..=95).contains(&opacity) {
                return Err("nr_opacity_invalid".into());
            }
            next.opacity = opacity;
        }
        let result = if update_shortcuts {
            apply_settings(&mut state, next, &GlobalRegistrar(&app), save)
        } else {
            save(&next).map(|_| state.settings = next)
        };
        let status = snapshot(&state);
        (result, status)
    };
    let _ = app.emit("nr-settings-changed", &result.1);
    result.0.map(|_| result.1)
}
#[tauri::command]
pub async fn nr_overlay_select_session(app: AppHandle, id: u64) {
    let mut state = STATE.lock().unwrap();
    state.target = Some(id);
    let status = snapshot(&state);
    drop(state);
    let _ = app.emit("nr-settings-changed", status);
}
pub fn visibility_requested() -> bool {
    VISIBLE.load(Ordering::Acquire)
}
pub(crate) fn show_window<R: Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    // The NR window is non-focusable so showing it keeps the game's keyboard focus.
    window.show().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                HWND_TOPMOST, IsWindowVisible, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
            },
        };
        let address = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        window
            .run_on_main_thread(move || {
                let hwnd = HWND(address as _);
                let flags = SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE;
                // Restack without activating or unhiding the window.
                unsafe {
                    if IsWindowVisible(hwnd).as_bool()
                        && let Err(error) = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags)
                    {
                        log::warn!("Failed to raise NR overlay: {error}");
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn request_visibility<R: Runtime>(app: &AppHandle<R>, visible: bool) {
    VISIBLE.store(visible, Ordering::Release);
    let _ = app.emit("nr-overlay-visibility", visible);
}
pub fn set_visible<R: Runtime>(app: &AppHandle<R>, visible: bool) -> Result<(), String> {
    request_visibility(app, visible);
    let result = if visible {
        crate::toolbar::create_tool_window_internal(app, "nr")
    } else if let Some(window) = app.get_webview_window("nr_overlay") {
        window.hide().map_err(|e| e.to_string())
    } else {
        Ok(())
    };
    if result.is_err() {
        let actual = app
            .get_webview_window("nr_overlay")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        request_visibility(app, actual);
    }
    result
}
#[tauri::command]
pub async fn nr_overlay_set_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    set_visible(&app, visible)
}
#[tauri::command]
pub fn nr_overlay_is_visible(app: AppHandle) -> bool {
    app.get_webview_window("nr_overlay")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}
async fn toggle_nr<R: Runtime>(_app: &AppHandle<R>) -> Result<(), String> {
    if TOGGLING.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            TOGGLING.store(false, Ordering::Release);
        }
    }
    let _guard = Guard;
    let config = crate::hdr_enhanced::get_config().await?;
    crate::native_components::set_global_nr_enabled(!config.settings.nr_enabled).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        keys: RefCell<Vec<u32>>,
        conflict: Option<u32>,
    }
    impl Registrar for Fake {
        fn register(&self, key: Shortcut) -> Result<(), String> {
            if self.conflict == Some(key.id()) {
                return Err("nr_shortcut_conflict".into());
            }
            self.keys.borrow_mut().push(key.id());
            Ok(())
        }
        fn unregister(&self, key: Shortcut) -> Result<(), String> {
            self.keys.borrow_mut().retain(|id| *id != key.id());
            Ok(())
        }
    }
    #[test]
    fn conflict_and_persistence_failure_keep_previous_bindings() {
        let mut state = OverlayState { nr_ready: true, ..OverlayState::default() };
        let old = bindings(&state.settings).unwrap()[0];
        state.owned.insert(old.id(), old);
        let new: Shortcut = "Ctrl+Alt+KeyK".parse().unwrap();
        let mut fake = Fake {
            keys: RefCell::new(vec![old.id()]),
            conflict: Some(new.id()),
        };
        let next = Settings {
            nr_shortcut: new.to_string(),
            ..Settings::default()
        };
        assert!(apply_settings(&mut state, next.clone(), &fake, |_| Ok(())).is_err());
        assert_eq!(state.settings, Settings::default());
        assert_eq!(*fake.keys.borrow(), vec![old.id()]);
        fake.conflict = None;
        assert!(apply_settings(&mut state, next, &fake, |_| Err("disk failed".into())).is_err());
        assert_eq!(state.settings, Settings::default());
        assert_eq!(*fake.keys.borrow(), vec![old.id()]);
    }
    #[test]
    fn canonical_duplicates_and_unmodified_keys_are_rejected() {
        assert!(normalized_key("KeyN").is_err());
        assert!(normalized_key("Ctrl+Alt+Shift+KeyT").is_err());
        let key = normalized_key("Ctrl+Alt+N").unwrap();
        assert!(bindings(&Settings {
            nr_shortcut: key.clone(),
            overlay_shortcut: key,
            ..Settings::default()
        })
        .is_err());
        assert_eq!(normalized_key("").unwrap(), "");
    }
    #[test]
    fn capture_restores_latest_bindings() {
        let mut state = OverlayState { nr_ready: true, ..OverlayState::default() };
        let fake = Fake {
            keys: RefCell::new(vec![]),
            conflict: None,
        };
        let old = bindings(&state.settings).unwrap();
        replace_bindings(&fake, &mut state.owned, &old).unwrap();
        replace_bindings(&fake, &mut state.owned, &[]).unwrap();
        state.capture = Some(("manager".into(), std::time::Instant::now()));
        let next = Settings {
            nr_shortcut: "Ctrl+Alt+KeyK".into(),
            ..Settings::default()
        };
        apply_settings(&mut state, next, &fake, |_| Ok(())).unwrap();
        assert!(fake.keys.borrow().is_empty());
        finish_capture(&mut state, &fake).unwrap();
        assert!(state.capture.is_none());
        assert_eq!(
            *fake.keys.borrow(),
            vec![bindings(&state.settings).unwrap()[0].id()]
        );
    }
    #[test]
    fn gamepad_settings_preserve_keyboard_and_old_settings_load() {
        let old: Settings =
            serde_json::from_str(r#"{"nrShortcut":"Ctrl+Alt+KeyN","opacity":62}"#).unwrap();
        assert_eq!(old, Settings::default());
        let mut state = OverlayState { nr_ready: true, ..OverlayState::default() };
        let fake = Fake {
            keys: RefCell::new(vec![]),
            conflict: None,
        };
        let keyboard = bindings(&state.settings).unwrap();
        replace_bindings(&fake, &mut state.owned, &keyboard).unwrap();
        let next = Settings {
            nr_gamepad: "LB+RB+X".into(),
            overlay_gamepad: "LB+RB+Y".into(),
            ..old
        };
        assert!(apply_settings(&mut state, next.clone(), &fake, |_| Err("disk".into())).is_err());
        assert!(state.settings.nr_gamepad.is_empty());
        apply_settings(&mut state, next.clone(), &fake, |_| Ok(())).unwrap();
        assert_eq!(state.settings, next);
        assert_eq!(*fake.keys.borrow(), vec![keyboard[0].id()]);
        let duplicate = Settings {
            overlay_gamepad: "LB+RB+X".into(),
            ..next
        };
        assert!(apply_settings(&mut state, duplicate, &fake, |_| Ok(())).is_err());
        assert_eq!(state.settings.overlay_gamepad, "LB+RB+Y");
    }
    fn status(installed: bool, host_supported: bool, adapter_present: bool) -> crate::native_components::providers::EnhancementComponentStatus {
        crate::native_components::providers::EnhancementComponentStatus { installed, host_supported, adapter_present, ..Default::default() }
    }
    #[test]
    fn shortcuts_only_for_an_installed_and_usable_dlss_nr() {
        assert_eq!(shortcuts_wanted(Some(&status(true, true, true))), (true, "DLSS NR installed and usable"));
        assert!(!shortcuts_wanted(Some(&status(false, true, true))).0);
        assert!(!shortcuts_wanted(Some(&status(true, false, true))).0);
        assert!(!shortcuts_wanted(Some(&status(true, true, false))).0);
        assert_eq!(shortcuts_wanted(None), (false, "DLSS NR status unknown"));
    }
    #[test]
    fn readiness_registers_after_install_and_releases_on_uninstall_keeping_the_bindings() {
        let mut state = OverlayState::default();
        let fake = Fake { keys: RefCell::new(vec![]), conflict: None };
        // not ready: a settings change is stored but holds no key
        let next = Settings { overlay_shortcut: "Ctrl+Alt+KeyO".into(), ..Settings::default() };
        apply_settings(&mut state, next.clone(), &fake, |_| Ok(())).unwrap();
        assert!(fake.keys.borrow().is_empty());
        assert_eq!(state.settings, next);
        // installed: both stored keys are registered
        assert!(set_ready(&mut state, true, &fake, "install: DLSS NR installed and usable"));
        let both: Vec<u32> = bindings(&next).unwrap().iter().map(|k| k.id()).collect();
        assert_eq!(fake.keys.borrow().len(), 2);
        assert!(both.iter().all(|id| fake.keys.borrow().contains(id)));
        assert!(!set_ready(&mut state, true, &fake, "status"));
        // uninstalled: released, the bindings stay stored for a reinstall
        assert!(set_ready(&mut state, false, &fake, "uninstall: DLSS NR not installed"));
        assert!(fake.keys.borrow().is_empty() && state.owned.is_empty());
        assert_eq!(state.settings, next);
        assert!(set_ready(&mut state, true, &fake, "install"));
        assert_eq!(fake.keys.borrow().len(), 2);
    }
    #[test]
    fn a_key_held_by_another_app_does_not_keep_the_other_back() {
        let mut state = OverlayState { settings: Settings { overlay_shortcut: "Ctrl+Alt+KeyO".into(), ..Settings::default() }, ..OverlayState::default() };
        let taken: Shortcut = "Ctrl+Alt+KeyN".parse().unwrap();
        let fake = Fake { keys: RefCell::new(vec![]), conflict: Some(taken.id()) };
        set_ready(&mut state, true, &fake, "install");
        let overlay: Shortcut = "Ctrl+Alt+KeyO".parse().unwrap();
        assert_eq!(*fake.keys.borrow(), vec![overlay.id()]);
        assert!(!state.owned.contains_key(&taken.id()));
    }

}
