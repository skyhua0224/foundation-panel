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

use super::display_sliders::{self, Facts, Kind};
use super::*;
use crate::ctx_popup::{self, TrayTarget};
use crate::menu_place::Rect as PhysRect;
use crate::dpi_scale_core::ScaleRange;
use crate::tray_prefs::{self, Action, ClickGate, Decision, Gesture, Variant};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, AtomicU32};
use tauri::menu::Menu;

static GATE: Mutex<Option<ClickGate>> = Mutex::new(None);
/// Opens of the menu window that failed in a row; two switch the icon back to the native menu until the person changes the setting.
static FAILURES: AtomicU32 = AtomicU32::new(0);
const FAILURES_BEFORE_NATIVE: u32 = 2;
/// How long a menu asked for while the window starts waits for its page (a cold WebView2 of a tray-only start).
const WINDOW_WAIT: Duration = Duration::from_millis(12_000);

/// Every item id of the native menu (menu.rs) except the containers: what the three menus can send back, and all that `tray_popup_act` accepts.
pub(super) const ITEM_IDS: [&str; 46] = [
    "tray_status", "recover_service", "tray_notification_action", "open_main_panel", "connect_device", "download_client", "open_desktop", "auto_start", "open_sunshine", "vdd_settings", "vdd_create",
    "vdd_close", "vdd_toggle_keep_enabled", "vdd_toggle_headless_create", "reset_display", "show_toolbar", "host_performance", "rtss_control", "web_stream", "log_console", "debug_page", "lang_zh",
    "lang_en", "lang_ja", "prevent_sleep", "restart_user_mode", "import_config", "export_config", "reset_config", "clear_cache", "check_update", "about", "feedback", "star_project", "visit_project_sunshine",
    "visit_project_moonlight", "visit_site", "visit_bilibili", "star_github", "sponsor_qiin", "sponsor_yundi339", "sponsor_skyhua", "sponsor_wechat", "sponsor_github", "restart", "shutdown",
];

/// Whether the icon carries the native menu now: the person asked for it, the menu window failed twice in a row, or WebView2 would not make the window at all (it is switched off, or every attempt of
/// `ctx_popup::schedule_create_after` failed: the last resort of a machine whose WebView2 is broken). The tray menu is the Material menu in every memory tier and at every pressure: the window stays
/// up, and a click while it is still being made waits for it (`open_menu`) instead of taking the native menu.
pub(super) fn use_native_menu() -> bool {
    wants_native(tray_prefs::current().force_native, FAILURES.load(Ordering::SeqCst), ctx_popup::health())
}

/// The decision behind `use_native_menu`, apart from the state it reads.
pub(super) fn wants_native(force_native: bool, failures: u32, health: ctx_popup::Health) -> bool {
    force_native || failures >= FAILURES_BEFORE_NATIVE || matches!(health, ctx_popup::Health::Disabled | ctx_popup::Health::Failed)
}

/// Whether the icon carries the native menu right now (set where the menu is attached or taken away): while it does, Windows shows that menu on a right click by itself.
static NATIVE_ATTACHED: AtomicBool = AtomicBool::new(false);

pub(super) fn note_native_attached(attached: bool) {
    NATIVE_ATTACHED.store(attached, Ordering::SeqCst);
}

/// What a right click on the icon comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RightClick {
    /// the menu window opens
    Window,
    /// the icon carries the native menu: the system shows it, nothing is done here (two menus on one click is the worst outcome)
    System,
    /// the native menu is wanted but not attached yet (it is being put on): nothing shows for this click
    Nothing,
}

pub(super) fn right_click_plan(wants_native: bool, attached: bool) -> RightClick {
    match (attached, wants_native) {
        (true, _) => RightClick::System,
        (false, false) => RightClick::Window,
        (false, true) => RightClick::Nothing,
    }
}

