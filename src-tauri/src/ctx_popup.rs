//! The window of the context menu.
//!
//! The Material menu of the panel (foundation-ui MContextMenu) is drawn inside the page, so near the edge of the main window it was cut or had to turn inward. A native menu is not bound to its window:
//! it opens at the pointer and is only kept inside the monitor's work area. This module gives the panel the same: one borderless, transparent window, created hidden once the main window has painted,
//! that shows the same Material menu (the page `ctxmenu/index.html` draws it with the same component and tokens, in the theme of the window the menu belongs to).
//!
//! Flow of one menu (every step in this file; the page side is src/renderer/composables/menuPopup.js and foundation-ui MMenuPopup):
//!   1. the page that owns the menu calls `ctx_popup_open` (the menu as data, the theme, where it was opened);
//!   2. the monitor under the pointer is found; the window is moved there at the size of its work area, cloaked (DWM) so that it renders but nobody sees it, and the menu goes to the window's page;
//!   3. the page draws the menu, measures it and calls `ctx_popup_ready` with its size; the window is sized to the menu (plus the room for its shadow), placed at the pointer by `menu_place`
//!      (flipped left/up against the WORK AREA, not against the main window), uncloaked and takes the focus (the keys go to the menu);
//!   4. the menu ends: `ctx_popup_pick` (an item, as its path), `ctx_popup_cancel` (Escape, a press outside), the focus left the window, or the owner moved, was minimised or hidden. The window hides,
//!      the owner window gets the focus back (unless the focus went to another program), and the owner is told the path of the choice (`ctx-popup:result`): its actions run in the owner.
//!
//! Without a window (it could not be created, the page did not answer, `SUNSHINE_GUI_CTX_POPUP=off`) `ctx_popup_open` fails and the page draws the menu inside itself, as before.
//!
//! The window is made at every start, with the main window or without it (a tray-only start has no other WebView), and made again when it is lost (destroyed, or its page stopped answering): the tray's
//! native menu is for the time the window cannot show (`health()`), not for a start that has no main window yet.

use crate::menu_place::{Edge, MonitorInfo, Rect, client_to_screen, css_to_physical, pick_monitor, place_beside_icon, place_surface, room_css, sane_scale, taskbar_edge};
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window, WindowEvent};

pub const POPUP_LABEL: &str = "ctxmenu";
/// The room around the menu surface for its elevation, in CSS px (foundation-ui POPUP_PAD).
const PAD_CSS: f64 = 16.0;
/// Room kept between the menu and the edge of the work area, in CSS px (the same 8 px as the in-page placement).
const MARGIN_CSS: f64 = 8.0;
/// Room kept between the tray menu and the taskbar and the work area's edge, in CSS px (a Windows 11 flyout sits 12 px from the taskbar).
const TRAY_MARGIN_CSS: f64 = 12.0;
/// Width of the window while the page measures the menu: the widest Material menu is 280 px, plus the room for the shadow.
const MEASURE_WIDTH_CSS: f64 = 360.0;
/// How long the page has to draw and measure a menu.
const READY_TIMEOUT: Duration = Duration::from_millis(900);
/// A window that was shown and did not take the focus within this time is given up (the focus rules of the system, or a window that never rendered).
const FOCUS_GIVE_UP: Duration = Duration::from_millis(1500);

static LOADED: AtomicBool = AtomicBool::new(false);
/// The window could not be made (or made again): the tray falls back to its native menu until it can.
static FAILED: AtomicBool = AtomicBool::new(false);
/// The application is going down: a window that is destroyed then is not made again.
static EXITING: AtomicBool = AtomicBool::new(false);
/// Woken when the page of the window has loaded (a menu that was asked for while the window starts waits for it).
static LOADED_NOTIFY: tokio::sync::Notify = tokio::sync::Notify::const_new();
/// The self test runs: the window stays cloaked and takes no focus (a measurement instance shows nothing on the desk of whoever uses the machine).
static SELFTEST: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<Open>> = Mutex::new(None);
/// A background loop that keeps making the window after the attempts of the start are used up is running.
static RECOVERING: AtomicBool = AtomicBool::new(false);
/// One attempt loop at a time: two (the start and the first view of the main window both ask) would race to make the same window.
static CREATING: AtomicBool = AtomicBool::new(false);

/// A menu opened from the tray icon: where the icon is and which side of the monitor the taskbar is on. The window is placed beside the icon, away from the taskbar, on every `ready`.
#[derive(Clone, Copy)]
struct TrayAnchor {
    icon: Rect,
    edge: Edge,
}

