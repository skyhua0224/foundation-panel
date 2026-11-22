//! The launch splash (启动画面): while a game starts, a click-through window on the display the stream captures shows its name and stage until the game's
//! window is up. The host decides the state (docs/launch-splash.md of the host repository) and publishes it in the tray state; this side owns the screen,
//! because Core may run as a service in session 0 and cannot see the user's windows.
//!
//! One worker thread follows the host's states and the clock (plan.rs decides, probe.rs finds the game's window). The window is made once, hidden, the first time
//! a host that offers the splash is seen, so the first launch does not wait for a WebView2 to start; it is shown with SetWindowPos(SWP_NOACTIVATE | SWP_SHOWWINDOW)
//! and never takes focus [ms-setwindowpos], it is transparent to the mouse and touch (set_ignore_cursor_events) and it is not in the taskbar.
//!
//! The splash comes before the game. The host announces a launch the moment it accepts the request (display_ready false) and the window is readied while the
//! display and encoder setup runs: made if it is not there, the page loaded, the hero read and decoded. When the display is set up (display_ready true) the
//! window goes onto it, and once the page has painted its first frame there (two animation frames after the window is shown) the host is told
//! (`launch_splash_shown`, sunshine.rs) and starts the game. A window that merely exists is not an answer.

mod art;
mod plan;
mod probe;

use crate::sunshine;
use plan::{Cmd, Frame, Plan};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const CAPABILITY: &str = "launch-splash-v1";
const WINDOW_LABEL: &str = "launch_splash";
const FRAME_EVENT: &str = "launch-splash:frame";
const LEAVE_EVENT: &str = "launch-splash:leave";
/// The window is on the display: the page paints a frame and reports it (launch_splash_painted).
const PRESENT_EVENT: &str = "launch-splash:present";
/// How often the search for the game's window runs while a launch is followed.
const TICK: Duration = Duration::from_millis(120);
/// Nothing to follow: wake rarely.
const IDLE: Duration = Duration::from_secs(5);
/// The page fades out in this time (css: 180 ms); the window is hidden after it.
const FADE_MS: u64 = 200;

enum Msg {
    State(sunshine::TrayLaunchState),
    Disconnected,
}

static SENDER: OnceLock<Mutex<mpsc::Sender<Msg>>> = OnceLock::new();
/// Counts the shows, so a hide that was waiting out the fade does not hide a splash that came back meanwhile.
static SHOWN_GENERATION: AtomicU64 = AtomicU64::new(0);

/// What the page was last given: a page that loads after the first event asks for it (launch_splash_frame).
static LAST: Mutex<Option<FramePayload>> = Mutex::new(None);

/// The clock of one launch, from the Panel's side: when the announcement arrived, when the window was ready, when the display became ready.
struct Timing {
    id: String,
    kind: String,
    announced: Instant,
    /// the window was there before the announcement (a warm Panel) or had to be made
    window_was_there: bool,
    prepared_ms: Option<i64>,
    ready: Option<Instant>,
    answered: bool,
}

static TIMING: Mutex<Option<Timing>> = Mutex::new(None);

fn millis(from: Instant) -> i64 {
    from.elapsed().as_millis() as i64
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FramePayload {
    id: String,
    app_name: String,
    art: String,
    stage: String,
    notice: String,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// The tray state arrived (every revision). Starts the worker the first time a host that offers the splash is seen.
pub fn on_tray_state<R: Runtime + 'static>(app: &AppHandle<R>, state: &sunshine::TrayState) {
    if !state.capabilities.iter().any(|capability| capability == CAPABILITY) {
        return;
    }
    send(app, Msg::State(state.launch.clone()));
}

/// Core went away: a splash that is up goes.
pub fn on_core_disconnected<R: Runtime + 'static>(app: &AppHandle<R>) {
    if SENDER.get().is_some() {
        send(app, Msg::Disconnected);
    }
}

