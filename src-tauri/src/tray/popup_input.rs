//! The tray icon's input and the menus it opens in the Panel's menu window.
//!
//! The icon used to carry Tauri's native menu (menu.rs) and opened the main window on a click. Now what each gesture does is the person's choice (tray_prefs.rs, set in 个性化): the RIGHT click opens
//! one of three Material menus (foundation-ui MTrayMenu: V1 compact list, V2 status card + list, V3 quick panel) in the borderless menu window beside the icon (ctx_popup.rs `open_tray`, placed by
//! menu_place.rs against the side of the screen the taskbar is on); the LEFT click and the double click each open the main window, one of the menus, or nothing.
//!
//! Every item of those menus is one of the ids of the native menu, and a chosen item runs through the same handler (`handle_tray_menu_event`): nothing about what an item does changed.
//!
//! The native menu stays as the fallback: it is the icon's menu until the menu window has loaded, again when the person asks for it ("force native"), and again when the window fails twice in a
//! row. A menu that is attached to the icon makes Windows show it on a right click by itself, so while the popup is in use the icon has no menu at all.

use super::*;
use crate::ctx_popup::{self, TrayTarget};
use crate::menu_place::Rect as PhysRect;
use crate::tray_prefs::{self, Action, ClickGate, Decision, Gesture, Variant};
use serde_json::{Value, json};
use std::sync::atomic::AtomicU32;
use tauri::menu::Menu;

static GATE: Mutex<Option<ClickGate>> = Mutex::new(None);
/// Opens of the menu window that failed in a row; two switch the icon back to the native menu until the person changes the setting.
static FAILURES: AtomicU32 = AtomicU32::new(0);
const FAILURES_BEFORE_NATIVE: u32 = 2;

/// Every item id of the native menu (menu.rs) except the containers: what the three menus can send back, and all that `tray_popup_act` accepts.
pub(super) const ITEM_IDS: [&str; 37] = [
    "tray_status", "recover_service", "tray_notification_action", "open_main_panel", "connect_device", "download_client", "open_desktop", "auto_start", "open_sunshine", "vdd_settings", "vdd_create",
    "vdd_close", "vdd_toggle_keep_enabled", "vdd_toggle_headless_create", "reset_display", "show_toolbar", "host_performance", "rtss_control", "web_stream", "log_console", "debug_page", "lang_zh",
    "lang_en", "lang_ja", "prevent_sleep", "restart_user_mode", "import_config", "export_config", "reset_config", "clear_cache", "check_update", "about", "star_project", "visit_project_sunshine",
    "visit_project_moonlight", "restart", "shutdown",
];

/// Whether the icon carries the native menu now.
pub(super) fn use_native_menu() -> bool {
    tray_prefs::current().force_native || FAILURES.load(Ordering::SeqCst) >= FAILURES_BEFORE_NATIVE || !ctx_popup::available()
}

/// Gives the icon the native menu or takes it away, as `use_native_menu` says.
pub(super) fn apply_menu_mode<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if use_native_menu() {
        match build_tray_menu(app) {
            Ok(menu) => {
                if let Err(e) = tray.set_menu(Some(menu)) {
                    error!("tray: the native menu could not be attached: {}", e);
                }
            }
            Err(e) => error!("tray: the native menu could not be built: {}", e),
        }
    } else if let Err(e) = tray.set_menu(None::<Menu<R>>) {
        error!("tray: the native menu could not be removed: {}", e);
    }
}

/// The person saved the settings: the popup gets another chance.
pub(super) fn reset_failures() {
    FAILURES.store(0, Ordering::SeqCst);
}

fn physical_rect(rect: &tauri::Rect) -> PhysRect {
    let p = rect.position.to_physical::<f64>(1.0);
    let s = rect.size.to_physical::<f64>(1.0);
    PhysRect::new(p.x.round() as i32, p.y.round() as i32, (s.width.round() as i32).max(1), (s.height.round() as i32).max(1))
}

fn gate<T>(f: impl FnOnce(&mut ClickGate) -> T) -> T {
    let mut guard = GATE.lock().unwrap_or_else(|p| p.into_inner());
    f(guard.get_or_insert_with(ClickGate::default))
}