struct Open {
    token: u64,
    /// Some for the tray's menu: no owner window (the label is empty); the choice goes to the tray's own handler.
    tray: Option<TrayAnchor>,
    owner: String,
    anchor: (i32, i32),
    monitor: MonitorInfo,
    ready: Option<tokio::sync::oneshot::Sender<()>>,
    shown: bool,
    /// Where the menu surface is (physical px): another level of the same menu keeps its corner there, so the menu does not jump when a submenu replaces its items.
    surface: Option<Rect>,
    /// The window has had the focus: from then on losing it ends the menu.
    armed: bool,
    shown_at: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reason {
    Pick,
    Cancel,
    FocusLost,
    OwnerGone,
    Replaced,
    Failed,
    SelfTest,
}

impl Reason {
    /// The owner window gets the focus back when the person ended the menu inside it; not when the focus went to another program or the owner is gone.
    fn gives_focus_back(self, popup_was_focused: bool) -> bool {
        match self {
            Reason::Pick => true,
            // a menu that failed after the window was shown (and activated, cloaked) must not leave the panel without the foreground
            Reason::Cancel | Reason::Failed => popup_was_focused,
            _ => false,
        }
    }
}

fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SUNSHINE_GUI_CTX_POPUP").map(|v| !v.eq_ignore_ascii_case("off") && v != "0").unwrap_or(true))
}

fn state() -> std::sync::MutexGuard<'static, Option<Open>> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---- the window ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// The state of the window, as the tray decides which menu the icon opens (tray/popup_input.rs `wants_native`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    /// switched off (`SUNSHINE_GUI_CTX_POPUP=off`): menus stay where they were
    Disabled,
    /// being made, or its page is loading: a menu asked for now waits for it
    Starting,
    Ready,
    /// it could not be made, or was lost and could not be made again, after every attempt of `schedule_create_after`
    Failed,
}

pub fn health() -> Health {
    if !enabled() {
        Health::Disabled
    } else if LOADED.load(Ordering::SeqCst) {
        Health::Ready
    } else if FAILED.load(Ordering::SeqCst) {
        Health::Failed
    } else {
        Health::Starting
    }
}

/// The window did not load in time: the tray's native menu takes the icon until it does (`ctx_popup_loaded` clears this).
pub fn mark_failed() {
    FAILED.store(true, Ordering::SeqCst);
}

/// The application is exiting: the window is not made again.
pub fn note_exiting() {
    EXITING.store(true, Ordering::SeqCst);
}

/// Creates the hidden menu window once the main window is up (the first right-click then has no creation to wait for). Safe to call again.
pub fn schedule_create(app: &AppHandle) {
    // after the first paint and the first requests of the panel: the menu window is never in their way
    schedule_create_after(app, Duration::from_millis(1500));
}

/// Creates the hidden menu window after `delay`. A start without a main window (the tray alone) makes it at once: nothing else is painting, and the tray's menu is its only surface.
pub fn schedule_create_after(app: &AppHandle, delay: Duration) {
    if !enabled() {
        info!("[ctxpop] disabled (SUNSHINE_GUI_CTX_POPUP=off): menus stay in the page");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        create_with_retry(&app).await;
    });
}

/// Makes the window, trying again while WebView2 refuses it with the invalid-state code (webview_env.rs: another browser process on the user data folder runs with other options, or the one of
/// the panel before this one is still shutting down). Every failure is logged with the stage; when the attempts are used up the tray's native menu takes the icon (`Health::Failed`) and a slow
/// loop keeps trying, so that the Material menu is back as soon as WebView2 lets the window be made.
async fn create_with_retry(app: &AppHandle) {
    if CREATING.swap(true, Ordering::SeqCst) {
        return;
    }
    create_attempts(app).await;
    CREATING.store(false, Ordering::SeqCst);
}

async fn create_attempts(app: &AppHandle) {
    let started = Instant::now();
    let mut attempt = 0;
    loop {
        let shared = crate::webview_env::is_shared();
        match ensure_created(app) {
            Ok(()) => {
                crate::webview_env::note_success("tray-menu-window", attempt, shared, started);
                return;
            }
            Err(error) => {
                if shared {
                    // an environment that belongs to a browser process that is gone must not be asked again
                    crate::webview_env::forget();
                }
                match crate::webview_env::note_failure("tray-menu-window", attempt, shared, &error) {
                    Some(wait) if !EXITING.load(Ordering::SeqCst) => {
                        attempt += 1;
                        tokio::time::sleep(wait).await;
                    }
                    _ => break,
                }
            }
        }
    }
    warn!("[ctxpop] the menu window could not be created: the native menu takes the icon until it can be");
    FAILED.store(true, Ordering::SeqCst);
    crate::tray::refresh_menu_mode(app);
    recover_slowly(app);
}