/// Gives the icon the native menu or takes it away, as `use_native_menu` says.
pub(super) fn apply_menu_mode<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    if use_native_menu() {
        match build_tray_menu(app) {
            Ok(menu) => {
                match tray.set_menu(Some(menu)) {
                    Ok(()) => note_native_attached(true),
                    Err(e) => error!("tray: the native menu could not be attached: {}", e),
                }
                refresh_native_sliders_if_native(app);
            }
            Err(e) => error!("tray: the native menu could not be built: {}", e),
        }
    } else {
        match tray.set_menu(None::<Menu<R>>) {
            Ok(()) => note_native_attached(false),
            Err(e) => error!("tray: the native menu could not be removed: {}", e),
        }
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
    // first of all, while the click message is still the one being dispatched: what was it, a finger, a pen or a mouse (input_source.rs)
    input_source::note_icon_event();
    let Some(app) = ctx_popup::app() else { return };
    let prefs = tray_prefs::current();
    match event {
        TrayIconEvent::Click { button: MouseButton::Left, button_state: tauri::tray::MouseButtonState::Down, .. } => {
            gate(|g| g.left_down());
            if matches!(prefs.action(Gesture::Click), Action::Panel) || matches!(prefs.action(Gesture::Double), Action::Panel) {
                crate::mem_adaptive::prewarm_main(&app, "tray press");
            }
        },
        TrayIconEvent::Click { button: MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, rect, .. } => {
            let decision = gate(|g| g.left_up(&prefs, tray_prefs::double_click_wait()));
            run(&app, decision, physical_rect(&rect));
        }
        TrayIconEvent::DoubleClick { button: MouseButton::Left, rect, .. } => {
            let decision = gate(|g| g.double_click(&prefs));
            run(&app, decision, physical_rect(&rect));
        }
        // the pointer is on the icon (the events repeat while it moves): a main window that was let go for memory is made again now, hidden, so that the click that follows finds it
        TrayIconEvent::Enter { .. } | TrayIconEvent::Move { .. } => crate::mem_adaptive::prewarm_main(&app, "tray hover"),
        TrayIconEvent::Click { button: MouseButton::Right, button_state: tauri::tray::MouseButtonState::Up, rect, .. } => {
            crate::mem_adaptive::note_click();
            // with the native menu on the icon, the system shows it by itself on this very event
            match right_click_plan(use_native_menu(), NATIVE_ATTACHED.load(Ordering::SeqCst)) {
                RightClick::Window => perform(&app, prefs.action(Gesture::Right), physical_rect(&rect), true),
                RightClick::System => {}
                RightClick::Nothing => debug!("tray: right click while the native menu is being attached: nothing shown"),
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
        // the window is still being made (a tray-only start, or it was lost and is made again): the menu waits for it; the native menu takes the icon only when the window cannot show
        if ctx_popup::health() == ctx_popup::Health::Starting && !ctx_popup::wait_loaded(WINDOW_WAIT).await {
            warn!("tray: the menu window did not load in {} ms: the native menu takes the icon until it does", WINDOW_WAIT.as_millis());
            ctx_popup::mark_failed();
        }
        if ctx_popup::health() != ctx_popup::Health::Ready {
            // WebView2 would not make the window yet: the click is answered by the native menu, and the window is tried again now
            if ctx_popup::health() == ctx_popup::Health::Failed {
                ctx_popup::schedule_create_after(&app, Duration::ZERO);
            }
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || apply_menu_mode(&handle));
            return;
        }
        let state = build_state(&app).await;
        match ctx_popup::open_tray(app.clone(), variant, state, TrayTarget { icon }).await {
            Ok(true) => FAILURES.store(0, Ordering::SeqCst),
            // closed or replaced before it was drawn (a second click, the focus went elsewhere): the person did it, nothing failed
            Ok(false) => {}
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

/// "1440p@120" for a mode of 2560x1440 at 120 (short: it is the sub-label of a tile).
pub(super) fn short_mode(height: u32, fps: u32) -> String {
    let label = if height >= 2160 { "4K".to_string() } else { format!("{height}p") };
    if fps > 0 { format!("{label}@{fps}") } else { label }
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
    put("feedback", s.feedback);
    put("star_project", s.star_project);
    put("visit_project_sunshine", s.visit_project_sunshine);
    put("visit_project_moonlight", s.visit_project_moonlight);
    put("visit_site", s.visit_site);
    put("visit_bilibili", s.visit_bilibili);
    put("star_github", s.star_github);
    put("sponsor_qiin", "qiin2333");
    put("sponsor_yundi339", "Yundi339");
    put("sponsor_skyhua", "skyhua0224");
    put("sponsor_wechat", "微信赞赏 / WeChat");
    put("sponsor_github", "GitHub Sponsors");
    put("restart", s.restart);
    put("shutdown", s.shutdown);
    Value::Object(m)
}

/// The state of the host for a menu: what the native menu knows (menu.rs) and what it does not show (streams, display mode, update, DS5).
pub(super) async fn build_state(app: &AppHandle) -> Value {
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
    let stream = match first {
        Some(x) => Some((x.width, x.height, x.fps)),
        // the live list did not come: a stream is still on, its display is the one in use (the primary one when none matches)
        None if connected && tray_state.as_ref().is_some_and(|state| host_word(&state.status) == "streaming") => Some((0, 0, 0)),
        None => None,
    };
    let sliders = slider_state(connected, vdd_active, stream).await;
    let notification = tray_state.as_ref().filter(|state| state.notification.active).map(|state| menu::tray_notification_label(s, state));
    let update_version = update::last_seen_update();
    #[cfg(target_os = "windows")]
    let (auto_start, user_mode) = (desktop_settings::is_combined_auto_start_enabled(), *SUNSHINE_USER_MODE_STATE.lock().unwrap_or_else(|p| p.into_inner()));
    #[cfg(not(target_os = "windows"))]
    let (auto_start, user_mode) = (false, false);

    let prefs = tray_prefs::current();
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
            "show_toolbar": app.get_webview_window("toolbar").is_some(),
        },
        "update": update_version.map(|version| json!({ "version": version })),
        "ds5": ds5,
        "notification": notification,
        "sliders": sliders,
        // what the press that opened the menu was, and the person's density choice: the page opens in touch density for a finger (input_source.rs; foundation-ui MTrayMenu)
        "touch": input_source::facts().to_json(),
        "density": prefs.density,
        // the tiles and sliders the person chose for the quick panel (null = the default of the menu)
        "layout": { "tiles": prefs.tiles, "sliders": prefs.sliders },
        "labels": labels(s),
    })
}

// ---- the sliders (display_sliders.rs) -----------------------------------------------------------------------------------------------------------------------------------------------------

const VIRTUAL_OUTPUT: &str = "ZakoHDR";
const DISPLAY_TIMEOUT: Duration = Duration::from_millis(1500);

fn config_text(config: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    config.get(key).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|n| n.to_string())))
}

