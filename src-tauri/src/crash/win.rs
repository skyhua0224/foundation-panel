//! Windows part of the panel's crash capture: the unhandled-exception filter, a vectored handler for the codes nothing recovers from, the C
//! runtime's abort, `MiniDumpWriteDump`, the stack walk and the WebView2 `ProcessFailed` event.
//!
//! The thread that crashed does almost nothing: it copies its context and wakes a worker made at start, then waits (bounded). The worker writes
//! the dump first (the kernel does the work), then a meta with the faulting frame alone, then walks the stack and writes the meta again, so a
//! stack too damaged to walk still leaves a report. Nothing on that path allocates: file names, the JSON and the frames live in fixed
//! buffers, because the heap may be what is broken. Sources: SetUnhandledExceptionFilter, AddVectoredExceptionHandler, MiniDumpWriteDump,
//! RtlVirtualUnwind and ICoreWebView2::add_ProcessFailed (Microsoft Learn, docs entries dr-ms-crash-* of the source index).

use super::meta::{self, Frame, MetaInput};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use core::mem::MaybeUninit;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicUsize, Ordering};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, HMODULE};
use windows::Win32::Storage::FileSystem::{
    CREATE_ALWAYS, CreateFileW, DeleteFileW, FILE_ATTRIBUTE_NORMAL, FILE_BEGIN, FILE_SHARE_MODE, GetFileSizeEx, MOVEFILE_REPLACE_EXISTING, MoveFileExW,
    SetEndOfFile, SetFilePointerEx, WriteFile,
};
use windows::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, EXCEPTION_RECORD, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpNormal,
    MiniDumpWithThreadInfo, MiniDumpWithUnloadedModules, MiniDumpWriteDump, RtlCaptureContext,
    RtlCaptureStackBackTrace, RtlLookupFunctionEntry, RtlVirtualUnwind, SetUnhandledExceptionFilter, UNW_FLAG_NHANDLER,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Threading::{CreateEventW, GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId, SetEvent, TerminateProcess, WaitForSingleObject};
use windows::core::PCWSTR;

struct Shared<T>(UnsafeCell<T>);
// SAFETY: the request is written by the one thread that wins CRASHING and read by the worker after the event; never concurrently.
unsafe impl<T> Sync for Shared<T> {}

#[repr(C, align(16))]
struct Request {
    kind: [u8; 16],
    code: [u8; 48],
    ctx: CONTEXT,
    rec: EXCEPTION_RECORD,
    ep: EXCEPTION_POINTERS,
    tid: u32,
    skip: usize,
}

static REQUEST: Shared<MaybeUninit<Request>> = Shared(UnsafeCell::new(MaybeUninit::zeroed()));
static REQUEST_EVENT: AtomicIsize = AtomicIsize::new(0);
static DONE_EVENT: AtomicIsize = AtomicIsize::new(0);
static WORKER_TID: AtomicU32 = AtomicU32::new(0);
static CRASHING: AtomicBool = AtomicBool::new(false);
static PENDING_WIDE: OnceLock<Vec<u16>> = OnceLock::new();
static WEBVIEW_REPORTS: AtomicUsize = AtomicUsize::new(0);

const PATH_CAP: usize = 1300;
const FRAME_CAP: usize = meta::MAX_FRAMES + 4;
const SIGABRT: i32 = 22;

unsafe extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn abort() -> !;
}

fn handle(raw: &AtomicIsize) -> HANDLE {
    HANDLE(raw.load(Ordering::Acquire) as *mut core::ffi::c_void)
}

fn set_text<const N: usize>(dst: &mut [u8; N], src: &str) {
    dst.fill(0);
    for (i, b) in src.bytes().take(N - 1).enumerate() {
        dst[i] = b;
    }
}

fn text<const N: usize>(src: &[u8; N]) -> &str {
    let len = src.iter().position(|&b| b == 0).unwrap_or(N);
    core::str::from_utf8(&src[..len]).unwrap_or("")
}