/// After the attempts of the start failed: one more try every 20 s, up to 30 times (10 minutes), until a window is there.
fn recover_slowly(app: &AppHandle) {
    if RECOVERING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for round in 1..=30 {
            tokio::time::sleep(Duration::from_secs(20)).await;
            if EXITING.load(Ordering::SeqCst) || app.get_webview_window(POPUP_LABEL).is_some() {
                break;
            }
            match ensure_created(&app) {
                Ok(()) => {
                    info!("[webview-env] tray-menu-window: made by the slow retry (round {round})");
                    break;
                }
                Err(error) => {
                    if crate::webview_env::is_shared() {
                        crate::webview_env::forget();
                    }
                    debug!("[webview-env] tray-menu-window: slow retry {round} failed: {error}");
                }
            }
        }
        RECOVERING.store(false, Ordering::SeqCst);
    });
}

/// Waits for the page of the window to load. False when it did not within `limit` (or the window is off or failed).
pub async fn wait_loaded(limit: Duration) -> bool {
    let started = Instant::now();
    loop {
        let woken = LOADED_NOTIFY.notified();
        tokio::pin!(woken);
        woken.as_mut().enable();
        match health() {
            Health::Ready => return true,
            Health::Disabled | Health::Failed => return false,
            Health::Starting => {}
        }
        let left = limit.saturating_sub(started.elapsed());
        if left.is_zero() || tokio::time::timeout(left, woken).await.is_err() {
            return health() == Health::Ready;
        }
    }
}

/// The window is lost (its page stopped answering, or it is gone): it is destroyed if it is still there and made again, once. The tray waits for the new one (`Health::Starting`) instead of dropping to its
/// native menu; if the new one cannot be made either, `FAILED` is set and the native menu takes the icon.
fn heal(app: &AppHandle) {
    LOADED.store(false, Ordering::SeqCst);
    if EXITING.load(Ordering::SeqCst) {
        return;
    }
    match app.get_webview_window(POPUP_LABEL) {
        // `Destroyed` (handle_window_event) makes the next one
        Some(window) => {
            if let Err(error) = window.destroy() {
                warn!("[ctxpop] the menu window could not be destroyed: {error}");
                FAILED.store(true, Ordering::SeqCst);
                crate::tray::refresh_menu_mode(app);
            }
        }
        None => schedule_create_after(app, Duration::from_millis(100)),
    }
}

fn ensure_created(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(POPUP_LABEL).is_some() {
        return Ok(());
    }
    let started = Instant::now();
    LOADED.store(false, Ordering::SeqCst);
    let window = crate::webview_env::adopt(tauri::WebviewWindowBuilder::new(app, POPUP_LABEL, tauri::WebviewUrl::App("ctxmenu/index.html".into())))
        .title("")
        .inner_size(MEASURE_WIDTH_CSS, 400.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| e.to_string())?;
    crate::webview_env::capture(&window);
    crate::windows::disable_context_menu(&window);
    #[cfg(target_os = "windows")]
    crate::windows::configure_webview_security(&window);
    info!("[ctxpop] menu window created hidden in {} ms", started.elapsed().as_millis());
    Ok(())
}

/// The page of the menu window has installed its listeners: menus can be sent to it.
#[tauri::command]
pub fn ctx_popup_loaded(app: AppHandle) {
    FAILED.store(false, Ordering::SeqCst);
    LOADED.store(true, Ordering::SeqCst);
    LOADED_NOTIFY.notify_waiters();
    info!("[ctxpop] menu page ready");
    // the tray's own menu window is usable now: the native menu steps aside (unless the person asked for it), and a window that is loaded again has earned the failures it was blamed for
    crate::tray::menu_window_loaded(&app);
}

/// The window can show menus now: it exists and its page has loaded.
pub fn available() -> bool {
    health() == Health::Ready
}

/// The app handle of the running application, kept for the tray icon's callbacks (they are generic over the runtime; the menu window is not).
static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();
pub fn remember_app(app: &AppHandle) {
    let _ = APP.set(app.clone());
}
pub fn app() -> Option<AppHandle> {
    APP.get().cloned()
}

