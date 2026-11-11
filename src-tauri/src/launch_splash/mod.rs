//! The launch splash (启动画面): while a game starts, a click-through window on the display the stream captures shows its name and stage until the game's
//! window is up. The host decides the state (docs/launch-splash.md of the host repository) and publishes it in the tray state; this side owns the screen,
//! because Core may run as a service in session 0 and cannot see the user's windows.
//!
//! One worker thread follows the host's states and the clock (plan.rs decides, probe.rs finds the game's window). The window is made once, hidden, the first time
//! a host that offers the splash is seen, so the first launch does not wait for a WebView2 to start; it is shown with SetWindowPos(SWP_NOACTIVATE | SWP_SHOWWINDOW)
//! and never takes focus [ms-setwindowpos-20261111], it is transparent to the mouse and touch (set_ignore_cursor_events) and it is not in the taskbar.

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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const CAPABILITY: &str = "launch-splash-v1";
const WINDOW_LABEL: &str = "launch_splash";
const FRAME_EVENT: &str = "launch-splash:frame";
const LEAVE_EVENT: &str = "launch-splash:leave";
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

#[derive(Serialize, Clone)]
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
    // the window is made before the first launch asks for it
    ensure_window(&app);
    loop {
        let timeout = if plan.watching().is_some() { TICK } else { IDLE };
        let cmds = match rx.recv_timeout(timeout) {
            Ok(Msg::State(state)) => {
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
            apply(&app, &mut world, cmd);
        }
    }
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
        Cmd::Show(frame) => {
            SHOWN_GENERATION.fetch_add(1, Ordering::AcqRel);
            let Some(window) = ensure_window(app) else { return };
            let rect = display_rect(app, &frame.display);
            if let Some((_, _, width, height)) = rect {
                world.display_area = width as i64 * height as i64;
            }
            publish(app, payload(world, &frame));
            place_and_show(app, &window, rect);
        }
        Cmd::Update(frame) => {
            publish(app, payload(world, &frame));
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

fn publish<R: Runtime>(app: &AppHandle<R>, frame: FramePayload) {
    *LAST.lock().unwrap() = Some(frame.clone());
    let _ = app.emit_to(WINDOW_LABEL, FRAME_EVENT, frame);
}

/// The window of the splash: transparent, always on top, not focusable, click-through, hidden until a launch asks for it.
fn ensure_window<R: Runtime + 'static>(app: &AppHandle<R>) -> Option<tauri::WebviewWindow<R>> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return Some(window);
    }
    match tauri::WebviewWindowBuilder::new(app, WINDOW_LABEL, tauri::WebviewUrl::App("launch-splash/index.html".into()))
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
        .build()
    {
        Ok(window) => {
            let _ = window.set_ignore_cursor_events(true);
            crate::windows::disable_context_menu(&window);
            #[cfg(target_os = "windows")]
            crate::windows::configure_webview_security(&window);
            Some(window)
        }
        Err(error) => {
            log::warn!("launch splash: its window could not be made: {error}");
            None
        }
    }
}

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
fn place_and_show<R: Runtime + 'static>(app: &AppHandle<R>, window: &tauri::WebviewWindow<R>, rect: Option<(i32, i32, u32, u32)>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos};
    let Ok(hwnd) = window.hwnd() else { return };
    let address = hwnd.0 as usize;
    let _ = app.run_on_main_thread(move || {
        let hwnd = HWND(address as _);
        let (x, y, width, height, flags) = match rect {
            Some((x, y, w, h)) => (x, y, w as i32, h as i32, SWP_NOACTIVATE | SWP_SHOWWINDOW),
            None => (0, 0, 0, 0, SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE),
        };
        // topmost, on the display, shown, and the game keeps its focus
        if let Err(error) = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, flags) } {
            log::warn!("launch splash: it could not be shown: {error}");
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn place_and_show<R: Runtime + 'static>(_app: &AppHandle<R>, window: &tauri::WebviewWindow<R>, _rect: Option<(i32, i32, u32, u32)>) {
    let _ = window.show();
}

fn hide<R: Runtime + 'static>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let _ = window.hide();
    }
}