/// The file name of the module that holds `pc` and `pc` minus its base.
fn resolve(pc: u64) -> Frame {
    let mut module = HMODULE::default();
    // SAFETY: FROM_ADDRESS with UNCHANGED_REFCOUNT only reads the loader's list; pc is just an address to look up.
    let found = unsafe {
        GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, PCWSTR(pc as *const u16), &mut module)
    };
    if found.is_err() || module.0.is_null() {
        return Frame::new("unknown", pc);
    }
    let mut name = [0u16; 260];
    // SAFETY: the buffer is ours and its length is passed implicitly by the slice.
    let len = unsafe { GetModuleFileNameW(Some(module), &mut name) } as usize;
    let start = name[..len.min(260)].iter().rposition(|&c| c == u16::from(b'\\') || c == u16::from(b'/')).map_or(0, |i| i + 1);
    let mut ascii = [0u8; 64];
    let mut n = 0;
    for &c in &name[start..len.min(260)] {
        if n == ascii.len() - 1 {
            break;
        }
        ascii[n] = if c < 0x80 { c as u8 } else { b'?' };
        n += 1;
    }
    let mut frame = Frame::EMPTY;
    frame.set_module(&ascii[..n]);
    frame.offset = pc.wrapping_sub(module.0 as u64);
    frame
}

fn readable(address: u64) -> bool {
    use windows::Win32::System::Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery};
    // SAFETY: a plain C struct, all zero is a valid value.
    let mut info: MEMORY_BASIC_INFORMATION = unsafe { core::mem::zeroed() };
    // SAFETY: VirtualQuery only reads the address-space description.
    let got = unsafe { VirtualQuery(Some(address as *const core::ffi::c_void), &mut info, core::mem::size_of::<MEMORY_BASIC_INFORMATION>()) };
    got != 0 && info.State == MEM_COMMIT && (info.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0)) == 0 && address + 16 <= info.BaseAddress as u64 + info.RegionSize as u64
}

/// Frames of a context from the unwind data of the images; stops at a stack it cannot read.
fn unwind(mut ctx: CONTEXT, frames: &mut [Frame; FRAME_CAP]) -> usize {
    let mut n = 0;
    while n < FRAME_CAP {
        let pc = ctx.Rip;
        if pc == 0 {
            break;
        }
        frames[n] = resolve(pc);
        n += 1;
        if !readable(ctx.Rsp) {
            break;
        }
        // frames after the first are return addresses: look the function up one byte earlier
        let lookup = if n > 1 { pc - 1 } else { pc };
        let mut image_base = 0u64;
        // SAFETY: the lookup reads the images' unwind tables; the unwind stays inside the stack the check above found readable.
        unsafe {
            let entry = RtlLookupFunctionEntry(lookup, &mut image_base, None);
            if entry.is_null() {
                ctx.Rip = *(ctx.Rsp as *const u64);
                ctx.Rsp += 8;
            } else {
                let mut handler_data = core::ptr::null_mut();
                let mut frame_pointer = 0u64;
                RtlVirtualUnwind(UNW_FLAG_NHANDLER, image_base, lookup, entry, &mut ctx, &mut handler_data, &mut frame_pointer, None);
            }
        }
    }
    n
}

/// The stack of the calling thread as module + offset, for a panic report.
pub(super) fn capture_stack(max: usize) -> Vec<Frame> {
    let mut raw = [core::ptr::null_mut::<core::ffi::c_void>(); meta::MAX_FRAMES];
    // SAFETY: the slice is ours.
    let got = unsafe { RtlCaptureStackBackTrace(1, &mut raw[..max.min(meta::MAX_FRAMES)], None) } as usize;
    raw[..got].iter().map(|&p| resolve(p as u64)).collect()
}

type WidePath = [u16; PATH_CAP];

fn path_in_pending(stem: &str, ext: &str) -> Option<WidePath> {
    let dir = PENDING_WIDE.get()?;
    let mut buffer = [0u16; PATH_CAP];
    let mut n = 0;
    let mut push = |c: u16| {
        if n + 1 < PATH_CAP {
            buffer[n] = c;
            n += 1;
        }
    };
    dir.iter().for_each(|&c| push(c));
    push(u16::from(b'\\'));
    stem.bytes().for_each(|b| push(u16::from(b)));
    ext.bytes().for_each(|b| push(u16::from(b)));
    Some(buffer)
}