/// An event of the tray icon (called by the icon's callback on the main thread).
pub(super) fn on_icon_event(event: TrayIconEvent) {
    let Some(app) = ctx_popup::app() else { return };
    let prefs = tray_prefs::current();
    match event {
        TrayIconEvent::Click { button: MouseButton::Left, button_state: tauri::tray::MouseButtonState::Down, .. } => gate(|g| g.left_down()),
        TrayIconEvent::Click { button: MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, rect, .. } => {
            let decision = gate(|g| g.left_up(&prefs, tray_prefs::double_click_wait()));
            run(&app, decision, physical_rect(&rect));
        }
        TrayIconEvent::DoubleClick { button: MouseButton::Left, rect, .. } => {
            let decision = gate(|g| g.double_click(&prefs));
            run(&app, decision, physical_rect(&rect));
        }
        TrayIconEvent::Click { button: MouseButton::Right, button_state: tauri::tray::MouseButtonState::Up, rect, .. } => {
            // with the native menu on the icon, the system shows it by itself on this very event
            if !use_native_menu() {
                perform(&app, prefs.action(Gesture::Right), physical_rect(&rect), true);
            }
        }
        _ => {}
    }
}

fn run(app: &AppHandle, decision: Decision, icon: PhysRect) {
    match decision {
        Decision::Now(action) => perform(app, action, icon, false),
        Decision::After(wait, ticket, action) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(wait).await;
                if gate(|g| g.fire(ticket)) {
                    perform(&app, action, icon, false);
                }
            });
        }
        Decision::Ignore => {}
    }
}

fn perform(app: &AppHandle, action: Action, icon: PhysRect, from_right: bool) {
    match action {
        Action::None => {}
        Action::Panel => open_main_panel_from_tray(app, "click"),
        Action::Menu(variant) => {
            if use_native_menu() {
                // the native menu cannot be opened by a left click: the main window opens, as the icon always did
                if !from_right {
                    open_main_panel_from_tray(app, "click (native menu)");
                }
                return;
            }
            open_menu(app, variant, icon);
        }
    }
}

fn open_menu(app: &AppHandle, variant: Variant, icon: PhysRect) {
    // the press that closed the menu on screen is released on the icon now: that is the person closing the menu, not opening it again
    if tray_prefs::just_closed_by_icon_press() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = build_state(&app).await;
        match ctx_popup::open_tray(app.clone(), variant, state, TrayTarget { icon }).await {
            Ok(()) => FAILURES.store(0, Ordering::SeqCst),
            Err(e) => {
                let failures = FAILURES.fetch_add(1, Ordering::SeqCst) + 1;
                warn!("tray: the {} menu could not be shown ({} in a row): {}", variant.as_str(), failures, e);
                if failures >= FAILURES_BEFORE_NATIVE {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || apply_menu_mode(&handle));
                }
            }
        }
    });
}

/// A menu of the tray is on screen: send it the state of now (called when the host state changes).
pub(super) fn push_state_if_open() {
    if !ctx_popup::tray_menu_open() {
        return;
    }
    let Some(app) = ctx_popup::app() else { return };
    tauri::async_runtime::spawn(async move {
        let state = build_state(&app).await;
        ctx_popup::update_tray_state(&app, state);
    });
}

// ---- the state a menu is drawn from ---------------------------------------------------------------------------------------------------------------------------------------------------------

const LIVE_TIMEOUT: Duration = Duration::from_millis(450);

fn host_word(status: &str) -> &'static str {
    match status {
        "streaming" => "streaming",
        "paused" => "paused",
        "pairing" => "pairing",
        _ => "idle",
    }
}

/// "1440p · 120 Hz" for a mode of 2560x1440 at 120.
pub(super) fn short_mode(height: u32, fps: u32) -> String {
    let label = if height >= 2160 { "4K".to_string() } else { format!("{height}p") };
    if fps > 0 { format!("{label} · {fps} Hz") } else { label }
}

/// The streams as the menu draws them: client, app, bitrate in Mbps (null when unknown), frame rate.
pub(super) fn session_rows(sessions: &[sunshine::SessionInfo]) -> Vec<Value> {
    sessions
        .iter()
        .map(|x| {
            json!({
                "name": x.client_name,
                "app": if x.app_name.trim().is_empty() { Value::Null } else { Value::String(x.app_name.clone()) },
                "bitrate": if x.bitrate > 0 { json!(f64::from(x.bitrate) / 1000.0) } else { Value::Null },
                "fps": x.fps,
            })
        })
        .collect()
}

