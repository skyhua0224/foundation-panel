//! Memory-adaptive release of the idle main window and the launch splash. On by default on every machine; `SUNSHINE_GUI_MEM_MODE=off|adaptive|low|auto` (or `{"mode": "..."}` in
//! `<config>/Sunshine GUI/memory-mode.json`) overrides the choice the controller makes.
//!
//! The main window stays hidden in the tray while memory is plentiful. At elevated or critical system pressure, the poller may release a hidden main window after saving its state.
//! The Material tray menu stays resident. It shares the WebView2 environment with the main window and the launch splash.
//!
//! * `Setting::resolve` selects Adaptive or Low by installed memory. Both tiers require a fresh system pressure sample to release the main window.
//! * The poller measures available physical memory and system commit headroom and checks the low-memory notification (sources in `mem_adaptive_core.rs`).
//! * Predictable reopen inputs prewarm the main window and restore the saved route, scroll position, tabs and drafts.
//! * `Guard` requires a qualified cold open and blocks release for six hours at a pressure level where a released reopen exceeded the limit.
//!
//! Measurement overrides: `SUNSHINE_GUI_MEM_TRAY_IDLE_SECS` and `SUNSHINE_GUI_MEM_FAKE_PRESSURE_FILE` affect the splash. `SUNSHINE_GUI_MEM_GUARD=off` bypasses the latency guard.
//! Main window release still requires measured system pressure.

use crate::mem_adaptive_core::{self as core, Guard, Mode, Pressure, Setting, Verdict};
use crate::test_isolation as dirs;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

/// How often the machine and the idle windows are looked at.
const TICK: Duration = Duration::from_secs(3);
/// A click older than this was not answered by the Material menu.
const CLICK_STALE: Duration = Duration::from_secs(10);
/// A main window made ahead of a click is kept this long to let the person open it.
const PREWARM_GRACE: Duration = Duration::from_secs(25);

// ---- the tier ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------

pub fn setting() -> Setting {
    static SETTING: OnceLock<Setting> = OnceLock::new();
    *SETTING.get_or_init(|| std::env::var("SUNSHINE_GUI_MEM_MODE").ok().map(|text| Setting::parse(&text)).or_else(setting_from_file).unwrap_or(Setting::Auto))
}

fn setting_from_file() -> Option<Setting> {
    let path = dirs::config_dir()?.join("Sunshine GUI").join("memory-mode.json");
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    value.get("mode").and_then(|mode| mode.as_str()).map(Setting::parse)
}

/// The tier of this machine, decided once: the setting, or the preset for the installed memory.
pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    *MODE.get_or_init(|| {
        let total = system::sample().map_or(0, |sample| sample.total_phys);
        let mode = setting().resolve(total);
        info!("[memadapt] tier {mode:?} (setting {:?}, {} MB installed)", setting(), total >> 20);
        mode
    })
}

static PRESSURE: AtomicU8 = AtomicU8::new(0);

fn pressure() -> Pressure {
    match PRESSURE.load(Ordering::Relaxed) {
        2 => Pressure::Critical,
        1 => Pressure::Elevated,
        _ => Pressure::None,
    }
}

fn set_pressure(pressure: Pressure) {
    PRESSURE.store(pressure as u8, Ordering::Relaxed);
}

/// The splash window is made at the start when the tier and system pressure allow it.
pub fn warm_splash() -> bool {
    core::splash_policy(mode(), pressure()).warm_splash
}

// ---- the clock ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// Starts the poller. Does nothing while the tier is off.
pub fn start(app: &AppHandle) {
    if mode() == Mode::Off {
        info!("[memadapt] off: nothing is released");
        return;
    }
    load_guard();
    if let Some(sample) = system::sample() {
        info!("[memadapt] {} MB installed, {} MB available", sample.total_phys >> 20, sample.avail_phys >> 20);
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            tick(&app);
        }
    });
}

fn tick(app: &AppHandle) {
    let measured = observe_pressure();
    let mode = mode();
    if core::may_release_hidden_main(mode, measured) {
        release_hidden_main(app, measured);
    }

    let splash_pressure = std::env::var_os("SUNSHINE_GUI_MEM_FAKE_PRESSURE_FILE")
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| core::parse_fake_pressure(&text))
        .unwrap_or(measured);
    let splash_delay = core::override_secs(std::env::var("SUNSHINE_GUI_MEM_TRAY_IDLE_SECS").ok().as_deref())
        .or(core::splash_policy(mode, splash_pressure).splash_idle_release);
    release_idle_splash(app, splash_delay);
}