/// The displays listed a moment ago: the state is rebuilt on every change of the host's state and listing every mode of every display is not free. A slider that was set lists afresh.
static OUTPUT_CACHE: Mutex<Option<(std::time::Instant, Vec<display_sliders::Output>)>> = Mutex::new(None);
const OUTPUT_CACHE_FOR: Duration = Duration::from_millis(1500);

/// The displays of the machine now (Win32, off the async threads) and what decides which of them the sliders change.
async fn displays_and_facts(vdd_active: bool, stream: Option<(u32, u32, u32)>, fresh: bool) -> (Vec<display_sliders::Output>, Facts, serde_json::Map<String, Value>) {
    let cached = if fresh { None } else { OUTPUT_CACHE.lock().unwrap_or_else(|p| p.into_inner()).as_ref().filter(|(at, _)| at.elapsed() < OUTPUT_CACHE_FOR).map(|(_, list)| list.clone()) };
    let outputs = match cached {
        Some(list) => list,
        None => {
            let list: Vec<display_sliders::Output> = tokio::time::timeout(DISPLAY_TIMEOUT, tokio::task::spawn_blocking(display_sliders::outputs)).await.ok().and_then(Result::ok).unwrap_or_default();
            *OUTPUT_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = Some((std::time::Instant::now(), list.clone()));
            list
        }
    };
    let config = crate::vdd::read_full_sunshine_config().await.unwrap_or_default();
    let output_is_virtual = config_text(&config, "output_name").as_deref() == Some(VIRTUAL_OUTPUT);
    (outputs, Facts { virtual_active: vdd_active, output_is_virtual, stream }, config)
}