/// The labels of the native menu in the person's language, by id (the menus show the same words).
pub(super) fn labels(s: &TrayStrings) -> Value {
    let mut m = serde_json::Map::new();
    let mut put = |id: &str, text: &str| {
        m.insert(id.to_string(), Value::String(text.to_string()));
    };
    put("recover_service", s.recover_service);
    put("tray_notification_action", s.notification);
    put("open_main_panel", s.open_main_panel);
    put("connect_device", s.connect_device);
    put("download_client", s.download_client);
    put("open_desktop", s.open_desktop);
    #[cfg(target_os = "windows")]
    put("auto_start", s.auto_start);
    put("open_sunshine", s.open_sunshine);
    put("vdd_settings", s.vdd_settings);
    put("vdd_create", s.vdd_create);
    put("vdd_close", s.vdd_close);
    put("vdd_toggle_keep_enabled", s.vdd_keep);
    put("vdd_toggle_headless_create", s.vdd_headless);
    put("reset_display", s.reset_display);
    put("show_toolbar", s.show_toolbar);
    put("host_performance", s.host_performance);
    put("rtss_control", s.rtss_control);
    #[cfg(any(debug_assertions, feature = "beta"))]
    put("web_stream", s.web_stream);
    put("log_console", s.log_console);
    #[cfg(debug_assertions)]
    put("debug_page", s.debug_page);
    put("lang_zh", "中文");
    put("lang_en", "English");
    put("lang_ja", "日本語");
    put("prevent_sleep", s.prevent_sleep);
    put("restart_user_mode", s.restart_user_mode);
    put("import_config", s.import_config);
    put("export_config", s.export_config);
    put("reset_config", s.reset_config);
    put("clear_cache", s.clear_cache);
    put("check_update", s.check_update);
    put("about", s.about);
    put("star_project", s.star_project);
    put("visit_project_sunshine", s.visit_project_sunshine);
    put("visit_project_moonlight", s.visit_project_moonlight);
    put("restart", s.restart);
    put("shutdown", s.shutdown);
    Value::Object(m)
}

/// The state of the host for a menu: what the native menu knows (menu.rs) and what it does not show (streams, display mode, update, DS5).
pub(super) async fn build_state(_app: &AppHandle) -> Value {
    let s = get_tray_strings();
    let locale = get_current_locale();
    let (tray_state, connection, _) = TRAY_RUNTIME_STATE.lock().unwrap_or_else(|p| p.into_inner()).menu_snapshot();
    let connected = connection == CoreConnectionState::Connected;
    let (live, ds5) = if connected {
        let sessions = tokio::time::timeout(LIVE_TIMEOUT, sunshine::get_active_sessions());
        let pad = tokio::time::timeout(LIVE_TIMEOUT, crate::dualsense::dualsense_get_status());
        let (sessions, pad) = tokio::join!(sessions, pad);
        (
            sessions.ok().and_then(Result::ok),
            pad.ok().and_then(Result::ok).is_some_and(|status| status.state == "active"),
        )
    } else {
        (None, false)
    };
    let streams: Vec<Value> = match (&live, &tray_state) {
        (Some(rows), _) => session_rows(rows),
        // the live list did not come in time: the names the core's own tray state has
        (None, Some(state)) if connected => state
            .sessions
            .iter()
            .map(|x| json!({ "name": x.client_name, "app": if state.app_name.trim().is_empty() { Value::Null } else { Value::String(state.app_name.clone()) }, "bitrate": Value::Null, "fps": 0 }))
            .collect(),
        _ => Vec::new(),
    };
    let first = live.as_ref().and_then(|rows| rows.first());
    let vdd_active = tray_state.as_ref().is_some_and(|state| state.vdd.active);
    let (vdd_mode, vdd_short) = match first {
        Some(x) if vdd_active && x.width > 0 && x.height > 0 => (format!("{}×{}@{}", x.width, x.height, x.fps), short_mode(x.height, x.fps)),
        _ => (String::new(), String::new()),
    };
    let notification = tray_state.as_ref().filter(|state| state.notification.active).map(|state| menu::tray_notification_label(s, state));
    let update_version = update::last_seen_update();
    #[cfg(target_os = "windows")]
    let (auto_start, user_mode) = (desktop_settings::is_combined_auto_start_enabled(), *SUNSHINE_USER_MODE_STATE.lock().unwrap_or_else(|p| p.into_inner()));
    #[cfg(not(target_os = "windows"))]
    let (auto_start, user_mode) = (false, false);

    json!({
        "locale": locale,
        "connection": match connection { CoreConnectionState::Connected => "connected", CoreConnectionState::Connecting => "connecting", CoreConnectionState::Disconnected => "disconnected" },
        "host": tray_state.as_ref().map(|state| host_word(&state.status)).unwrap_or("idle"),
        "rtss": crate::rtss::is_rtss_installed(),
        "dev": cfg!(any(debug_assertions, feature = "beta")),
        "sessions": streams,
        "vdd": {
            "active": vdd_active,
            "cooldown": tray_state.as_ref().is_some_and(|state| state.vdd.cooldown),
            "mode": vdd_mode,
            "short": vdd_short,
        },
        "toggles": {
            "auto_start": auto_start,
            "prevent_sleep": *PREVENT_SLEEP_STATE.lock().unwrap_or_else(|p| p.into_inner()),
            "vdd_toggle_keep_enabled": tray_state.as_ref().is_some_and(|state| state.vdd.keep_enabled),
            "vdd_toggle_headless_create": tray_state.as_ref().is_some_and(|state| state.vdd.headless_create_enabled),
            "restart_user_mode": user_mode,
        },
        "update": update_version.map(|version| json!({ "version": version })),
        "ds5": ds5,
        "notification": notification,
        "labels": labels(s),
    })
}