fn send<R: Runtime + 'static>(app: &AppHandle<R>, msg: Msg) {
    let sender = SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        let worker_app = app.clone();
        if let Err(error) = std::thread::Builder::new().name("launch-splash".into()).spawn(move || run(worker_app, rx)) {
            log::warn!("launch splash: its thread did not start: {error}");
        }
        Mutex::new(tx)
    });
    let _ = sender.lock().unwrap().send(msg);
}

/// What the worker keeps between messages.
#[derive(Default)]
struct World {
    /// the windows that were up when the launch began (a launcher that leaves is found by a new large window)
    baseline: Option<HashSet<isize>>,
    /// the area of the display the splash is on, in pixels
    display_area: i64,
    /// the picture of the launch being shown: (path, data URL)
    art: Option<(String, Option<String>)>,
    /// the page has the picture of this (launch id, picture path): the events after it leave the data URL out, it is megabytes
    art_sent: Option<(String, String)>,
}

impl World {
    fn art_of(&mut self, path: &str) -> String {
        if self.art.as_ref().is_none_or(|(known, _)| known != path) {
            self.art = Some((path.to_string(), art::data_url(path)));
        }
        self.art.as_ref().and_then(|(_, url)| url.clone()).unwrap_or_default()
    }
}

fn run<R: Runtime + 'static>(app: AppHandle<R>, rx: mpsc::Receiver<Msg>) {
    let mut plan = Plan::default();
    let mut world = World::default();
    // the window is made before the first launch asks for it, unless the machine is short of memory (mem_adaptive.rs): then the host's announcement makes it, while the display is set up
    if crate::mem_adaptive::warm_splash() {
        ensure_window(&app);
        note_use();
    }
    loop {
        if plan.watching().is_some() {
            note_use();
        }
        let timeout = if plan.watching().is_some() { TICK } else { IDLE };
        let received = match measurement_announcement() {
            Some(state) => Ok(Msg::State(state)),
            None => rx.recv_timeout(timeout),
        };
        let cmds = match received {
            Ok(Msg::State(state)) => {
                note_timing(&app, &state);
                let began = plan.watching().map(|l| l.id.clone()) != Some(state.id.clone()) && state.active && state.kind != "resume";
                if began {
                    #[cfg(target_os = "windows")]
                    {
                        world.baseline = Some(probe::snapshot_windows().into_iter().map(|w| w.handle).collect());
                    }
                }
                plan.on_state(&state, now_ms())
            }
            Ok(Msg::Disconnected) => plan.on_disconnect(),
            Err(RecvTimeoutError::Timeout) => {
                let found = window_found(&plan, &world);
                plan.on_tick(found, now_ms())
            }
            Err(RecvTimeoutError::Disconnected) => return,
        };
        for cmd in cmds {
            note_use();
            apply(&app, &mut world, cmd);
        }
    }
}

/// Measurement only: with `SUNSHINE_GUI_TEST_SPLASH_FILE` set, a file at that path (its text is the launch id) is taken as the host's announcement of a launch whose display is not ready, and deleted. It is looked
/// at every time the worker wakes (at most every `IDLE`). Nothing of it exists without the variable.
fn measurement_announcement() -> Option<sunshine::TrayLaunchState> {
    let path = std::env::var_os("SUNSHINE_GUI_TEST_SPLASH_FILE")?;
    let id = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    serde_json::from_value(serde_json::json!({ "id": id, "active": true, "kind": "launch", "app_name": "Memory test", "stage": "preparing", "display_ready": false })).ok()
}

/// Measurement only (SUNSHINE_GUI_TEST_SPLASH_FILE): starts the worker at the application's start, as the first state of a host that offers the splash does in the product.
pub fn start_for_measurement<R: Runtime + 'static>(app: &AppHandle<R>) {
    if std::env::var_os("SUNSHINE_GUI_TEST_SPLASH_FILE").is_some() {
        send(app, Msg::Disconnected);
    }
}