fn write_file(path: &WidePath, bytes: &[u8]) -> bool {
    // SAFETY: the path is NUL terminated (the buffer starts zeroed).
    unsafe {
        let Ok(file) = CreateFileW(PCWSTR(path.as_ptr()), GENERIC_WRITE.0, FILE_SHARE_MODE(0), None, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, None) else { return false };
        let mut written = 0u32;
        let ok = WriteFile(file, Some(bytes), Some(&mut written), None).is_ok() && written as usize == bytes.len();
        let _ = CloseHandle(file);
        ok
    }
}

fn write_meta(stem: &str, kind: &str, code: &str, frames: &[Frame]) {
    let Some(state) = super::state() else { return };
    let Some(json) = meta::render_meta(&MetaInput {
        product: "panel",
        version: &state.version,
        os_version: &state.os_version,
        install_id: "",
        kind,
        code,
        time: super::unix_now(),
        uptime_s: state.started.elapsed().as_secs(),
        frames,
    }) else {
        return;
    };
    let (Some(temp), Some(target)) = (path_in_pending(stem, ".json.tmp"), path_in_pending(stem, ".json")) else { return };
    if write_file(&temp, json.as_bytes()) {
        // SAFETY: both paths are NUL terminated.
        let _ = unsafe { MoveFileExW(PCWSTR(temp.as_ptr()), PCWSTR(target.as_ptr()), MOVEFILE_REPLACE_EXISTING) };
    }
}

fn write_dump(stem: &str, ep: *mut EXCEPTION_POINTERS, tid: u32) -> bool {
    let Some(path) = path_in_pending(stem, ".dmp") else { return false };
    // SAFETY: handles and pointers are ours for the length of the call; the exception pointers, when given, outlive it.
    unsafe {
        let Ok(file) = CreateFileW(PCWSTR(path.as_ptr()), GENERIC_READ.0 | GENERIC_WRITE.0, FILE_SHARE_MODE(0), None, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, None) else { return false };
        let info = MINIDUMP_EXCEPTION_INFORMATION { ThreadId: tid, ExceptionPointers: ep, ClientPointers: false.into() };
        let exception = (!ep.is_null()).then_some(&info as *const MINIDUMP_EXCEPTION_INFORMATION);
        // The smallest dump that still names the faulting place (MINIDUMP_TYPE, Microsoft Learn): MiniDumpNormal has no heap, no data segments and
        // no handle data; MiniDumpFilterMemory keeps only the pointer-sized values of each stack (a string, a key or a token that lived on a stack
        // is blanked); MiniDumpFilterModulePaths removes the account name from module paths; MiniDumpWithoutOptionalData and
        // MiniDumpWithoutAuxiliaryState drop what is not essential. MiniDumpWithIndirectlyReferencedMemory (the heap behind each stack pointer,
        // where credentials, session tokens and page text live) is NOT used.
        let kind = windows::Win32::System::Diagnostics::Debug::MINIDUMP_TYPE(
            MiniDumpNormal.0 | MiniDumpWithThreadInfo.0 | MiniDumpWithUnloadedModules.0 | 0x0000_0008 | 0x0000_0080 | 0x0000_0400 | 0x0000_4000,
        );
        let _ = SetFilePointerEx(file, 0, None, FILE_BEGIN);
        let _ = SetEndOfFile(file);
        let mut ok = MiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(), file, kind, exception, None, None).is_ok();
        let mut bytes = 0i64;
        let _ = GetFileSizeEx(file, &mut bytes);
        if ok && bytes as u64 > meta::MAX_DUMP_BYTES {
            ok = false;
        }
        let _ = CloseHandle(file);
        if !ok {
            let _ = DeleteFileW(PCWSTR(path.as_ptr()));
        }
        ok
    }
}