/// The theme (colours, light or dark) of the main window as it was last seen: the tray's menu is opened without a main window to ask. Kept in memory and in a small file, so that a start that begins
/// in the tray has the person's colours from the first menu.
static THEME: Mutex<Value> = Mutex::new(Value::Null);
fn theme_file() -> Option<std::path::PathBuf> {
    crate::test_isolation::config_dir().map(|dir| dir.join("Sunshine GUI").join("tray-theme.json"))
}
fn remember_theme(theme: &Value) {
    if theme.is_null() {
        return;
    }
    let mut guard = THEME.lock().unwrap_or_else(|p| p.into_inner());
    if *guard != *theme {
        *guard = theme.clone();
    }
}
fn current_theme() -> Value {
    let mut guard = THEME.lock().unwrap_or_else(|p| p.into_inner());
    if guard.is_null() {
        if let Some(text) = theme_file().and_then(|f| std::fs::read_to_string(f).ok()) {
            *guard = serde_json::from_str(&text).unwrap_or(Value::Null);
        }
    }
    guard.clone()
}
/// The main window reports its theme whenever the appearance changes.
#[tauri::command]
pub fn tray_popup_theme_set(theme: Value) {
    if theme.is_null() {
        return;
    }
    remember_theme(&theme);
    if let (Some(file), Ok(text)) = (theme_file(), serde_json::to_string(&theme)) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(file, text);
    }
}

// ---- opening ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRequest {
    pub token: u64,
    pub items: Value,
    #[serde(default)]
    pub back: Option<String>,
    #[serde(default)]
    pub theme: Value,
    /// The menu was opened with the mouse: the pointer's screen position is the anchor.
    #[serde(default)]
    pub pointer: bool,
    /// Where the page opened it, CSS px of the page (the Menu key has no pointer).
    #[serde(default)]
    pub client_x: f64,
    #[serde(default)]
    pub client_y: f64,
    /// Device pixels per CSS px of the page.
    #[serde(default)]
    pub ratio: f64,
}

fn monitors_of(owner: &WebviewWindow) -> Vec<MonitorInfo> {
    owner
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let w = m.work_area();
            MonitorInfo { work: Rect::new(w.position.x, w.position.y, w.size.width as i32, w.size.height as i32), scale: sane_scale(m.scale_factor()) }
        })
        .collect()
}

/// Opens the menu. Resolves when the window is shown (the menu is on screen); the choice comes later as the event `ctx-popup:result`. An error means no window was shown: the page draws the menu itself.
#[tauri::command]
pub async fn ctx_popup_open(owner: WebviewWindow, request: OpenRequest) -> Result<(), String> {
    if !enabled() {
        return Err("the menu window is off".into());
    }
    let started = Instant::now();
    let app = owner.app_handle().clone();
    let popup = app.get_webview_window(POPUP_LABEL).ok_or("the menu window does not exist")?;
    if !LOADED.load(Ordering::SeqCst) {
        return Err("the menu window has not loaded".into());
    }
    // the menu that is on is over: its owner is told
    finish(&app, None, Reason::Replaced);

    remember_theme(&request.theme);
    let ratio = if request.ratio > 0.0 { request.ratio } else { owner.scale_factor().unwrap_or(1.0) };
    let anchor = request
        .pointer
        .then(|| owner.cursor_position().ok().map(|p| (p.x.round() as i32, p.y.round() as i32)))
        .flatten()
        .or_else(|| owner.inner_position().ok().map(|o| client_to_screen((o.x, o.y), (request.client_x, request.client_y), ratio)))
        .ok_or("no position for the menu")?;
    let monitors = monitors_of(&owner);
    let monitor = pick_monitor(&monitors, anchor).map(|i| monitors[i]).ok_or("no monitor")?;
    let (_, room_h) = room_css(monitor.work, monitor.scale, MARGIN_CSS);

    let (tx, rx) = tokio::sync::oneshot::channel();
    *state() = Some(Open { token: request.token, tray: None, owner: owner.label().to_string(), anchor, monitor, ready: Some(tx), shown: false, surface: None, armed: false, shown_at: None });

    // the window waits on the monitor of the pointer, as large as the work area allows, cloaked: the page lays the menu out in a window that renders, at the scale it will have
    let measure = Rect::new(
        monitor.work.x,
        monitor.work.y,
        css_to_physical(MEASURE_WIDTH_CSS, monitor.scale).min(monitor.work.w),
        css_to_physical(room_h + 2.0 * PAD_CSS, monitor.scale).min(monitor.work.h),
    );
    #[cfg(target_os = "windows")]
    crate::windows::set_cloak(&popup, true);
    apply_rect(&popup, measure);
    let _ = popup.show();

    app.emit_to(
        POPUP_LABEL,
        "ctx-popup:open",
        json!({ "token": request.token, "items": request.items, "back": request.back, "theme": request.theme, "maxHeight": room_h }),
    )
    .map_err(|e| {
        finish(&app, None, Reason::Failed);
        e.to_string()
    })?;

    match tokio::time::timeout(READY_TIMEOUT, rx).await {
        Ok(Ok(())) => {}
        Ok(Err(_)) => return Err("the menu was closed before it was shown".into()),
        Err(_) => {
            warn!("[ctxpop] the page did not answer in {} ms: the menu stays in the page", READY_TIMEOUT.as_millis());
            finish(&app, None, Reason::Failed);
            return Err("the menu window did not answer".into());
        }
    }
    info!("[ctxpop] shown {} ms after the request (token {})", started.elapsed().as_millis(), request.token);
    watch(app, request.token);
    Ok(())
}