// ---- commands -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// The page of the menu window chose an item: the window closes and the item runs through the native menu's handler.
pub(super) fn act(app: &AppHandle, token: u64, id: &str) {
    if !ITEM_IDS.contains(&id) || id == "tray_status" {
        warn!("tray: the menu window sent an unknown item '{}'", id);
        ctx_popup::pick_tray(app, token);
        return;
    }
    if !ctx_popup::pick_tray(app, token) {
        return; // not the menu that is on screen (a late answer)
    }
    handle_tray_menu_event(app, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIONS: &str = include_str!("actions.rs");
    const MENU: &str = include_str!("menu.rs");

    #[test]
    fn the_item_ids_are_the_ids_menu_rs_builds() {
        assert_eq!(ITEM_IDS.len(), 37);
        for id in ITEM_IDS {
            assert!(MENU.contains(&format!("\"{id}\"")), "menu.rs has no '{id}'");
        }
        let mut sorted = ITEM_IDS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 37, "no id twice");
    }

    #[test]
    fn every_actionable_id_has_its_arm_in_the_one_handler() {
        for id in ITEM_IDS.iter().filter(|id| **id != "tray_status") {
            assert!(ACTIONS.contains(&format!("\"{id}\" =>")), "handle_tray_menu_event has no arm for '{id}'");
        }
    }

    #[test]
    fn the_menus_carry_the_words_of_the_native_menu_in_every_language() {
        for s in [&ZH_STRINGS, &EN_STRINGS, &JA_STRINGS] {
            let labels = labels(s);
            let map = labels.as_object().unwrap();
            for id in ITEM_IDS.iter().filter(|id| **id != "tray_status") {
                #[cfg(not(target_os = "windows"))]
                if *id == "auto_start" {
                    continue;
                }
                #[cfg(not(any(debug_assertions, feature = "beta")))]
                if *id == "web_stream" {
                    continue;
                }
                #[cfg(not(debug_assertions))]
                if *id == "debug_page" {
                    continue;
                }
                assert!(map.get(*id).and_then(Value::as_str).is_some_and(|t| !t.is_empty()), "no label for '{id}'");
            }
        }
        assert_eq!(labels(&ZH_STRINGS)["vdd_create"], "创建基地虚拟显示器");
        assert_eq!(labels(&EN_STRINGS)["vdd_settings"], "Foundation Virtual Display Settings");
    }

    #[test]
    fn streams_are_shown_with_megabits_and_an_unknown_rate_is_null() {
        let row = |name: &str, bitrate: u32| sunshine::SessionInfo {
            client_name: name.into(), client_address: String::new(), state: "RUNNING".into(), session_id: 1, width: 2560, height: 1440, fps: 120, bitrate,
            host_audio: true, enable_hdr: false, enable_mic: false, app_name: "Desktop".into(), app_id: 1,
        };
        let rows = session_rows(&[row("iPad", 48_200), row("Deck", 0)]);
        assert_eq!(rows[0]["bitrate"], json!(48.2));
        assert_eq!(rows[0]["app"], "Desktop");
        assert!(rows[1]["bitrate"].is_null());
        assert_eq!(short_mode(1440, 120), "1440p · 120 Hz");
        assert_eq!(short_mode(2160, 60), "4K · 60 Hz");
        assert_eq!(short_mode(1080, 0), "1080p");
    }

    #[test]
    fn the_host_word_is_one_the_menus_know() {
        assert_eq!([host_word("streaming"), host_word("paused"), host_word("pairing"), host_word("idle"), host_word("notification"), host_word("")], ["streaming", "paused", "pairing", "idle", "idle", "idle"]);
    }
}