/// One report: dump, then the meta with the faulting frame, then the full meta.
fn write_reports(request: &Request) {
    if !super::enabled() {
        return;
    }
    let kind = text(&request.kind);
    let code = text(&request.code);
    let stem = meta::stem(super::unix_now(), "panel", kind, std::process::id());
    write_dump(stem.as_str(), core::ptr::from_ref(&request.ep).cast_mut(), request.tid);
    write_meta(stem.as_str(), kind, code, &[resolve(request.ctx.Rip)]);
    let mut frames = [Frame::EMPTY; FRAME_CAP];
    let count = unwind(request.ctx, &mut frames);
    let from = if request.skip < count { request.skip } else { 0 };
    write_meta(stem.as_str(), kind, code, &frames[from..count.min(from + meta::MAX_FRAMES)]);
}

fn worker() {
    WORKER_TID.store(unsafe { GetCurrentThreadId() }, Ordering::Release);
    loop {
        // SAFETY: the event handles live for the process.
        unsafe { WaitForSingleObject(handle(&REQUEST_EVENT), u32::MAX) };
        // SAFETY: the crashing thread filled the request before setting the event and now only waits.
        let request = unsafe { (*REQUEST.0.get()).assume_init_ref() };
        write_reports(request);
        // SAFETY: as above.
        let _ = unsafe { SetEvent(handle(&DONE_EVENT)) };
    }
}

fn finish(exit_code: u32) -> ! {
    // SAFETY: ends this process, which is the point.
    unsafe {
        let _ = TerminateProcess(GetCurrentProcess(), exit_code);
    }
    loop {
        std::thread::park();
    }
}

/// The crashing thread: hand over to the worker, wait for it (bounded), end the process.
#[inline(never)]
fn report_and_die(kind: &str, code: &str, ep: *const EXCEPTION_POINTERS, exit_code: u32) -> ! {
    // SAFETY: plain thread id read.
    if unsafe { GetCurrentThreadId() } == WORKER_TID.load(Ordering::Acquire) {
        finish(exit_code); // the reporter itself crashed
    }
    if CRASHING.swap(true, Ordering::AcqRel) {
        loop {
            std::thread::park(); // another thread is reporting; this one waits for the process to end
        }
    }
    if super::enabled() && REQUEST_EVENT.load(Ordering::Acquire) != 0 {
        // SAFETY: this thread won CRASHING, so it is the only writer; the worker is waiting on the event.
        unsafe {
            let request = (*REQUEST.0.get()).assume_init_mut();
            set_text(&mut request.kind, kind);
            set_text(&mut request.code, code);
            request.tid = GetCurrentThreadId();
            if ep.is_null() {
                // no exception behind it (abort): a context here, the frames of this function and its handler left out
                RtlCaptureContext(&mut request.ctx);
                request.rec = core::mem::zeroed();
                request.rec.ExceptionCode = windows::Win32::Foundation::NTSTATUS(0xE0000001u32 as i32);
                request.rec.ExceptionAddress = request.ctx.Rip as *mut core::ffi::c_void;
                request.skip = 2;
            } else {
                request.rec = *(*ep).ExceptionRecord;
                request.ctx = *(*ep).ContextRecord;
                request.skip = 0;
            }
            request.ep = EXCEPTION_POINTERS { ExceptionRecord: &mut request.rec, ContextRecord: &mut request.ctx };
            let _ = SetEvent(handle(&REQUEST_EVENT));
            WaitForSingleObject(handle(&DONE_EVENT), 60_000);
        }
    }
    finish(exit_code)
}

fn hex_code(code: u32) -> meta::Buf<16> {
    let mut out = meta::Buf::<16>::new();
    let _ = write!(out, "0x{code:08X}");
    out
}

unsafe extern "system" fn unhandled_filter(ep: *const EXCEPTION_POINTERS) -> i32 {
    // SAFETY: the system passes valid pointers.
    let code = unsafe { (*(*ep).ExceptionRecord).ExceptionCode.0 as u32 };
    report_and_die("crash", hex_code(code).as_str(), ep, code)
}

/// Only the codes no program recovers from: a first-chance access violation is often caught on purpose (guarded reads, CPU probes).
unsafe extern "system" fn vectored_fatal(ep: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: the system passes valid pointers.
    let code = unsafe { (*(*ep).ExceptionRecord).ExceptionCode.0 as u32 };
    if code == 0xC000_00FD /* stack overflow */ || code == 0xC000_0374 /* heap corruption */ {
        report_and_die("crash", hex_code(code).as_str(), ep, code);
    }
    0 // EXCEPTION_CONTINUE_SEARCH
}