fn observe_pressure() -> Pressure {
    let Some(sample) = system::sample() else { return Pressure::None };
    let before = pressure();
    let now = core::classify(&sample, before);
    if now != before {
        info!(
            "[memadapt] pressure {before:?} -> {now:?}: {} MB of {} MB physical available, commit headroom {} MB, system low-memory signal {}",
            sample.avail_phys >> 20,
            sample.total_phys >> 20,
            if sample.commit_headroom == u64::MAX { u64::MAX } else { sample.commit_headroom >> 20 },
            sample.low_signal
        );
        set_pressure(now);
    }
    now
}

fn release_hidden_main(app: &AppHandle, pressure: Pressure) {
    let Some(window) = crate::windows::main_window_sleeping(app) else { return };
    let warming = {
        let state = prewarm_state();
        state.building || state.ready_at.is_some_and(|at| at.elapsed() < PREWARM_GRACE)
    };
    if warming {
        return;
    }
    if !release_allowed(pressure) {
        debug!("[memadapt] release held back by the guard at {pressure:?} pressure");
        return;
    }
    keep_bounds(&window);
    info!("[memadapt] the main window has been hidden: released at {pressure:?} pressure");
    crate::view_state::arm_restore();
    prewarm_state().ready_at = None;
    RELEASE_PRESSURE.store(pressure as u8, Ordering::Relaxed);
    RELEASED_MAIN.store(true, Ordering::SeqCst);
    if !crate::windows::release_main_window(app) {
        warn!("[memadapt] the main window could not be released");
        RELEASED_MAIN.store(false, Ordering::SeqCst);
        crate::view_state::disarm_restore();
    }
}

fn release_idle_splash(app: &AppHandle, delay: Option<Duration>) {
    let Some(delay) = delay else { return };
    // a main window that shows a page does not need the splash let go (it is a memory release of the idle machine)
    if app.get_webview_window("main").is_some() && crate::windows::main_window_sleeping(app).is_none() {
        return;
    }
    if crate::launch_splash::idle_for(app).is_some_and(|idle| idle >= delay) {
        crate::launch_splash::release(app);
    }
}

// ---- coming back ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// The main window was let go and has not been shown again yet.
static RELEASED_MAIN: AtomicBool = AtomicBool::new(false);
/// The pressure level the last release happened at (the situation the guard judges it by).
static RELEASE_PRESSURE: AtomicU8 = AtomicU8::new(0);
#[derive(Default)]
struct Prewarm {
    building: bool,
    ready_at: Option<Instant>,
    requested: bool,
    target_url: Option<String>,
}

static PREWARM: Mutex<Prewarm> = Mutex::new(Prewarm {
    building: false, ready_at: None, requested: false, target_url: None,
});

fn prewarm_state() -> std::sync::MutexGuard<'static, Prewarm> {
    PREWARM.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Queue activation atomically with the end of a prewarm. Keep the most recent
/// explicit URL if a plain activation arrives before construction finishes.
pub fn defer_open(target_url: &mut Option<String>) -> bool {
    let mut prewarm = prewarm_state();
    if !prewarm.building {
        return false;
    }
    prewarm.requested = true;
    if target_url.is_some() {
        prewarm.target_url = target_url.take();
    }
    true
}

/// A pointer gesture, a tray menu or a shortcut can precede activation.
/// The single-instance callback can also begin construction while handling its arguments.
pub fn prewarm_main<R: Runtime>(app: &AppHandle<R>, why: &str) {
    if !RELEASED_MAIN.load(Ordering::SeqCst) || app.get_webview_window("main").is_some() || crate::windows::main_window_closing() {
        return;
    }
    {
        let mut prewarm = prewarm_state();
        if prewarm.building {
            return;
        }
        prewarm.building = true;
    }
    info!("[memadapt] {why}: the main window is made ahead of the click");
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let ready = match crate::windows::create_main_window_hidden(&app) {
            Ok(()) => {
                info!("[memadapt] the main window was made ahead in {} ms", started.elapsed().as_millis());
                true
            }
            Err(error) => {
                warn!("[memadapt] the main window could not be made ahead: {error}");
                false
            }
        };
        let (requested, target_url) = {
            let mut prewarm = prewarm_state();
            prewarm.building = false;
            prewarm.ready_at = ready.then(Instant::now);
            (std::mem::take(&mut prewarm.requested), prewarm.target_url.take())
        };
        if requested {
            crate::windows::activate_main_window(&app, target_url);
        }
    });
}