/// When the splash was last needed: a launch followed, a command applied, the window made.
static LAST_USE: Mutex<Option<Instant>> = Mutex::new(None);

fn note_use() {
    *LAST_USE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now());
}

/// How long the window has been idle (it exists, is hidden and no launch is followed). None when there is nothing to let go.
pub fn idle_for<R: Runtime>(app: &AppHandle<R>) -> Option<Duration> {
    let window = app.get_webview_window(WINDOW_LABEL)?;
    if window.is_visible().unwrap_or(true) {
        return None;
    }
    let last = (*LAST_USE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))?;
    Some(last.elapsed())
}

/// Lets the window go (memory release, mem_adaptive.rs). The next announcement makes it again, as for a first launch.
pub fn release<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else { return false };
    log::info!("launch splash: window released for memory");
    *LAST.lock().unwrap() = None;
    window.destroy().is_ok()
}

/// Starts the clock of a launch the first time its id is seen, and notes when its display became ready.
fn note_timing<R: Runtime + 'static>(app: &AppHandle<R>, state: &sunshine::TrayLaunchState) {
    if state.id.is_empty() || !state.active {
        return;
    }
    let mut timing = TIMING.lock().unwrap();
    if timing.as_ref().is_none_or(|t| t.id != state.id) {
        *timing = Some(Timing {
            id: state.id.clone(),
            kind: state.kind.clone(),
            announced: Instant::now(),
            window_was_there: app.get_webview_window(WINDOW_LABEL).is_some(),
            prepared_ms: None,
            ready: None,
            answered: false,
        });
    }
    if let Some(t) = timing.as_mut() {
        if state.display_ready && t.ready.is_none() {
            t.ready = Some(Instant::now());
        }
    }
}

/// The page has the launch's frame loaded and its picture decoded: the window is ready. Logged with the time since the announcement.
#[tauri::command]
pub fn launch_splash_prepared(id: String) {
    let mut timing = TIMING.lock().unwrap();
    let Some(t) = timing.as_mut().filter(|t| t.id == id) else { return };
    if t.prepared_ms.is_none() {
        let ms = millis(t.announced);
        t.prepared_ms = Some(ms);
        log::info!(
            "launch splash {id}: window ready {ms} ms after the announcement ({})",
            if t.window_was_there { "window was there" } else { "window had to be made" }
        );
    }
}

/// The page painted its first frame with the window on the display. Tells the host, which starts the game on it.
#[tauri::command]
pub fn launch_splash_painted(id: String) {
    let (panel_ms, prepare_ms, kind) = {
        let mut timing = TIMING.lock().unwrap();
        let Some(t) = timing.as_mut().filter(|t| t.id == id && !t.answered) else { return };
        t.answered = true;
        let since_ready = millis(t.ready.unwrap_or(t.announced));
        (since_ready, t.prepared_ms.unwrap_or(-1), t.kind.clone())
    };
    log::info!("launch splash {id}: first frame painted {panel_ms} ms after the display was ready");
    // a resume has nobody waiting for it
    if kind != "launch" {
        return;
    }
    tauri::async_runtime::spawn(async move {
        if let Err(error) = sunshine::post_launch_splash_shown(&id, panel_ms, prepare_ms).await {
            log::debug!("launch splash {id}: the host was not told: {error}");
        }
    });
}

#[cfg(target_os = "windows")]
fn window_found(plan: &Plan, world: &World) -> bool {
    let Some(launch) = plan.watching() else { return false };
    let table = probe::snapshot_processes();
    let tree = probe::tree_of(&launch.pids, &table);
    // no process is known yet (the prepare commands are still running): nothing to guess from
    let any_alive = launch.pids.is_empty() || tree.iter().any(|pid| table.contains_key(pid));
    let before = if launch.kind == "resume" { None } else { world.baseline.as_ref() };
    probe::game_window_present(&probe::snapshot_windows(), std::process::id(), &tree, any_alive, before, world.display_area)
}