#[inline(never)]
extern "C" fn on_sigabrt(_signal: i32) {
    report_and_die("abort", "SIGABRT", core::ptr::null(), 3)
}

/// Installs the native handlers and the worker.
pub(super) fn install() {
    let Some(state) = super::state() else { return };
    let mut wide: Vec<u16> = state.pending.as_os_str().to_string_lossy().encode_utf16().collect();
    wide.truncate(PATH_CAP - 200);
    let _ = PENDING_WIDE.set(wide);
    // SAFETY: plain Win32 calls with valid arguments.
    unsafe {
        let (Ok(request), Ok(done)) = (CreateEventW(None, false, false, PCWSTR::null()), CreateEventW(None, true, false, PCWSTR::null())) else { return };
        REQUEST_EVENT.store(request.0 as isize, Ordering::Release);
        DONE_EVENT.store(done.0 as isize, Ordering::Release);
    }
    if std::thread::Builder::new().name("crash-worker".into()).stack_size(512 * 1024).spawn(worker).is_err() {
        return;
    }
    // SAFETY: handlers are plain functions that live for the process.
    unsafe {
        SetUnhandledExceptionFilter(Some(unhandled_filter));
        AddVectoredExceptionHandler(1, Some(vectored_fatal));
        signal(SIGABRT, on_sigabrt as usize);
    }
}

/// SUNSHINE_TEST_CRASH: fail on purpose, after the handlers are in.
pub(super) fn test_crash(mode: &str) {
    // give the worker its thread id before anything crashes
    std::thread::sleep(std::time::Duration::from_millis(300));
    match mode {
        "null-deref" => {
            // SAFETY: not safe on purpose.
            unsafe { core::ptr::write_volatile(core::ptr::null_mut::<u32>(), 42) };
        }
        "abort" => {
            // SAFETY: the C runtime's abort.
            unsafe { abort() }
        }
        _ => {}
    }
}

fn webview_kind(kind: i32) -> &'static str {
    match kind {
        0 => "browser_exited",
        1 => "render_exited",
        2 => "render_unresponsive",
        3 => "frame_render_exited",
        4 => "utility_exited",
        5 => "sandbox_helper_exited",
        6 => "gpu_exited",
        7 => "ppapi_plugin_exited",
        8 => "ppapi_broker_exited",
        _ => "unknown_exited",
    }
}

/// `COREWEBVIEW2_PROCESS_FAILED_REASON` by name (Microsoft Learn, WebView2 `ProcessFailed` event args).
fn webview_reason(reason: i32) -> &'static str {
    match reason {
        0 => "unexpected",
        1 => "unresponsive",
        2 => "terminated",
        3 => "crashed",
        4 => "launch_failed",
        5 => "out_of_memory",
        6 => "profile_deleted",
        _ => "unknown",
    }
}

/// The dialog that tells the person the window cannot be shown is shown once per run.
static WINDOW_FAILED_SHOWN: AtomicBool = AtomicBool::new(false);