// ---- the guard ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

static GUARD: Mutex<Option<Guard>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reopen {
    /// the window was hidden and is shown again (never let go)
    Kept,
    /// the window was let go and made ahead of the click
    Prewarmed,
    /// the window was let go and the click had to make it
    Cold,
    /// the first window of the run
    First,
}

static OPEN_REQUEST: Mutex<Option<(Instant, Reopen)>> = Mutex::new(None);
/// An ask for the window older than this is a stale one (the window was never shown for it).
const OPEN_REQUEST_TTL: Duration = Duration::from_secs(30);

fn guard<T>(f: impl FnOnce(&mut Guard) -> T) -> T {
    let mut slot = GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(slot.get_or_insert_with(Guard::default))
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

fn guard_path() -> Option<std::path::PathBuf> {
    Some(dirs::config_dir()?.join("Sunshine GUI").join("memory-guard.json"))
}

fn load_guard() {
    let Some(text) = guard_path().and_then(|path| std::fs::read_to_string(path).ok()) else { return };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}')) else { return };
    let Some(list) = value.get("blocked_until").and_then(|list| list.as_array()) else { return };
    let mut blocks = [0u64; 3];
    for (slot, item) in blocks.iter_mut().zip(list) {
        *slot = item.as_u64().unwrap_or(0);
    }
    guard(|guard| *guard = Guard::with_blocks(blocks));
    if blocks.iter().any(|&until| until > now_secs()) {
        info!("[memadapt] the guard holds releases back in some situations until {blocks:?} (seconds since the epoch)");
    }
}

fn save_guard() {
    let blocks = guard(|guard| guard.blocks());
    let Some(path) = guard_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(error) = std::fs::write(path, serde_json::json!({ "blocked_until": blocks }).to_string()) {
        debug!("[memadapt] the guard could not be kept: {error}");
    }
}

fn guard_enabled() -> bool {
    !std::env::var("SUNSHINE_GUI_MEM_GUARD").is_ok_and(|value| value.eq_ignore_ascii_case("off"))
}

/// Whether the reopen-latency guard permits release at this pressure.
fn release_allowed(pressure: Pressure) -> bool {
    !guard_enabled() || guard(|guard| guard.cold_open_qualified() && guard.allows_release(pressure, now_secs()))
}

/// The person asked for the main window (a click, a hotkey, a second start): the clock of the reopen starts. Only a window that is hidden or absent counts.
pub fn note_open_request(existing_hidden: bool, exists: bool) {
    if exists && !existing_hidden {
        return;
    }
    let kind = if RELEASED_MAIN.load(Ordering::SeqCst) {
        let prewarm = prewarm_state();
        if prewarm.building || prewarm.ready_at.is_some() { Reopen::Prewarmed } else { Reopen::Cold }
    } else if exists {
        Reopen::Kept
    } else {
        Reopen::First
    };
    let mut request = OPEN_REQUEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // a second ask for the same window (the click that waits for a window being made, and then the activation that shows it) keeps the first clock
    if request.is_some_and(|(at, _)| at.elapsed() < OPEN_REQUEST_TTL) {
        return;
    }
    *request = Some((Instant::now(), kind));
}

/// The main window is on screen: the reopen is timed and judged.
pub fn note_main_shown() {
    let Some((at, kind)) = OPEN_REQUEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() else { return };
    let ms = at.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    match kind {
        Reopen::First => {
            let limit = guard(|guard| {
                guard.record_cold_open(ms);
                guard.limit_ms()
            });
            info!("[memadapt] first cold window: {ms} ms against {limit} ms; main release requires a qualified cold open");
        }
        Reopen::Kept => {
            guard(|guard| guard.record_kept(ms));
            info!("[memadapt] reopen kept window: {ms} ms");
        }
        Reopen::Prewarmed | Reopen::Cold => {
            let situation = match RELEASE_PRESSURE.load(Ordering::Relaxed) {
                2 => Pressure::Critical,
                1 => Pressure::Elevated,
                _ => Pressure::None,
            };
            let (verdict, limit) = guard(|guard| (guard.record_released(situation, ms, now_secs()), guard.limit_ms()));
            info!(
                "[memadapt] reopen after a release ({}): {ms} ms against {limit} ms for a kept window: {verdict:?}{}",
                if kind == Reopen::Prewarmed { "made ahead" } else { "made by the click" },
                if verdict == Verdict::Blocked && guard_enabled() { "; releasing stops in this situation for six hours" } else { "" }
            );
            if verdict == Verdict::Blocked && guard_enabled() {
                save_guard();
            }
            RELEASED_MAIN.store(false, Ordering::SeqCst);
            prewarm_state().ready_at = None;
            crate::view_state::clear_carry();
        }
    }
}

