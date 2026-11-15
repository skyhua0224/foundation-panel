//! Error capture of the Panel shell (the Rust side): a panic hook that records the location, the stack, the shell's recent actions, a scrubbed tail of the Panel log and the build, as an `error_report`
//! event (`error_report.rs` builds it in the shape the host and the Worker take).
//!
//! Ways out. A panic may end the process, so the report is first appended to a small spool file next to the Panel log. Where the shell lives on, a Tauri event tells the page, which drains the spool
//! (`error_capture_drain`) and queues the events in its ordinary telemetry sender: the same consent choice (error reports), the same endpoint through the host (`POST /api/telemetry/events`), the same
//! install identity and queue. At the next start the page drains what a fatal panic left. There is no second sender and no second consent check in the shell.
//!
//! The log tail goes through the shared scrubber (`log_scrub.rs`, the same rules and test vectors as the Worker), and the Worker scrubs it once more. The panic message is never read.
//!
//! Primary sources: Rust std `std::panic::set_hook` and `PanicHookInfo` (doc.rust-lang.org/std/panic), Windows `RtlCaptureStackBackTrace` and `GetModuleHandleExW` (Microsoft Learn).
use crate::error_report::{Breadcrumbs, Crumb, LOG_LINE_CHARS, LOG_LINES, MAX_CRUMBS, MAX_FRAMES, clean_frame, panic_report, spool_append, spool_drain};
use serde_json::Value;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// At most this many reports are written in one run (a panic loop must not fill the disk).
const MAX_PER_RUN: usize = 5;

static CRUMBS: std::sync::LazyLock<Breadcrumbs> = std::sync::LazyLock::new(|| Breadcrumbs::new(MAX_CRUMBS));
static IN_HOOK: AtomicBool = AtomicBool::new(false);
static WRITTEN: AtomicUsize = AtomicUsize::new(0);
static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
static BUILD: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

pub fn spool_path() -> std::path::PathBuf {
    crate::logger::panel_log_path().with_file_name("error-spool.jsonl")
}

/// A crumb of the shell: a page opened, a service started. `id` becomes a lower-case word.
pub fn crumb(kind: Crumb, id: &str) {
    CRUMBS.add(kind, id, now_ms());
}

/// A Tauri command was invoked (the command's name, a fixed identifier of this program).
pub fn note_command(name: &str) {
    crumb(Crumb::Act, name);
}

/// The scrubbed tail of the Panel log (the last 64 KiB read, 40 lines kept).
fn panel_log_tail() -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(crate::logger::panel_log_path()) else { return Vec::new() };
    let len = file.metadata().map_or(0, |m| m.len());
    if file.seek(SeekFrom::Start(len.saturating_sub(64 * 1024))).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if file.take(64 * 1024).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    // the first line of a tail may be cut in the middle
    let text = if len > 64 * 1024 { text.split_once('\n').map_or(String::new(), |(_, rest)| rest.to_string()) } else { text.into_owned() };
    crate::log_scrub::scrubbed_tail(&text, LOG_LINES, LOG_LINE_CHARS)
}

/// The stack of the calling thread as `module+0xoffset` (Windows: RtlCaptureStackBackTrace and the module of each address); empty elsewhere, where the panel is only built for tests.
#[cfg(windows)]
pub fn capture_frames(skip: u32) -> Vec<String> {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;
    use windows::Win32::System::LibraryLoader::{GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW, GetModuleHandleExW};
    use windows::core::PCWSTR;

    let mut addresses = [std::ptr::null_mut::<std::ffi::c_void>(); MAX_FRAMES];
    // SAFETY: the API fills at most as many entries as the slice holds (MAX_FRAMES).
    let count = unsafe { RtlCaptureStackBackTrace(skip, &mut addresses, None) } as usize;
    let mut frames = Vec::new();
    for &address in addresses.iter().take(count) {
        let mut module = HMODULE::default();
        // SAFETY: FROM_ADDRESS treats the second argument as an address inside a module; UNCHANGED_REFCOUNT takes no reference, so there is nothing to release.
        let found = unsafe { GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, PCWSTR(address.cast()), &mut module) };
        if found.is_err() {
            continue;
        }
        let mut name = [0u16; 260];
        // SAFETY: `name` is a writable buffer; the API takes its length from the slice.
        let n = unsafe { GetModuleFileNameW(Some(module), &mut name) } as usize;
        let path = String::from_utf16_lossy(&name[..n.min(name.len())]);
        let file = path.rsplit(['\\', '/']).next().unwrap_or("");
        let offset = (address as usize).wrapping_sub(module.0 as usize);
        if let Some(frame) = clean_frame(&format!("{file}+0x{offset:x}")) {
            frames.push(frame);
        }
    }
    frames
}