// ---- the tray's menu --------------------------------------------------------------------------------------------------------------------------------------------------------------------

static TRAY_TOKEN: AtomicU64 = AtomicU64::new(8_000_000_000);

/// Where the tray icon is and what the taskbar around it looks like: the icon's rectangle (physical px of the virtual desktop).
pub struct TrayTarget {
    pub icon: Rect,
}

/// Opens one of the tray's menus beside the icon. Resolves when the menu is on screen; the choice comes back through `tray_popup_act` (the page calls it) and runs through the tray's own handler.
/// Ok(true): the menu is on screen. Ok(false): it ended before it was shown, because the person closed it or a newer menu replaced it (not a failure). An error means the window could not show it: the window
/// is made again (`heal`) and the caller counts the failure.
pub async fn open_tray(app: AppHandle, variant: crate::tray_prefs::Variant, menu_state: Value, target: TrayTarget) -> Result<bool, String> {
    if !enabled() {
        return Err("the menu window is off".into());
    }
    let started = Instant::now();
    let popup = app.get_webview_window(POPUP_LABEL).ok_or("the menu window does not exist")?;
    if !LOADED.load(Ordering::SeqCst) {
        return Err("the menu window has not loaded".into());
    }
    finish(&app, None, Reason::Replaced);

    let icon = target.icon;
    let centre = (icon.x + icon.w / 2, icon.y + icon.h / 2);
    let monitors = popup.available_monitors().map_err(|e| e.to_string())?;
    let infos: Vec<MonitorInfo> = monitors
        .iter()
        .map(|m| {
            let w = m.work_area();
            MonitorInfo { work: Rect::new(w.position.x, w.position.y, w.size.width as i32, w.size.height as i32), scale: sane_scale(m.scale_factor()) }
        })
        .collect();
    let index = pick_monitor(&infos, centre).ok_or("no monitor")?;
    let monitor = infos[index];
    let bounds = Rect::new(monitors[index].position().x, monitors[index].position().y, monitors[index].size().width as i32, monitors[index].size().height as i32);
    let edge = taskbar_edge(bounds, monitor.work, icon);
    let (_, room_h) = room_css(monitor.work, monitor.scale, TRAY_MARGIN_CSS);

    let token = TRAY_TOKEN.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = tokio::sync::oneshot::channel();
    *state() = Some(Open { token, tray: Some(TrayAnchor { icon, edge }), owner: String::new(), anchor: centre, monitor, ready: Some(tx), shown: false, surface: None, armed: false, shown_at: None });

    // the window waits on the icon's monitor, as large as the work area allows and cloaked, while the page lays the card out at the scale it will have
    let measure = Rect::new(
        monitor.work.x,
        monitor.work.y,
        css_to_physical(variant.width_css() + 2.0 * PAD_CSS, monitor.scale).min(monitor.work.w),
        css_to_physical(room_h + 2.0 * PAD_CSS, monitor.scale).min(monitor.work.h),
    );
    #[cfg(target_os = "windows")]
    crate::windows::set_cloak(&popup, true);
    apply_rect(&popup, measure);
    let _ = popup.show();

    app.emit_to(
        POPUP_LABEL,
        "tray-popup:open",
        json!({ "token": token, "variant": variant.as_str(), "state": menu_state, "theme": current_theme(), "edge": edge.as_str(), "maxHeight": room_h }),
    )
    .map_err(|e| {
        finish(&app, None, Reason::Failed);
        e.to_string()
    })?;

    match tokio::time::timeout(READY_TIMEOUT, rx).await {
        Ok(Ok(())) => {}
        // the person closed it, or a newer menu took its place, before the page had drawn it: nothing failed
        Ok(Err(_)) => return Ok(false),
        Err(_) => {
            warn!("[ctxpop] the tray menu page did not answer in {} ms: the window is made again", READY_TIMEOUT.as_millis());
            finish(&app, None, Reason::Failed);
            heal(&app);
            return Err("the menu window did not answer".into());
        }
    }
    info!("[ctxpop] tray menu {} shown {} ms after the click (edge {}, token {})", variant.as_str(), started.elapsed().as_millis(), edge.as_str(), token);
    crate::mem_adaptive::note_menu_shown();
    // "open the panel" is one tap away: a main window that was let go is made again now
    crate::mem_adaptive::prewarm_main(&app, "tray menu opened");
    watch(app, token);
    Ok(true)
}