/// Watches the WebView2 processes of this webview. Every failure (a few per run) is recorded as kind "panic": the kind and the exit code are the frame. A dead renderer is
/// reloaded; one that keeps dying, or a GPU process that cannot stay up, makes the panel start again with GPU acceleration off; a dead browser starts the panel again
/// (`recover.rs` decides; `restart` starts it), so the window does not stay blank. When nothing helps any more, the person is told in a native dialog.
pub(crate) fn register_webview_failures(core_webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2, restart: impl Fn() + 'static) {
    use super::recover::{self, Action};
    use webview2_com::Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_REASON, ICoreWebView2ProcessFailedEventArgs2};
    use webview2_com::ProcessFailedEventHandler;
    use wv2_windows_core::Interface;
    let webview = core_webview.clone();
    let mut history = recover::History::default();
    let handler = ProcessFailedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else { return Ok(()) };
        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
        // SAFETY: COM calls on the event's own arguments.
        unsafe { args.ProcessFailedKind(&mut kind)? };
        let mut exit_code = 0i32;
        let mut reason = 0i32;
        if let Ok(args2) = args.cast::<ICoreWebView2ProcessFailedEventArgs2>() {
            // SAFETY: as above.
            let _ = unsafe { args2.ExitCode(&mut exit_code) };
            let mut value = COREWEBVIEW2_PROCESS_FAILED_REASON::default();
            // SAFETY: as above.
            if unsafe { args2.Reason(&mut value) }.is_ok() {
                reason = value.0;
            }
        }
        let name = webview_kind(kind.0);
        // The panel is quitting or restarting itself (a tray quit, the update handoff, an elevated or webview
        // restart): the WebView2 processes end with it. The recovery this handler exists for (reload the page,
        // start the panel again, tell the person) would fight the shutdown, and the report would call a normal
        // exit a failure of the machine; the guidance behind ProcessFailed covers a running app, and an app on
        // its way out has nothing to recover (panel-ms-webview2-process-events-20261121).
        if crate::exit_state::is_exiting() {
            log::info!(
                "WebView2 {name} (exit code {exit_code:#x}) while the panel is exiting ({}): not a failure, not reported",
                crate::exit_state::reason().unwrap_or("unknown")
            );
            return Ok(());
        }
        // The reason (a crash, an out-of-memory exit, a launch that failed) is what tells a driver fault from a full machine; it goes to the log.
        log::warn!("WebView2 process failed: {name}, reason {}, exit code {exit_code:#x}", webview_reason(reason));
        let mut label = meta::Buf::<48>::new();
        let _ = write!(label, "webview2:{name}");
        // a panel that has run for ten minutes has recovered from the last restart
        let restarts = match super::state() {
            Some(state) if state.started.elapsed() < std::time::Duration::from_secs(600) => recover::restarts_so_far(std::env::var(recover::RESTARTS_ENV).ok().as_deref()),
            _ => 0,
        };
        let mode = super::fallback::render_mode();
        let action = recover::decide(name, std::time::Instant::now(), &mut history, restarts, mode);
        // the report is written before the action runs: a restart ends this process. A GPU process that exited once and was started again by Chromium is no crash.
        if recover::worth_reporting(name, &history) && WEBVIEW_REPORTS.fetch_add(1, Ordering::Relaxed) < 3 {
            let frame = Frame::new(label.as_str(), u64::from(exit_code as u32));
            super::record("panic", label.as_str(), &[frame]);
        }
        match action {
            Action::Reload => {
                log::warn!("WebView2 {name} (exit code {exit_code:#x}): reloading the page");
                // SAFETY: the event runs on the thread of the webview.
                let _ = unsafe { webview.Reload() };
            }
            Action::Restart => {
                log::warn!("WebView2 {name} (exit code {exit_code:#x}): starting the panel again");
                // SAFETY: nothing else reads the environment of this process at this point.
                unsafe { std::env::set_var(recover::RESTARTS_ENV, (restarts + 1).to_string()) };
                restart();
            }
            Action::SoftwareRestart => {
                log::warn!("WebView2 {name} (exit code {exit_code:#x}): the window keeps failing; starting the panel again with GPU acceleration off");
                super::fallback::remember(&format!("webview2:{name} reason {} exit code {exit_code:#x}", webview_reason(reason)));
                restart();
            }
            Action::GiveUp => {
                log::error!("WebView2 {name} (exit code {exit_code:#x}): the window still fails with GPU acceleration off; telling the person");
                // once: every further failure of the same window would only stack dialogs
                if !WINDOW_FAILED_SHOWN.swap(true, Ordering::Relaxed) {
                    crate::webview_runtime::show_window_failed_dialog();
                }
            }
            Action::Nothing => {}
        }
        Ok(())
    }));
    let mut token = 0;
    // SAFETY: the handler is kept alive by the COM registration.
    if let Err(error) = unsafe { core_webview.add_ProcessFailed(&handler, &mut token) } {
        log::warn!("could not watch WebView2 process failures: {error}");
    }
}