#[cfg(not(target_os = "windows"))]
fn window_found(_plan: &Plan, _world: &World) -> bool {
    false
}

/// Measurement instances only (SUNSHINE_GUI_TEST_OFFSCREEN): announces (active) or ends a launch whose display is not ready, as the host does at the start of a launch: the window is made and readied, nothing is
/// put on a screen. The log then has "window ready N ms after the announcement (window was there | window had to be made)".
#[tauri::command]
pub fn launch_splash_selftest(app: AppHandle, id: String, active: bool) -> Result<(), String> {
    if !crate::windows::test_offscreen() {
        return Err("measurement instances only".into());
    }
    let state: sunshine::TrayLaunchState = serde_json::from_value(serde_json::json!({
        "id": id, "active": active, "kind": "launch", "app_name": "Memory test", "stage": "preparing", "display_ready": false
    }))
    .map_err(|e| e.to_string())?;
    send(&app, Msg::State(state));
    Ok(())
}

/// The frame the page should draw now, or None when no splash is up. The page calls it once after it has started listening.
#[tauri::command]
pub fn launch_splash_frame() -> Option<FramePayload> {
    LAST.lock().unwrap().clone()
}

fn payload(world: &mut World, frame: &Frame) -> FramePayload {
    FramePayload {
        id: frame.id.clone(),
        app_name: frame.app_name.clone(),
        art: world.art_of(&frame.art),
        stage: frame.stage.clone(),
        notice: frame.notice.clone(),
    }
}

fn apply<R: Runtime + 'static>(app: &AppHandle<R>, world: &mut World, cmd: Cmd) {
    match cmd {
        Cmd::Prepare(frame) => {
            // the window, the page and the picture are made ready now, while the host sets the display up; nothing is put on a screen
            if ensure_window(app).is_none() {
                return;
            }
            publish(app, world, &frame);
        }
        Cmd::Show(frame) => {
            SHOWN_GENERATION.fetch_add(1, Ordering::AcqRel);
            let Some(window) = ensure_window(app) else { return };
            let rect = display_rect(app, &frame.display);
            if let Some((_, _, width, height)) = rect {
                world.display_area = width as i64 * height as i64;
            }
            publish(app, world, &frame);
            place_and_show(app, &window, rect, frame.id.clone());
        }
        Cmd::Update(frame) => {
            publish(app, world, &frame);
        }
        Cmd::Hide => {
            *LAST.lock().unwrap() = None;
            let _ = app.emit_to(WINDOW_LABEL, LEAVE_EVENT, ());
            let generation = SHOWN_GENERATION.load(Ordering::Acquire);
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(FADE_MS));
                if SHOWN_GENERATION.load(Ordering::Acquire) == generation {
                    hide(&app);
                }
            });
        }
    }
}

/// Gives the page the frame. `LAST` keeps the whole frame for a page that loads late; the event leaves the picture out when the page already has it, and
/// does not go at all when nothing changed (the show after the readying).
fn publish<R: Runtime>(app: &AppHandle<R>, world: &mut World, frame: &Frame) {
    let full = payload(world, frame);
    let mut last = LAST.lock().unwrap();
    if last.as_ref() == Some(&full) {
        return;
    }
    let art_key = (full.id.clone(), world.art.as_ref().map(|(path, _)| path.clone()).unwrap_or_default());
    let mut event = full.clone();
    if world.art_sent.as_ref() == Some(&art_key) {
        event.art = String::new();
    } else {
        world.art_sent = Some(art_key);
    }
    *last = Some(full);
    drop(last);
    let _ = app.emit_to(WINDOW_LABEL, FRAME_EVENT, event);
}