/// `state.sliders`: the target display and what it can do now, the scale, the bitrate cap (only with a connected core).
async fn slider_state(connected: bool, vdd_active: bool, stream: Option<(u32, u32, u32)>) -> Value {
    let (outputs, facts, config) = displays_and_facts(vdd_active, stream, false).await;
    let target = display_sliders::choose(&outputs, &facts);
    // the scale is a property of the display the sliders point at (the stream's, else the virtual display, else the primary one), and what Windows lets that display take is read now: it changes with the
    // display and its resolution
    let device = target.and_then(|(i, _)| outputs.get(i)).map(|o| o.device.clone());
    let dpi = match crate::dpi_scale::read_range(device).await {
        Ok(range) => Some(range),
        Err(error) => {
            warn!("tray: the scale range of the target display could not be read: {}", error);
            None
        }
    };
    let bitrate = connected.then(|| config_text(&config, "max_bitrate").and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0));
    display_sliders::sliders_json(&outputs, target, dpi.as_ref(), bitrate)
}

/// The facts of the moment a slider is released (the menu's state may be a second old).
async fn live_facts() -> (bool, Option<(u32, u32, u32)>) {
    let (tray_state, connection, _) = TRAY_RUNTIME_STATE.lock().unwrap_or_else(|p| p.into_inner()).menu_snapshot();
    let connected = connection == CoreConnectionState::Connected;
    let vdd_active = tray_state.as_ref().is_some_and(|state| state.vdd.active);
    let sessions = if connected { tokio::time::timeout(LIVE_TIMEOUT, sunshine::get_active_sessions()).await.ok().and_then(Result::ok) } else { None };
    let stream = match sessions.as_ref().and_then(|rows| rows.first()) {
        Some(x) => Some((x.width, x.height, x.fps)),
        None if connected && tray_state.as_ref().is_some_and(|state| host_word(&state.status) == "streaming") => Some((0, 0, 0)),
        None => None,
    };
    (vdd_active, stream)
}

/// The display whose scale the sliders and the Panel's pages change: the target of the mode sliders (the stream's display, else the virtual display, else the primary one). None when Windows lists no
/// display at all; the primary display is then the one.
pub(super) async fn scale_display() -> Option<String> {
    let (vdd_active, stream) = live_facts().await;
    let (outputs, facts, _) = displays_and_facts(vdd_active, stream, false).await;
    display_sliders::choose(&outputs, &facts).and_then(|(i, _)| outputs.get(i)).map(|o| o.device.clone())
}