#[cfg(not(windows))]
pub fn capture_frames(_skip: u32) -> Vec<String> {
    Vec::new()
}

/// Install the panic hook (call once, early in `main`). The previous hook still runs afterwards, so the message goes to stderr or the log as before.
pub fn install(build: &str) {
    let _ = BUILD.set(build.to_string());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // a panic while capturing a panic must not recurse
        if !IN_HOOK.swap(true, Ordering::SeqCst) {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| capture(info)));
            IN_HOOK.store(false, Ordering::SeqCst);
        }
        previous(info);
    }));
}

/// Give the hook an app handle so that a panic of a thread the shell survives reaches the page at once.
pub fn attach(app: &tauri::AppHandle) {
    let _ = APP.set(app.clone());
}

fn capture(info: &std::panic::PanicHookInfo<'_>) {
    if WRITTEN.load(Ordering::SeqCst) >= MAX_PER_RUN {
        return;
    }
    let thread = std::thread::current();
    let location = info.location().map(|l| (l.file().to_string(), l.line()));
    let report = panic_report(
        location.as_ref().map(|(f, l)| (f.as_str(), *l)),
        thread.name().unwrap_or(""),
        capture_frames(2),
        CRUMBS.snapshot(now_ms()),
        panel_log_tail(),
        BUILD.get().map_or("", String::as_str),
    );
    let event = report.event();
    // the same panic again in this run is the same error: written once
    let fingerprint = event["props"]["fingerprint"].as_str().unwrap_or("").to_string();
    if let Ok(mut seen) = SEEN.lock() {
        if seen.contains(&fingerprint) {
            return;
        }
        seen.push(fingerprint);
    }
    if spool_append(&spool_path(), &event).is_ok() {
        WRITTEN.fetch_add(1, Ordering::SeqCst);
        if let Some(app) = APP.get() {
            use tauri::Emitter;
            let app = app.clone();
            // not on the panicking thread: the event machinery may be what failed
            let _ = std::thread::Builder::new().name("error-capture-emit".into()).spawn(move || {
                let _ = app.emit("error-capture", ());
            });
        }
    }
}

/// The reports the shell wrote since the page last asked (at its start, and after each `error-capture` event).
#[tauri::command]
pub fn error_capture_drain() -> Vec<Value> {
    spool_drain(&spool_path())
}

/// Wrap the invoke handler so that every Tauri command is a crumb (its name only, at most once in 30 seconds): what the person did in the shell before an error.
pub fn record_commands<R: tauri::Runtime>(
    handler: impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke| {
        note_command(invoke.message.command());
        handler(invoke)
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn the_stack_of_this_thread_is_module_plus_offset() {
        let frames = capture_frames(0);
        assert!(!frames.is_empty(), "RtlCaptureStackBackTrace gave no frame");
        let shape = regex::Regex::new(r"(?i)^[A-Za-z0-9_.-]+\.(?:exe|dll)\+0x[0-9a-f]+$").unwrap();
        assert!(frames.iter().all(|f| shape.is_match(f)), "{frames:?}");
        assert!(frames.len() <= MAX_FRAMES);
        // the test binary is the first module on the stack
        assert!(frames.iter().any(|f| f.to_ascii_lowercase().starts_with("sunshine_gui") || f.to_ascii_lowercase().starts_with("sunshine-gui")), "{frames:?}");
    }

    #[test]
    fn a_real_panic_through_the_hook_leaves_a_report_in_the_spool() {
        let dir = tempfile::tempdir().unwrap();
        let spool = dir.path().join("error-spool.jsonl");
        let report = crate::error_report::panic_report(Some(("src/error_capture.rs", 1)), "worker", capture_frames(0), vec!["3|act|a".into()], vec!["line".into()], "test");
        crate::error_report::spool_append(&spool, &report.event()).unwrap();
        let drained = crate::error_report::spool_drain(&spool);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0]["props"]["lang"], "rust");
        assert!(drained[0]["props"]["frames"].as_array().unwrap().len() >= 2);
    }
}