/// The tray menu on screen (if any) has this token: an item was chosen; the window closes. Returns whether the token was the open menu's.
pub fn pick_tray(app: &AppHandle, token: u64) -> bool {
    if state().as_ref().is_some_and(|o| o.token == token && o.tray.is_some()) {
        finish(app, None, Reason::Pick);
        return true;
    }
    false
}

/// Sends a new state to the tray menu that is on (its host state changed while it was open).
pub fn update_tray_state(app: &AppHandle, state_value: Value) {
    let token = state().as_ref().filter(|o| o.tray.is_some()).map(|o| o.token);
    if let Some(token) = token {
        let _ = app.emit_to(POPUP_LABEL, "tray-popup:state", json!({ "token": token, "state": state_value }));
    }
}

/// A menu of the tray is on screen.
pub fn tray_menu_open() -> bool {
    state().as_ref().is_some_and(|o| o.tray.is_some())
}

/// The page drew the menu (or drew another level of it) and measured it: size and place the window, show it.
#[tauri::command]
pub async fn ctx_popup_ready(app: AppHandle, token: u64, width: f64, height: f64) -> Result<(), String> {
    let popup = app.get_webview_window(POPUP_LABEL).ok_or("the menu window does not exist")?;
    let (anchor, monitor, first, tx, tray) = {
        let mut guard = state();
        let open = guard.as_mut().filter(|o| o.token == token).ok_or("no such menu")?;
        let first = !open.shown;
        open.shown = true;
        // the first level opens at the pointer; the next ones where the surface is
        let at = open.surface.map(|r| (r.x, r.y)).unwrap_or(open.anchor);
        (at, open.monitor, first, open.ready.take(), open.tray)
    };
    let size = (css_to_physical(width.max(1.0), monitor.scale), css_to_physical(height.max(1.0), monitor.scale));
    let placed = match tray {
        // the tray's menu: beside the icon, away from the taskbar; a page of another height keeps the edge at the taskbar
        Some(t) => place_beside_icon(t.icon, size, monitor.work, t.edge, css_to_physical(TRAY_MARGIN_CSS, monitor.scale)),
        None => place_surface(anchor, size, monitor.work, css_to_physical(MARGIN_CSS, monitor.scale)),
    };
    if let Some(open) = state().as_mut().filter(|o| o.token == token) {
        open.surface = Some(placed.rect);
    }
    let window = placed.rect.grow(css_to_physical(PAD_CSS, monitor.scale));
    debug!("[ctxpop] menu {}x{} css at {:?} -> surface {:?} window {:?} (scale {})", width, height, anchor, placed.rect, window, monitor.scale);
    apply_rect(&popup, window);
    if first && !SELFTEST.load(Ordering::SeqCst) {
        #[cfg(target_os = "windows")]
        crate::windows::set_cloak(&popup, false);
        let _ = popup.set_focus();
        if let Some(open) = state().as_mut().filter(|o| o.token == token) {
            open.shown_at = Some(Instant::now());
        }
    }
    if first && SELFTEST.load(Ordering::SeqCst) {
        if let Some(open) = state().as_mut().filter(|o| o.token == token) {
            open.armed = false;
            open.shown_at = Some(Instant::now());
        }
    }
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
    Ok(())
}

/// Position and size in physical pixels, checked: a window that moves between monitors of different scale can be resized by the system on the way, so the numbers are set again until they hold.
///
/// On Windows the position and the size go in ONE call (`SetWindowPos`). The card sits at the taskbar side of the window (`.m-tray-host` is `justify-content: flex-end`) and the window's far edge is the one
/// that moves when a page of another height comes: tao's `set_position` and `set_size` are two calls, and between them the window has its new top and its old size, which moves the card by the
/// difference for a frame (a menu that grows jumped up, one that shrank jumped down) on every page change of a menu whose pages are not all as high. One call has no frame in between.
fn apply_rect(window: &WebviewWindow, rect: Rect) {
    for _ in 0..3 {
        if !set_rect_at_once(window, rect) {
            let _ = window.set_position(PhysicalPosition::new(rect.x, rect.y));
            let _ = window.set_size(PhysicalSize::new(rect.w.max(1) as u32, rect.h.max(1) as u32));
        }
        match (window.outer_position(), window.outer_size()) {
            (Ok(p), Ok(s)) if holds(rect, (p.x, p.y), (s.width, s.height)) => return,
            _ => {}
        }
    }
}