// ---- timings --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

static CLICK_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// A right click on the tray icon arrived (the instant the Material menu's time is counted from).
pub fn note_click() {
    *CLICK_AT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now());
}

/// The Material menu is on screen: logs the time since the right click (it includes a wait for the window when the click came while it was being made).
pub fn note_menu_shown() {
    let at = CLICK_AT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    if let Some(at) = at.filter(|at| at.elapsed() < CLICK_STALE) {
        info!("[memadapt] right click -> Material menu on screen in {} ms", at.elapsed().as_millis());
    }
}

// ---- the main window's place -------------------------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Bounds {
    position: (i32, i32),
    size: (u32, u32),
    maximized: bool,
}

static BOUNDS: Mutex<Option<Bounds>> = Mutex::new(None);

fn keep_bounds<R: Runtime>(window: &WebviewWindow<R>) {
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else { return };
    let maximized = window.is_maximized().unwrap_or(false);
    *BOUNDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Bounds { position: (position.x, position.y), size: (size.width, size.height), maximized });
}

/// A window that is made again after a release gets the place and size the released one had (once; only if that place is still on a monitor).
pub fn restore_bounds<R: Runtime>(window: &WebviewWindow<R>) {
    let Some(bounds) = BOUNDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() else { return };
    let on_a_monitor = window.available_monitors().map(|monitors| {
        monitors.iter().any(|monitor| {
            let (p, s) = (monitor.position(), monitor.size());
            bounds.position.0 + 80 > p.x && bounds.position.0 + 80 < p.x + s.width as i32 && bounds.position.1 + 40 > p.y && bounds.position.1 + 40 < p.y + s.height as i32
        })
    });
    if !on_a_monitor.unwrap_or(false) {
        debug!("[memadapt] the place of the released window is on no monitor any more: the default place is kept");
        return;
    }
    let _ = window.set_size(PhysicalSize::new(bounds.size.0, bounds.size.1));
    let _ = window.set_position(PhysicalPosition::new(bounds.position.0, bounds.position.1));
    if bounds.maximized {
        let _ = window.maximize();
    }
    info!("[memadapt] the window is made again at {:?} {:?}{}", bounds.position, bounds.size, if bounds.maximized { " (maximized)" } else { "" });
}

// ---- the machine ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod system {
    use crate::mem_adaptive_core::Sample;
    use std::sync::OnceLock;

    /// One look: available physical memory (`GlobalMemoryStatusEx`, Microsoft Learn "MEMORYSTATUSEX": `ullAvailPhys`), the commit that can still be promised (`GetPerformanceInfo`:
    /// `CommitLimit - CommitTotal` pages) and the system's low-memory notification (`CreateMemoryResourceNotification`, `QueryMemoryResourceNotification`).
    pub fn sample() -> Option<Sample> {
        use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut status).ok()?;
            let mut performance: PERFORMANCE_INFORMATION = std::mem::zeroed();
            let size = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;
            performance.cb = size;
            let commit_headroom = if GetPerformanceInfo(&mut performance, size).is_ok() {
                (performance.CommitLimit.saturating_sub(performance.CommitTotal) as u64).saturating_mul(performance.PageSize as u64)
            } else {
                u64::MAX
            };
            Some(Sample { total_phys: status.ullTotalPhys, avail_phys: status.ullAvailPhys, commit_headroom, low_signal: low_memory_signalled() })
        }
    }

    fn low_memory_signalled() -> bool {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Memory::{CreateMemoryResourceNotification, LowMemoryResourceNotification, QueryMemoryResourceNotification};
        static NOTIFICATION: OnceLock<isize> = OnceLock::new();
        let handle = *NOTIFICATION.get_or_init(|| unsafe { CreateMemoryResourceNotification(LowMemoryResourceNotification).map(|handle| handle.0 as isize).unwrap_or(0) });
        if handle == 0 {
            return false;
        }
        let mut state = windows::core::BOOL(0);
        unsafe { QueryMemoryResourceNotification(HANDLE(handle as *mut core::ffi::c_void), &mut state).is_ok() && state.as_bool() }
    }
}

#[cfg(not(target_os = "windows"))]
mod system {
    use crate::mem_adaptive_core::Sample;

    pub fn sample() -> Option<Sample> {
        None
    }
}