/// The window of the splash: transparent, always on top, not focusable, click-through, hidden until a launch asks for it.
fn ensure_window<R: Runtime + 'static>(app: &AppHandle<R>) -> Option<tauri::WebviewWindow<R>> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return Some(window);
    }
    // WebView2 can refuse a window for a moment (0x8007139F, webview_env.rs): tried again a few times, with the stage logged; the worker thread waits, the main thread never does
    let started = Instant::now();
    let mut attempt = 0;
    loop {
        let shared = crate::webview_env::is_shared();
        let built = crate::webview_env::adopt(tauri::WebviewWindowBuilder::new(app, WINDOW_LABEL, tauri::WebviewUrl::App("launch-splash/index.html".into())))
            .title("Launch splash")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .focusable(false)
            .visible(false)
            .inner_size(320.0, 180.0)
            .disable_drag_drop_handler()
            .build();
        match built {
            Ok(window) => {
                crate::webview_env::note_success("launch-splash-window", attempt, shared, started);
                crate::webview_env::capture(&window);
                let _ = window.set_ignore_cursor_events(true);
                crate::windows::disable_context_menu(&window);
                #[cfg(target_os = "windows")]
                crate::windows::configure_webview_security(&window);
                return Some(window);
            }
            Err(error) => {
                if shared {
                    crate::webview_env::forget();
                }
                match crate::webview_env::note_failure("launch-splash-window", attempt, shared, &error.to_string()) {
                    Some(wait) if attempt < SPLASH_ATTEMPTS => {
                        attempt += 1;
                        std::thread::sleep(wait);
                    }
                    _ => return None,
                }
            }
        }
    }
}

/// Attempts after the first one to make the splash window when WebView2 refuses it with the invalid-state code (250 + 750 + 2000 ms of waiting at most).
const SPLASH_ATTEMPTS: usize = 3;

/// x, y, width, height in physical pixels of the display the host names, else the primary one.
fn display_rect<R: Runtime>(app: &AppHandle<R>, display: &str) -> Option<(i32, i32, u32, u32)> {
    let monitors = app.available_monitors().ok()?;
    let names: Vec<String> = monitors.iter().map(|m| m.name().cloned().unwrap_or_default()).collect();
    let primary = app.primary_monitor().ok().flatten();
    let primary_index = primary.as_ref().and_then(|p| monitors.iter().position(|m| m.position() == p.position() && m.size() == p.size()));
    let monitor = &monitors[plan::pick_display(display, &names, primary_index)?];
    Some((monitor.position().x, monitor.position().y, monitor.size().width, monitor.size().height))
}

#[cfg(target_os = "windows")]
fn place_and_show<R: Runtime + 'static>(app: &AppHandle<R>, window: &tauri::WebviewWindow<R>, rect: Option<(i32, i32, u32, u32)>, id: String) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos};
    let Ok(hwnd) = window.hwnd() else { return };
    let address = hwnd.0 as usize;
    let emit_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        let hwnd = HWND(address as _);
        let (x, y, width, height, flags) = match rect {
            Some((x, y, w, h)) => (x, y, w as i32, h as i32, SWP_NOACTIVATE | SWP_SHOWWINDOW),
            None => (0, 0, 0, 0, SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE),
        };
        // topmost, on the display, shown, and the game keeps its focus
        if let Err(error) = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, flags) } {
            log::warn!("launch splash: it could not be shown: {error}");
            return;
        }
        // on the display: the page paints a frame and reports it
        let _ = emit_app.emit_to(WINDOW_LABEL, PRESENT_EVENT, id);
    });
}

#[cfg(not(target_os = "windows"))]
fn place_and_show<R: Runtime + 'static>(app: &AppHandle<R>, window: &tauri::WebviewWindow<R>, _rect: Option<(i32, i32, u32, u32)>, id: String) {
    let _ = window.show();
    let _ = app.emit_to(WINDOW_LABEL, PRESENT_EVENT, id);
}

fn hide<R: Runtime + 'static>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let _ = window.hide();
    }
}