/// Whether the window has the rectangle that was asked for (outer position and size, physical px).
fn holds(rect: Rect, position: (i32, i32), size: (u32, u32)) -> bool {
    position.0 == rect.x && position.1 == rect.y && size.0 as i32 == rect.w.max(1) && size.1 as i32 == rect.h.max(1)
}

/// Moves and sizes the window in one system call. False where there is no such call (not Windows) or it failed: the caller then moves and sizes in two.
#[cfg(target_os = "windows")]
fn set_rect_at_once(window: &WebviewWindow, rect: Rect) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetWindowPos};
    let Ok(hwnd) = window.hwnd() else { return false };
    match unsafe { SetWindowPos(HWND(hwnd.0), None, rect.x, rect.y, rect.w.max(1), rect.h.max(1), SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER) } {
        Ok(()) => true,
        Err(error) => {
            warn!("[ctxpop] SetWindowPos failed ({error}): the window is moved and sized in two calls");
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_rect_at_once(_window: &WebviewWindow, _rect: Rect) -> bool {
    false
}

// ---- closing ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// Ends the menu on screen (if any): hides the window, gives the focus back, tells the owner. Returns whether there was a menu.
fn finish(app: &AppHandle, path: Option<Vec<u32>>, reason: Reason) -> bool {
    let Some(open) = state().take() else { return false };
    if open.tray.is_some() && reason == Reason::FocusLost {
        // the press on the tray icon that took the focus is about to be released on the icon: that release must not open the menu again
        crate::tray_prefs::note_closed_by_focus_loss();
    }
    let popup = app.get_webview_window(POPUP_LABEL);
    let mut was_focused = false;
    if let Some(popup) = &popup {
        was_focused = popup.is_focused().unwrap_or(false);
        let _ = app.emit_to(POPUP_LABEL, "ctx-popup:close", json!({ "token": open.token }));
        if reason != Reason::Replaced {
            let _ = popup.hide();
            #[cfg(target_os = "windows")]
            if !SELFTEST.load(Ordering::SeqCst) {
                crate::windows::set_cloak(popup, false);
            }
        }
    }
    if !open.owner.is_empty() {
        let owner = app.get_webview_window(&open.owner);
        if reason.gives_focus_back(was_focused) {
            if let Some(owner) = &owner {
                let _ = owner.set_focus();
            }
        }
        let _ = app.emit_to(open.owner.as_str(), "ctx-popup:result", json!({ "token": open.token, "path": path, "reason": format!("{reason:?}") }));
    }
    debug!("[ctxpop] closed token {} ({reason:?}) path {path:?}", open.token);
    true
}

/// An item was chosen.
#[tauri::command]
pub async fn ctx_popup_pick(app: AppHandle, token: u64, path: Vec<u32>) {
    if state().as_ref().is_some_and(|o| o.token == token) {
        finish(&app, Some(path), Reason::Pick);
    }
}

/// The menu closed without a choice (Escape, a press outside, the page's own menu lost its focus).
#[tauri::command]
pub async fn ctx_popup_cancel(app: AppHandle, token: u64) {
    if state().as_ref().is_some_and(|o| o.token == token) {
        finish(&app, None, Reason::Cancel);
    }
}

/// While a menu is on: ends it when the owner is gone (minimised, hidden, closed), when the window lost the focus without the event being seen, or when it never got it.
fn watch(app: AppHandle, token: u64) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(120)).await;
            let (owner_label, armed, shown_at) = match state().as_ref().filter(|o| o.token == token) {
                Some(o) => (o.owner.clone(), o.armed, o.shown_at),
                None => return,
            };
            // the tray's menu has no owner window to keep an eye on
            let owner_ok = owner_label.is_empty() || app.get_webview_window(&owner_label).is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false));
            if !owner_ok {
                finish(&app, None, Reason::OwnerGone);
                return;
            }
            let Some(popup) = app.get_webview_window(POPUP_LABEL) else {
                finish(&app, None, Reason::Failed);
                return;
            };
            let focused = popup.is_focused().unwrap_or(false);
            let age = shown_at.map(|t| t.elapsed()).unwrap_or_default();
            if focused {
                if let Some(open) = state().as_mut().filter(|o| o.token == token) {
                    open.armed = true;
                }
            } else if armed && age > Duration::from_millis(250) {
                finish(&app, None, Reason::FocusLost);
                return;
            } else if !armed && age > Duration::from_millis(500) {
                // the system did not give the window the focus: ask again once, then give up
                if age < Duration::from_millis(700) {
                    let _ = popup.set_focus();
                } else if age > FOCUS_GIVE_UP {
                    finish(&app, None, Reason::FocusLost);
                    return;
                }
            }
        }
    });
}

