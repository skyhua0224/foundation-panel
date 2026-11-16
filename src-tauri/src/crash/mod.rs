//! Crash, panic and WebView2-failure capture for the panel; the upload of what it wrote (telemetry v1, `POST /v1/crash`).
//!
//! * A Rust panic leaves a meta (kind "panic"): the code location (file name and line, never the message: it may carry free text) as the
//!   first frame, then the stack as module + offset. The panic hook then hands over to the one that was there.
//! * A native crash (access violation, stack overflow, heap corruption, an abort of the C runtime) writes a minidump (threads, their stacks
//!   and what they point at; no full memory) and a meta, from a worker thread made at start, like the host does (`src/crash/` of the host).
//! * A failure of a WebView2 process (`ProcessFailed`) is recorded as kind "panic" with the failure kind and exit code as its one frame.
//!
//! Reports go to `%LOCALAPPDATA%\AlkaidLab\Sunshine Panel\crashes\pending` (the install folder is not writable by a standard user). They are
//! sent on the next start and every 30 minutes while error reports are on (`telemetry_errors` of the host's sunshine.conf, default on), with
//! the host's install id (`<config>/telemetry/install_id`); at most 20 stay, none is kept once error reports are turned off, and nothing here
//! waits on anything the panel needs to start.

mod meta;
mod recover;
mod redact;
mod upload;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub(crate) use win::register_webview_failures;

use meta::{Frame, MetaInput};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct State {
    pub pending: PathBuf,
    pub config_dir: PathBuf,
    pub version: String,
    pub os_version: String,
    pub started: Instant,
}

static STATE: OnceLock<State> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(true);
static PANIC_REPORTS: AtomicUsize = AtomicUsize::new(0);
static SEEN_PANICS: Mutex<Vec<(String, u32)>> = Mutex::new(Vec::new());

/// A panic that fires in a loop is one report, and a run sends at most a handful.
const MAX_PANIC_REPORTS: usize = 5;

pub(crate) fn state() -> Option<&'static State> {
    STATE.get()
}

/// Whether error reports may be written and sent right now.
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn read_conf(config_dir: &Path) -> String {
    std::fs::read_to_string(config_dir.join("sunshine.conf")).unwrap_or_default()
}

/// Re-reads the consent; turning error reports off removes what is waiting.
fn refresh_consent() {
    let Some(state) = STATE.get() else { return };
    let on = meta::errors_enabled(&read_conf(&state.config_dir));
    if ENABLED.swap(on, Ordering::Relaxed) && !on {
        meta::clear_pending(&state.pending);
    }
}

#[cfg(windows)]
fn os_version() -> String {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let build = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|key| key.get_value::<String, _>("CurrentBuildNumber"))
        .unwrap_or_default();
    if build.is_empty() { "Windows".to_string() } else { format!("Windows 10.0.{build}") }
}

#[cfg(not(windows))]
fn os_version() -> String {
    std::env::consts::OS.to_string()
}

fn pending_dir() -> PathBuf {
    // tests point the capture at a scratch folder with SUNSHINE_CRASH_DIR
    if let Some(dir) = std::env::var_os("SUNSHINE_CRASH_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    meta::pending_dir_in(&base)
}

/// Writes one report from a normal thread (a panic hook, a WebView2 callback): the meta file, no dump.
pub(crate) fn record(kind: &str, code: &str, frames: &[Frame]) {
    let Some(state) = STATE.get() else { return };
    if !enabled() {
        return;
    }
    let now = unix_now();
    let stem = meta::stem(now, "panel", kind, std::process::id());
    let Some(json) = meta::render_meta(&MetaInput {
        product: "panel",
        version: &state.version,
        os_version: &state.os_version,
        install_id: "",
        kind,
        code,
        time: now,
        uptime_s: state.started.elapsed().as_secs(),
        frames,
    }) else {
        return;
    };
    let target = state.pending.join(format!("{}.json", stem.as_str()));
    let temp = state.pending.join(format!("{}.json.tmp", stem.as_str()));
    if std::fs::write(&temp, json.as_bytes()).is_ok() {
        let _ = std::fs::rename(&temp, &target);
    }
    meta::trim_pending(&state.pending, meta::MAX_PENDING, 30);
}

fn on_panic(info: &std::panic::PanicHookInfo<'_>) {
    if !enabled() || PANIC_REPORTS.load(Ordering::Relaxed) >= MAX_PANIC_REPORTS {
        return;
    }
    let (file, line) = info.location().map_or(("unknown", 0), |l| (l.file(), l.line()));
    // the file name only: a path can carry a user name
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file).to_string();
    {
        let Ok(mut seen) = SEEN_PANICS.lock() else { return };
        if seen.iter().any(|(n, l)| *n == name && *l == line) {
            return;
        }
        seen.push((name.clone(), line));
    }
    PANIC_REPORTS.fetch_add(1, Ordering::Relaxed);
    let mut frames = vec![Frame::new(&format!("panic:{name}"), u64::from(line))];
    #[cfg(windows)]
    frames.extend(win::capture_stack(meta::MAX_FRAMES - 1));
    record("panic", "panic", &frames);
}

fn uploader() {
    let delay = std::env::var("SUNSHINE_CRASH_UPLOAD_DELAY_S").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(20);
    std::thread::sleep(Duration::from_secs(delay));
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
    let mut next_upload = Instant::now();
    loop {
        refresh_consent();
        if enabled() && Instant::now() >= next_upload {
            if let Some(state) = STATE.get() {
                let conf = read_conf(&state.config_dir);
                if let Some(id) = meta::read_install_id(&state.config_dir) {
                    runtime.block_on(upload::upload_pending_once(&state.pending, &meta::endpoint(&conf), &id));
                }
            }
            next_upload = Instant::now() + Duration::from_secs(30 * 60);
        }
        std::thread::sleep(Duration::from_secs(30));
    }
}

/// Installs the capture and starts the uploader. Call once, early in `main`.
pub(crate) fn install(version: &str, config_dir: PathBuf) {
    let pending = pending_dir();
    let _ = std::fs::create_dir_all(&pending);
    let state = State { pending, config_dir, version: version.to_string(), os_version: os_version(), started: Instant::now() };
    if STATE.set(state).is_err() {
        return;
    }
    refresh_consent();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        on_panic(info);
        previous(info);
    }));
    #[cfg(windows)]
    win::install();
    // 8 MiB: the first upload (TLS through SSPI, multipart bodies, a runtime of its own) overflowed the 2 MiB default twice on one machine, 20 seconds after the start
    let _ = std::thread::Builder::new().name("crash-upload".into()).stack_size(8 * 1024 * 1024).spawn(uploader);
    run_test_switch();
}

/// SUNSHINE_TEST_CRASH=null-deref|abort|panic|hang makes this process fail on purpose (tests only; never set in a real run).
fn run_test_switch() {
    match std::env::var("SUNSHINE_TEST_CRASH").as_deref() {
        Ok("panic") => {
            // the hook runs, then the thread unwinds; the process ends so the test sees the report
            let _ = std::thread::spawn(|| panic!("SUNSHINE_TEST_CRASH panic")).join();
            std::process::exit(101);
        }
        #[cfg(windows)]
        Ok(mode) if matches!(mode, "null-deref" | "abort" | "overflow") => win::test_crash(mode),
        _ => {}
    }
}