/// Sets a mode of the target display: the new resolution (keeping the rate where the display has it) or the new refresh rate.
async fn change_mode(want_size: Option<(u32, u32)>, want_hz: Option<u32>) -> Result<(), String> {
    let (vdd_active, stream) = live_facts().await;
    let (_, facts, _) = displays_and_facts(vdd_active, stream, true).await;
    tokio::task::spawn_blocking(move || {
        let outputs = display_sliders::outputs();
        let (at, kind) = display_sliders::choose(&outputs, &facts).ok_or("没有可调整的显示器")?;
        let output = &outputs[at];
        let mode = display_sliders::pick_mode(output, want_size, want_hz).ok_or("显示器没有这个模式")?;
        if mode == output.current {
            return Ok(());
        }
        let which = match kind {
            Kind::Stream => "stream display",
            Kind::Virtual => "virtual display",
            Kind::Desktop => "primary display",
        };
        info!("tray: {} {} -> {}x{}@{}", which, output.device, mode.width, mode.height, mode.hz);
        display_sliders::set_mode(&output.device, mode)
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Writes one slider's value to the machine (the display's mode, the scale, the bitrate cap) and returns how long the system needs to show it, and a notice for the page (null; for the scale, what came of a
/// step the display does not allow without signing out: `{ requested, applied, clamped: true, max }`). The quick page and the native menu's stepped items both come here, so the two cannot disagree
/// about what a value means.
async fn write_slider(key: &str, value: &Value) -> Result<(Duration, Value), String> {
    let mut notice = Value::Null;
    let settle = match key {
        "resolution" => {
            let size = value.as_str().and_then(display_sliders::parse_resolution).ok_or("分辨率无效")?;
            change_mode(Some(size), None).await?;
            Duration::from_millis(700)
        }
        "refresh" => {
            let hz = value.as_u64().and_then(|n| u32::try_from(n).ok()).filter(|n| (24..=1000).contains(n)).ok_or("刷新率无效")?;
            change_mode(None, Some(hz)).await?;
            Duration::from_millis(700)
        }
        "dpi" => {
            let dpi = value.as_u64().and_then(|n| u32::try_from(n).ok()).filter(|n| (100..=500).contains(n)).ok_or("缩放无效")?;
            // the display the sliders point at takes the nearest step it allows without signing out; a request beyond that is set to the highest step and the page is told (never refused silently)
            let (resolved, range) = crate::dpi_scale::set_scale(scale_display().await, dpi).await?;
            if resolved.clamped {
                info!("tray: scale {}% is beyond what the display allows without signing out; {}% is set", resolved.requested, resolved.applied);
                notice = crate::dpi_scale_core::outcome_json(resolved, range.max());
            }
            // the call is synchronous: Windows has the new scale when it returns, its windows take it a moment later
            Duration::from_millis(500)
        }
        "bitrate" => {
            let kbps = value.as_u64().filter(|n| *n <= 800_000).ok_or("码率无效")?;
            // the configuration API replaces the whole table: read it all, change the one key, write it back under the lock every such writer takes
            let _guard = crate::device_config::DEVICE_CONFIG_LOCK.lock().await;
            let mut config = crate::vdd::read_full_sunshine_config().await?;
            config.insert("max_bitrate".to_string(), json!(kbps.to_string()));
            // The host applies the ceiling to the running process and to every running session before it answers; its answer says what is in force. A host that says something else is told to the person.
            let answer = sunshine::post_sunshine_config_reporting(&config).await?;
            check_ceiling_in_force(&answer, kbps)?;
            Duration::from_millis(100)
        }
        other => return Err(format!("未知的滑块 {other}")),
    };
    Ok((settle, notice))
}

/// The host's answer to a saved bitrate ceiling: when it says which one is in force (a host that applies it at once does), it must be the one that was asked for. A host that does not say is not doubted.
fn check_ceiling_in_force(answer: &Value, asked_kbps: u64) -> Result<(), String> {
    match answer.get("max_bitrate_in_force").and_then(Value::as_u64) {
        Some(in_force) if in_force != asked_kbps => Err(format!("主机实际生效的码率上限是 {} Kbps", in_force)),
        _ => Ok(()),
    }
}

/// A slider of the quick page was released (`tray_popup_set`): the menu stays open; the answer comes when the value is set, and the state the host ended up in is pushed first. The answer is the notice
/// of `write_slider` (null when there is nothing to say).
pub(super) async fn set(app: &AppHandle, key: &str, value: &Value) -> Result<Value, String> {
    if !ctx_popup::tray_menu_open() {
        return Err("菜单已经关闭".to_string());
    }
    let (settle, notice) = write_slider(key, value).await?;
    tokio::time::sleep(settle).await;
    *OUTPUT_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = None; // what the display has now, not what it had
    ctx_popup::update_tray_state(app, build_state(app).await);
    Ok(notice)
}

// ---- the native menu's stepped sliders --------------------------------------------------------------------------------------------------------------------------------------------------------
//
// The native menu cannot draw a slider, so 码率上限 and 缩放 are two submenus of steps (the same two values the menus' sliders set, through `write_slider`). Which of them the menu has follows the same
// list as the sliders of the three menus (`layout.sliders`, default bitrate + dpi); the ticks show what was read a moment ago (the menu is built synchronously).

/// The steps of the native 码率上限 submenu, in Mbps (0 = 自动). The 缩放 submenu has the steps the display allows now (`ScaleRange::options`, read with the ticks); `NATIVE_DPI_STEPS` is every step Windows
/// has, which is all an item id may name (`slider_dpi_N`).
pub(super) const NATIVE_BITRATE_MBPS: [u64; 5] = [0, 10, 20, 50, 100];
pub(super) const NATIVE_DPI_STEPS: [u32; 12] = crate::dpi_scale_core::DPI_VALS;
const NATIVE_SLIDER_PREFIX: &str = "slider_";

/// (bitrate cap in kbps, what the target display allows for the scale) as the native menu last read them.
static NATIVE_SLIDERS: Mutex<(Option<u64>, Option<ScaleRange>)> = Mutex::new((None, None));

pub(super) fn native_slider_values() -> (Option<u64>, Option<ScaleRange>) {
    NATIVE_SLIDERS.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// The sliders the native menu has: the person's list (`layout.sliders`) else the default, only those that have a step list.
pub(super) fn native_slider_keys() -> Vec<String> {
    let list = tray_prefs::current().sliders.unwrap_or_else(|| vec!["bitrate".to_string(), "dpi".to_string()]);
    list.into_iter().filter(|k| k == "bitrate" || k == "dpi").collect()
}

/// `slider_bitrate_20` -> ("bitrate", 20000 kbps); `slider_dpi_150` -> ("dpi", 150); anything else None.
pub(super) fn parse_native_slider(id: &str) -> Option<(&'static str, u64)> {
    let rest = id.strip_prefix(NATIVE_SLIDER_PREFIX)?;
    if let Some(n) = rest.strip_prefix("bitrate_").and_then(|n| n.parse::<u64>().ok()).filter(|n| NATIVE_BITRATE_MBPS.contains(n)) {
        return Some(("bitrate", n * 1000));
    }
    rest.strip_prefix("dpi_").and_then(|n| n.parse::<u64>().ok()).filter(|n| NATIVE_DPI_STEPS.iter().any(|d| u64::from(*d) == *n)).map(|n| ("dpi", n))
}

/// Reads the cap and the scale (and the steps the display allows) for the ticks of the native menu, then rebuilds the menu when anything it shows changed.
pub(super) async fn refresh_native_sliders<R: Runtime + 'static>(app: AppHandle<R>) {
    let (_, connection, _) = TRAY_RUNTIME_STATE.lock().unwrap_or_else(|p| p.into_inner()).menu_snapshot();
    let connected = connection == CoreConnectionState::Connected;
    let keys = native_slider_keys();
    let bitrate = if connected && keys.iter().any(|k| k == "bitrate") {
        let config = crate::vdd::read_full_sunshine_config().await.unwrap_or_default();
        Some(config_text(&config, "max_bitrate").and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0))
    } else {
        None
    };
    let dpi = if cfg!(target_os = "windows") && keys.iter().any(|k| k == "dpi") { crate::dpi_scale::read_range(scale_display().await).await.ok() } else { None };
    let changed = {
        let mut held = NATIVE_SLIDERS.lock().unwrap_or_else(|p| p.into_inner());
        let changed = *held != (bitrate, dpi.clone());
        *held = (bitrate, dpi);
        changed
    };
    if changed {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || rebuild_tray_menu(&handle));
    }
}

/// The native menu has stepped sliders: read them in the background when the menu is the native one (it is rebuilt when they are read).
pub(super) fn refresh_native_sliders_if_native<R: Runtime + 'static>(app: &AppHandle<R>) {
    if !use_native_menu() || native_slider_keys().is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(refresh_native_sliders(app));
}