/// The events of every window of the panel that matter to the menu. Returns true when the event is dealt with (the menu window is never closed, only hidden).
pub fn handle_window_event(window: &Window, event: &WindowEvent) -> bool {
    let label = window.label();
    if label == POPUP_LABEL {
        match event {
            WindowEvent::Focused(true) => {
                if let Some(open) = state().as_mut() {
                    open.armed = true;
                }
            }
            WindowEvent::Focused(false) => {
                let armed = state().as_ref().is_some_and(|o| o.armed);
                if armed {
                    finish(window.app_handle(), None, Reason::FocusLost);
                }
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
                return true;
            }
            WindowEvent::Destroyed => {
                LOADED.store(false, Ordering::SeqCst);
                finish(window.app_handle(), None, Reason::Failed);
                // a lost window is made again (once per loss: a window that cannot be made sets FAILED and the native menu takes the icon); not while the application goes down
                if !EXITING.load(Ordering::SeqCst) {
                    warn!("[ctxpop] the menu window was destroyed: it is made again");
                    schedule_create_after(window.app_handle(), Duration::from_millis(100));
                }
            }
            _ => {}
        }
        return false;
    }
    // the window the menu belongs to moved, changed size or scale, or closed: the menu would point at nothing
    let is_owner = state().as_ref().is_some_and(|o| o.owner == label);
    if is_owner && matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } | WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed) {
        finish(window.app_handle(), None, Reason::OwnerGone);
    }
    false
}

// ---- measurement -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// Measurement instances only (SUNSHINE_GUI_TEST_OFFSCREEN): opens a menu of 12 items and a submenu `count` times at a point inside the (off-screen) main window and reports how long each took from the
/// request to the shown window. The renderer-to-visible time with the page's own IPC is in `window.__ctxPopupTimings` (src/renderer/composables/menuPopup.js).
#[tauri::command]
pub async fn ctx_popup_selftest(owner: WebviewWindow, count: u32) -> Result<Value, String> {
    if !crate::windows::test_offscreen() {
        return Err("only a measurement instance runs the self test".into());
    }
    let app = owner.app_handle().clone();
    let mut items: Vec<Value> = (0..10).map(|i| json!({ "id": format!("item-{i}"), "label": format!("Menu item {i}"), "icon": "content_copy" })).collect();
    items.push(json!({ "id": "sub", "label": "Submenu", "children": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] }));
    items.push(json!({ "id": "danger", "label": "Remove", "danger": true }));
    let mut times = Vec::new();
    SELFTEST.store(true, Ordering::SeqCst);
    let result = async {
    for i in 0..count.min(200) {
        let token = 9_000_000 + i as u64;
        let t = Instant::now();
        let request = OpenRequest { token, items: Value::Array(items.clone()), back: Some("Back".into()), theme: Value::Null, pointer: false, client_x: 120.0, client_y: 120.0, ratio: 0.0 };
        ctx_popup_open(owner.clone(), request).await?;
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        finish(&app, None, Reason::SelfTest);
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Ok::<(), String>(())
    }
    .await;
    SELFTEST.store(false, Ordering::SeqCst);
    result?;
    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| sorted.get(((sorted.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
    Ok(json!({ "count": times.len(), "p50_ms": at(0.5), "p95_ms": at(0.95), "min_ms": at(0.0), "max_ms": at(1.0), "times_ms": times }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_holds_the_rectangle_when_position_and_size_both_match() {
        let rect = Rect::new(100, 200, 360, 480);
        assert!(holds(rect, (100, 200), (360, 480)));
        assert!(!holds(rect, (100, 201), (360, 480)), "a window that moved");
        assert!(!holds(rect, (100, 200), (360, 481)), "a window the system resized on the way between monitors");
        assert!(holds(Rect::new(0, 0, 0, 0), (0, 0), (1, 1)), "a zero size is asked for as 1 px, as it is set");
    }

    #[test]
    fn position_and_size_go_in_one_call_on_windows() {
        let src = include_str!("ctx_popup.rs");
        let apply = &src[src.find("fn apply_rect").unwrap()..src.find("fn holds").unwrap()];
        assert!(apply.contains("set_rect_at_once(window, rect)"), "the first try is one SetWindowPos");
        assert!(src.contains("SetWindowPos(HWND(hwnd.0), None, rect.x, rect.y, rect.w.max(1), rect.h.max(1)"));
        // the two-call path is only the fallback
        assert!(apply.contains("if !set_rect_at_once"));
    }
}