/// A stepped item of the native menu was chosen: set the value, wait for the system to show it, read the ticks again. Returns whether `id` was one of them.
pub(super) fn native_slider_event<R: Runtime + 'static>(app: &AppHandle<R>, id: &str) -> bool {
    let Some((key, value)) = parse_native_slider(id) else { return false };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match write_slider(key, &json!(value)).await {
            Ok((settle, _)) => {
                info!("tray: native menu set {} = {}", key, value);
                tokio::time::sleep(settle).await;
            }
            Err(e) => warn!("tray: native menu could not set {}: {}", key, e),
        }
        refresh_native_sliders(app).await;
    });
    true
}

/// The quick panel was edited in the menu (`tray_popup_layout`): the layout is stored and the menu gets the state with it.
pub(super) async fn set_layout(app: &AppHandle, tiles: Option<Vec<String>>, sliders: Option<Vec<String>>) -> Result<(), String> {
    if !ctx_popup::tray_menu_open() {
        return Err("菜单已经关闭".to_string());
    }
    tray_prefs::save_layout(tiles, sliders)?;
    ctx_popup::update_tray_state(app, build_state(app).await);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIONS: &str = include_str!("actions.rs");
    const MENU: &str = include_str!("menu.rs");

    #[test]
    fn the_native_menu_is_for_a_window_that_cannot_show() {
        use ctx_popup::Health::{Disabled, Failed, Ready, Starting};
        // the window is up: the Material menu
        assert!(!wants_native(false, 0, Ready));
        // the window is being made (a tray-only start, a window made again): the menu waits for it, the native menu does not take the icon
        assert!(!wants_native(false, 0, Starting));
        // it cannot show: switched off, or it could not be made
        assert!(wants_native(false, 0, Failed));
        assert!(wants_native(false, 0, Disabled));
        // the person\'s own choice is always kept
        assert!(wants_native(true, 0, Ready) && wants_native(true, 0, Starting));
        // one failed open is not the end of the menu window, two in a row are
        assert!(!wants_native(false, FAILURES_BEFORE_NATIVE - 1, Ready));
        assert!(wants_native(false, FAILURES_BEFORE_NATIVE, Ready));
    }

    #[test]
    fn a_right_click_opens_one_menu_never_two() {
        // the icon carries the native menu: the system shows it, the window stays shut (even when the window is ready and the native menu is on its way out)
        assert_eq!(right_click_plan(true, true), RightClick::System);
        assert_eq!(right_click_plan(false, true), RightClick::System);
        // no native menu on the icon and the window is wanted: the window
        assert_eq!(right_click_plan(false, false), RightClick::Window);
        // the native menu is wanted and not attached yet: nothing, rather than a window the person turned off
        assert_eq!(right_click_plan(true, false), RightClick::Nothing);
    }

    #[test]
    fn a_menu_the_person_closed_before_it_was_drawn_is_not_a_failure() {
        let source = include_str!("popup_input.rs");
        // open_tray says Ok(false) for a menu that ended before it was shown (a second click, the focus went away); only an error counts toward the native fallback
        assert!(source.contains("Ok(true) => FAILURES.store(0, Ordering::SeqCst),"));
        assert!(source.contains("Ok(false) => {}"));
        let ctx = include_str!("../ctx_popup.rs");
        let open_tray = &ctx[ctx.find("pub async fn open_tray").unwrap()..ctx.find("pub fn pick_tray").unwrap()];
        assert!(open_tray.contains("Ok(Err(_)) => return Ok(false),"));
        assert!(!open_tray.contains("the menu was closed before it was shown"), "no error text for it any more");
    }

    #[test]
    fn the_menu_window_is_made_at_a_tray_only_start_too() {
        let app = include_str!("../app.rs");
        let at = app.find("if agent_only {\n        ctx_popup::schedule_create_after").expect("agent_only makes the menu window");
        assert!(app[at..].contains("std::time::Duration::ZERO"));
        assert!(!app.contains("if !agent_only {\n        ctx_popup::schedule_create"), "the old rule that left a tray-only start without the window");
        // a lost window is made again, and the tray does not carry the native menu while it starts
        let ctx = include_str!("../ctx_popup.rs");
        assert!(ctx.contains("WindowEvent::Destroyed") && ctx.contains("schedule_create_after(window.app_handle()"));
        assert!(include_str!("../tray.rs").contains("let native = popup_input::use_native_menu();"));
    }

    #[test]
    fn the_item_ids_are_the_ids_menu_rs_builds() {
        assert_eq!(ITEM_IDS.len(), 46);
        for id in ITEM_IDS {
            assert!(MENU.contains(&format!("\"{id}\"")), "menu.rs has no '{id}'");
        }
        let mut sorted = ITEM_IDS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ITEM_IDS.len(), "no id twice");
    }

    #[test]
    fn the_bitrate_ceiling_must_be_the_one_the_host_says_is_in_force() {
        assert!(check_ceiling_in_force(&json!({ "status": "true", "max_bitrate_in_force": 50000, "sessions_updated": 1 }), 50000).is_ok());
        assert!(check_ceiling_in_force(&json!({ "status": "true", "max_bitrate_in_force": 0 }), 0).is_ok());
        assert_eq!(check_ceiling_in_force(&json!({ "status": "true", "max_bitrate_in_force": 20000 }), 50000).unwrap_err(), "主机实际生效的码率上限是 20000 Kbps");
        assert!(check_ceiling_in_force(&json!({ "status": "true" }), 50000).is_ok(), "an older host that says nothing is not doubted");
        // the one writer is told apart from the answer by the source text: the host's own words reach the person through post_sunshine_config_reporting
        assert!(include_str!("popup_input.rs").contains("post_sunshine_config_reporting(&config).await?"));
    }

    #[test]
    fn the_native_menu_has_stepped_stand_ins_for_the_sliders() {
        assert_eq!(parse_native_slider("slider_bitrate_0"), Some(("bitrate", 0)));
        assert_eq!(parse_native_slider("slider_bitrate_20"), Some(("bitrate", 20_000)));
        assert_eq!(parse_native_slider("slider_dpi_150"), Some(("dpi", 150)));
        // 350 is a step Windows has (a large display may take it); 275 and 130 are not steps at all
        assert_eq!(parse_native_slider("slider_dpi_350"), Some(("dpi", 350)));
        for bad in ["slider_bitrate_7", "slider_dpi_130", "slider_dpi_275", "slider_dpi_600", "slider_refresh_60", "slider_bitrate_", "bitrate_10", "open_main_panel"] {
            assert_eq!(parse_native_slider(bad), None, "{bad}");
        }
        // every step of both submenus parses back to what it was built from, so a tick and its event cannot drift apart
        for m in NATIVE_BITRATE_MBPS {
            assert_eq!(parse_native_slider(&format!("slider_bitrate_{m}")), Some(("bitrate", m * 1000)));
        }
        for d in NATIVE_DPI_STEPS {
            assert_eq!(parse_native_slider(&format!("slider_dpi_{d}")), Some(("dpi", u64::from(d))));
        }
        assert!(!NATIVE_DPI_STEPS.contains(&275), "Windows has no 275 % step");
        // the one handler passes them on, and the builder makes them from the same lists
        assert!(ACTIONS.contains("popup_input::native_slider_event(app, menu_id)"));
        assert!(MENU.contains("NATIVE_BITRATE_MBPS") && MENU.contains("slider_bitrate_") && MENU.contains("slider_dpi_"));
        // the 缩放 submenu has only the steps the display allows now, read with the ticks
        assert!(MENU.contains("range.options") && !MENU.contains("NATIVE_DPI_STEPS"));
        // the slider written by the menus and by the native steps is one function
        assert!(include_str!("popup_input.rs").contains("write_slider(key, value).await?"));
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
        assert_eq!(short_mode(1440, 120), "1440p@120");
        assert_eq!(short_mode(2160, 60), "4K@60");
        assert_eq!(short_mode(1080, 0), "1080p");
    }

    #[test]
    fn the_host_word_is_one_the_menus_know() {
        assert_eq!([host_word("streaming"), host_word("paused"), host_word("pairing"), host_word("idle"), host_word("notification"), host_word("")], ["streaming", "paused", "pairing", "idle", "idle", "idle"]);
    }
}
