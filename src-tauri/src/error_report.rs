//! The error report of a component, as the host and the Worker take it (host `src/telemetry/error_report.h`, Worker `src/errorreports.ts`): a location, a stack, the last actions (breadcrumbs), a scrubbed log
//! tail and the build, as an `error_report` event. Pure code (std, serde_json): the panic hook and the Tauri side are in `error_capture.rs`.
//!
//! Everything here is fixed words and numbers. A panic *message* is never read (it can carry a path, a name or a value); the code is the word `panic`. A source location is `src/file.rs:line` relative to
//! the crate, a dependency is `dep/<crate>-<version>/src/file.rs:line`, a native frame is `module.exe+0xoffset`; none holds a user, build or home directory. The fingerprint is the host's (`fingerprint_of`
//! in `error_report.cpp`): the same error has the same fingerprint on every build.
//!
//! The spool is a small file of events that survives a process that is ending (see `error_capture.rs`).
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

pub const MAX_FRAMES: usize = 32;
pub const MAX_CRUMBS: usize = 50;
pub const LOG_LINES: usize = 40;
pub const LOG_LINE_CHARS: usize = 200;
/// The spool keeps at most this many reports, each a line of JSON of at most `MAX_EVENT_BYTES`.
const SPOOL_MAX_EVENTS: usize = 8;
const MAX_EVENT_BYTES: usize = 24 * 1024;
/// A command or action is a crumb at most once in this many milliseconds (a polling command must not flush the ring).
const CRUMB_REPEAT_MS: i64 = 30_000;
const COMPONENT: &str = "panel-shell";

// ---- words, frames, fingerprint (the same rules as the host's error_report.cpp) ----

/// A word for a crumb, a code or a component: lower case, characters outside [a-z0-9_.:+-] become `_`, at most `max` characters.
pub fn crumb_word(text: &str, max: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if out.len() >= max {
            break;
        }
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if matches!(c, '_' | '.' | ':' | '+' | '-') {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

/// A frame that may be sent: `module+0xoffset` or `file:line[:function]`, fixed characters, at most 120; otherwise `None`.
pub fn clean_frame(frame: &str) -> Option<String> {
    if frame.is_empty() || frame.len() > 120 {
        return None;
    }
    let ok = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | ':' | '+' | '<' | '>' | '~' | '$' | '@' | '!' | '-');
    frame.chars().all(ok).then(|| frame.to_string())
}

/// `src/foo.rs:12`: where a Rust source file is, without a build or home directory. The crate's own files start at `src/` (after `src-tauri/`), a dependency becomes
/// `dep/<crate>-<version>/src/file.rs`, the standard library `rustc/library/...`. Anything else has no usable location.
pub fn source_path(file: &str) -> Option<String> {
    let file = file.replace('\\', "/");
    let tail = if let Some(at) = file.find("/registry/src/") {
        // <cargo home>/registry/src/<index>-<hash>/<crate>-<version>/src/file.rs
        let rest = &file[at + "/registry/src/".len()..];
        let (_index, rest) = rest.split_once('/')?;
        format!("dep/{rest}")
    } else if let Some(at) = file.find("/rustc/") {
        // /rustc/<commit>/library/std/src/x.rs
        let rest = &file[at + "/rustc/".len()..];
        let (_commit, rest) = rest.split_once('/')?;
        format!("rustc/{rest}")
    } else if let Some(at) = file.rfind("src-tauri/src/") {
        file[at + "src-tauri/".len()..].to_string()
    } else if file.starts_with("src/") {
        file
    } else {
        return None;
    };
    Some(tail)
}

/// `src/foo.rs:12`, checked by `clean_frame`.
pub fn location_of(file: &str, line: u32) -> Option<String> {
    clean_frame(&format!("{}:{line}", source_path(file)?))
}

/// FNV-1a 64 over component, code and the first three frames (module only for a native frame, file and function without the line for a location), 16 hex digits: the host's `fingerprint_of`.
pub fn fingerprint_of(component: &str, code: &str, frames: &[String]) -> String {
    let mut h: u64 = 1469598103934665603;
    let mut mix = |text: &str| {
        for b in text.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(1099511628211);
        }
        h ^= 0xFF;
        h = h.wrapping_mul(1099511628211);
    };
    mix(component);
    mix(code);
    for frame in frames.iter().take(3) {
        let key = if let Some(plus) = frame.find("+0x") {
            frame[..plus].to_string()
        } else {
            let first = frame.find(':');
            let second = first.and_then(|f| frame[f + 1..].find(':').map(|s| s + f + 1));
            match first {
                Some(f) => format!("{}{}", &frame[..f], second.map_or("", |s| &frame[s..])),
                None => frame.clone(),
            }
        };
        mix(&key);
    }
    format!("{h:016x}")
}

// ---- breadcrumbs: the shell's last actions ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crumb {
    Nav,
    Act,
    Set,
    Stream,
    Dev,
    State,
    Net,
}

impl Crumb {
    fn name(self) -> &'static str {
        match self {
            Crumb::Nav => "nav",
            Crumb::Act => "act",
            Crumb::Set => "set",
            Crumb::Stream => "stream",
            Crumb::Dev => "dev",
            Crumb::State => "state",
            Crumb::Net => "net",
        }
    }
}

/// A bounded ring of the last actions: `<seconds before>|<kind>|<word>`. Thread safe; a poisoned lock is ignored, a capture must never panic.
pub struct Breadcrumbs {
    capacity: usize,
    inner: Mutex<(VecDeque<(i64, Crumb, String)>, HashMap<String, i64>)>,
}

impl Breadcrumbs {
    pub fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), inner: Mutex::new((VecDeque::new(), HashMap::new())) }
    }

    /// `id` is reduced to a word; an id that is empty afterwards is not recorded. The same id again within `CRUMB_REPEAT_MS` is not recorded either.
    pub fn add(&self, kind: Crumb, id: &str, now_ms: i64) {
        let word = crumb_word(id, 48);
        if word.is_empty() {
            return;
        }
        let Ok(mut guard) = self.inner.lock() else { return };
        let key = format!("{}|{word}", kind.name());
        if guard.1.get(&key).is_some_and(|last| now_ms - last < CRUMB_REPEAT_MS) {
            return;
        }
        if guard.1.len() > 256 {
            guard.1.clear();
        }
        guard.1.insert(key, now_ms);
        guard.0.push_back((now_ms, kind, word));
        while guard.0.len() > self.capacity {
            guard.0.pop_front();
        }
    }

    /// Oldest first.
    pub fn snapshot(&self, now_ms: i64) -> Vec<String> {
        let Ok(guard) = self.inner.lock() else { return Vec::new() };
        guard.0.iter().map(|(ms, kind, word)| format!("{}|{}|{word}", ((now_ms - ms) / 1000).clamp(0, 99_999), kind.name())).collect()
    }
}

// ---- the report ----

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub code: String,
    pub severity: String,
    pub loc: String,
    pub frames: Vec<String>,
    pub build: String,
    pub crumbs: Vec<String>,
    pub log: Vec<String>,
}

impl Report {
    /// The event as the page hands it to its sender: `{kind, name, props}`. Everything in `props` is a fixed word, a number or a scrubbed line.
    pub fn event(&self) -> Value {
        let mut frames: Vec<String> = Vec::new();
        if !self.loc.is_empty() {
            frames.push(self.loc.clone());
        }
        frames.extend(self.frames.iter().filter(|f| **f != self.loc).cloned());
        frames.truncate(MAX_FRAMES);
        let code = crumb_word(&self.code, 64);
        let fingerprint = fingerprint_of(COMPONENT, &code, &frames);
        let mut props = json!({
            "component": COMPONENT,
            "lang": "rust",
            "fingerprint": fingerprint,
            "code": code,
            "severity": match self.severity.as_str() { "fatal" | "warning" | "info" => self.severity.as_str(), _ => "error" },
            "count": 1,
        });
        let map = props.as_object_mut().expect("object");
        if !self.loc.is_empty() {
            map.insert("loc".into(), json!(self.loc));
        }
        if !self.build.is_empty() {
            map.insert("build".into(), json!(crumb_word(&self.build, 40)));
        }
        if !frames.is_empty() {
            map.insert("frames".into(), json!(frames));
        }
        if !self.crumbs.is_empty() {
            let from = self.crumbs.len().saturating_sub(MAX_CRUMBS);
            map.insert("crumbs".into(), json!(self.crumbs[from..]));
        }
        if !self.log.is_empty() {
            let from = self.log.len().saturating_sub(LOG_LINES);
            let lines: Vec<String> = self.log[from..].iter().map(|l| l.chars().take(LOG_LINE_CHARS).collect()).collect();
            map.insert("log".into(), json!(lines));
        }
        json!({ "kind": "error", "name": "error_report", "props": props })
    }
}

// ---- the spool ----

/// Append `event` to `path` (one JSON object per line). Keeps the newest `SPOOL_MAX_EVENTS`; an event larger than `MAX_EVENT_BYTES` loses its log first, then is dropped.
pub fn spool_append(path: &std::path::Path, event: &Value) -> std::io::Result<()> {
    let mut line = event.to_string();
    if line.len() > MAX_EVENT_BYTES {
        let mut slim = event.clone();
        if let Some(props) = slim.get_mut("props").and_then(Value::as_object_mut) {
            props.remove("log");
        }
        line = slim.to_string();
        if line.len() > MAX_EVENT_BYTES {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut lines: Vec<String> = std::fs::read_to_string(path).map(|t| t.lines().map(str::to_string).collect()).unwrap_or_default();
    lines.push(line);
    let from = lines.len().saturating_sub(SPOOL_MAX_EVENTS);
    let mut body = lines[from..].join("\n");
    body.push('\n');
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)
}

/// Read and remove the spool. A line that is not a well-formed `error_report` event is dropped.
pub fn spool_drain(path: &std::path::Path) -> Vec<Value> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let _ = std::fs::remove_file(path);
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v.get("kind").and_then(Value::as_str) == Some("error") && v.get("name").and_then(Value::as_str) == Some("error_report") && v.get("props").is_some_and(Value::is_object))
        .take(SPOOL_MAX_EVENTS)
        .collect()
}

/// The report of a panic at `location` (`file`, `line`) on a thread called `thread`. The thread `main` is the shell itself: fatal; any other thread is an error the shell may survive.
pub fn panic_report(location: Option<(&str, u32)>, thread: &str, frames: Vec<String>, crumbs: Vec<String>, log: Vec<String>, build: &str) -> Report {
    Report {
        code: "panic".into(),
        severity: if thread == "main" { "fatal" } else { "error" }.into(),
        loc: location.and_then(|(f, l)| location_of(f, l)).unwrap_or_default(),
        frames,
        build: build.into(),
        crumbs,
        log,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_lower_case_and_bounded() {
        assert_eq!(crumb_word("My Password Is Hunter2!!", 48), "my_password_is_hunter2");
        assert_eq!(crumb_word(&"a".repeat(100), 48).len(), 48);
        assert_eq!(crumb_word("名字", 48), "");
        assert_eq!(crumb_word("a/b\\c", 48), "a_b_c");
    }

    #[test]
    fn a_source_location_never_holds_a_build_or_home_directory() {
        assert_eq!(location_of("src/clipboard.rs", 1196).as_deref(), Some("src/clipboard.rs:1196"));
        assert_eq!(location_of(r"C:\Users\Sky Hua\build\sunshine-control-panel\src-tauri\src\tray\notifications.rs", 502).as_deref(), Some("src/tray/notifications.rs:502"));
        assert_eq!(location_of("/home/dev/work/panel/src-tauri/src/main.rs", 7).as_deref(), Some("src/main.rs:7"));
        assert_eq!(
            location_of("/home/dev/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.0/src/runtime/task/mod.rs", 45).as_deref(),
            Some("dep/tokio-1.52.0/src/runtime/task/mod.rs:45")
        );
        assert_eq!(location_of("/rustc/0123abcd/library/std/src/panicking.rs", 90).as_deref(), Some("rustc/library/std/src/panicking.rs:90"));
        assert_eq!(location_of("/tmp/elsewhere/x.rs", 3), None);
        assert_eq!(location_of("build.rs", 3), None);
    }

    #[test]
    fn a_frame_is_module_plus_offset_or_a_location_of_fixed_characters() {
        assert_eq!(clean_frame("sunshine-gui.exe+0x1a2b3").as_deref(), Some("sunshine-gui.exe+0x1a2b3"));
        assert_eq!(clean_frame("src/a.rs:12").as_deref(), Some("src/a.rs:12"));
        assert_eq!(clean_frame("C:\\Users\\x"), None);
        assert_eq!(clean_frame("with space"), None);
        assert_eq!(clean_frame(&"a".repeat(121)), None);
        assert_eq!(clean_frame("名字+0x1"), None);
    }

    #[test]
    fn the_fingerprint_is_the_hosts_and_ignores_offsets_and_line_numbers() {
        let a = fingerprint_of("panel-shell", "panic", &["src/a.rs:12".into(), "sunshine-gui.exe+0x100".into()]);
        let b = fingerprint_of("panel-shell", "panic", &["src/a.rs:99".into(), "sunshine-gui.exe+0x777".into()]);
        assert_eq!(a, b, "a line number and an offset move with every build");
        assert_ne!(a, fingerprint_of("panel-shell", "panic", &["src/b.rs:12".into(), "sunshine-gui.exe+0x100".into()]));
        assert_eq!(a.len(), 16);
        // the values the host's C++ telemetry::fingerprint_of (src/telemetry/error_report.cpp) gives for the same input: one algorithm in two languages
        assert_eq!(a, "027fe2f9e2fdcceb");
        assert_eq!(fingerprint_of("host", "E_X", &[]), "649e89205c3a1443");
        let four: Vec<String> = ["src/clipboard.rs:1196", "sunshine-gui.exe+0x4f10", "ntdll.dll+0x7a1", "x.dll+0x1"].map(String::from).to_vec();
        assert_eq!(fingerprint_of("panel-shell", "panic", &four), "1a4f883857d98b45", "only the first three frames count");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn the_ring_keeps_the_last_fifty_and_drops_a_repeat_of_the_same_action() {
        let ring = Breadcrumbs::new(MAX_CRUMBS);
        for i in 0..60 {
            ring.add(Crumb::Act, &format!("tile.{i}"), 1000 * i64::from(i));
        }
        ring.add(Crumb::Act, "tile.59", 61_000); // within 30 s of the same crumb: not again
        ring.add(Crumb::Set, "Encoder: NVENC!", 61_000);
        ring.add(Crumb::Nav, "", 61_000);
        let shot = ring.snapshot(65_000);
        assert_eq!(shot.len(), 50);
        assert_eq!(shot.last().map(String::as_str), Some("4|set|encoder:_nvenc"));
        assert_eq!(shot.first().map(String::as_str), Some("54|act|tile.11"));
        let shape = regex::Regex::new(r"^\d{1,5}\|(?:nav|act|set|stream|dev|state|net)\|[a-z0-9_.:+-]{1,48}$").unwrap();
        assert!(shot.iter().all(|c| shape.is_match(c)), "{shot:?}");
    }

    #[test]
    fn a_poll_that_repeats_is_one_crumb_until_thirty_seconds_pass() {
        let ring = Breadcrumbs::new(MAX_CRUMBS);
        for s in 0..20 {
            ring.add(Crumb::Act, "get_perf", s * 1000);
        }
        assert_eq!(ring.snapshot(20_000).len(), 1);
        ring.add(Crumb::Act, "get_perf", 31_000);
        assert_eq!(ring.snapshot(31_000).len(), 2);
    }

    fn sample() -> Report {
        Report {
            code: "panic".into(),
            severity: "fatal".into(),
            loc: "src/clipboard.rs:1196".into(),
            frames: vec!["sunshine-gui.exe+0x4f10".into(), "src/clipboard.rs:1196".into(), "ntdll.dll+0x7a1".into()],
            build: "9a80a0be44b4".into(),
            crumbs: vec!["12|nav|home".into(), "3|act|save_settings".into()],
            log: vec!["[2026-11-15 10:00:00.000] ERROR clipboard: agent died".into()],
        }
    }

    #[test]
    fn the_event_has_the_shape_the_host_and_the_worker_take() {
        let e = sample().event();
        assert_eq!(e["kind"], "error");
        assert_eq!(e["name"], "error_report");
        let p = &e["props"];
        assert_eq!(p["component"], "panel-shell");
        assert_eq!(p["lang"], "rust");
        assert_eq!(p["code"], "panic");
        assert_eq!(p["severity"], "fatal");
        assert_eq!(p["loc"], "src/clipboard.rs:1196");
        assert_eq!(p["frames"][0], "src/clipboard.rs:1196", "the location comes first and is not repeated");
        assert_eq!(p["frames"].as_array().unwrap().len(), 3);
        assert_eq!(p["crumbs"].as_array().unwrap().len(), 2);
        assert_eq!(p["log"][0], "[2026-11-15 10:00:00.000] ERROR clipboard: agent died");
        // what the Worker accepts: FRAME_RE and CRUMB_RE
        let frame = regex::Regex::new(r"^[A-Za-z0-9_./:+<>~$@!-]{1,120}$").unwrap();
        for f in p["frames"].as_array().unwrap() {
            assert!(frame.is_match(f.as_str().unwrap()), "{f}");
        }
        assert_eq!(p["fingerprint"].as_str().unwrap().len(), 16);
    }

    #[test]
    fn an_unknown_severity_is_error_and_the_lists_are_bounded() {
        let mut r = sample();
        r.severity = "catastrophic".into();
        r.frames = (0..100).map(|i| format!("m.dll+0x{i:x}")).collect();
        r.crumbs = (0..80).map(|i| format!("{i}|act|a{i}")).collect();
        r.log = (0..90).map(|i| format!("line {i} {}", "x".repeat(300))).collect();
        let p = &r.event()["props"];
        assert_eq!(p["severity"], "error");
        assert!(p["frames"].as_array().unwrap().len() <= MAX_FRAMES);
        assert_eq!(p["crumbs"].as_array().unwrap().len(), MAX_CRUMBS);
        assert_eq!(p["log"].as_array().unwrap().len(), LOG_LINES);
        assert!(p["log"].as_array().unwrap().iter().all(|l| l.as_str().unwrap().chars().count() <= LOG_LINE_CHARS));
    }

    #[test]
    fn a_thread_called_main_is_fatal_another_is_an_error() {
        let main = panic_report(Some(("src/main.rs", 3)), "main", vec![], vec![], vec![], "b");
        let other = panic_report(Some(("src/main.rs", 3)), "worker-1", vec![], vec![], vec![], "b");
        assert_eq!(main.severity, "fatal");
        assert_eq!(other.severity, "error");
        assert_eq!(main.loc, "src/main.rs:3");
    }

    #[test]
    fn the_spool_keeps_the_newest_eight_and_drain_empties_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs").join("error-spool.jsonl");
        for i in 0..11 {
            let mut r = sample();
            r.loc = format!("src/a.rs:{i}");
            spool_append(&path, &r.event()).unwrap();
        }
        let drained = spool_drain(&path);
        assert_eq!(drained.len(), 8);
        assert_eq!(drained.last().unwrap()["props"]["loc"], "src/a.rs:10");
        assert_eq!(drained.first().unwrap()["props"]["loc"], "src/a.rs:3");
        assert!(!path.exists());
        assert!(spool_drain(&path).is_empty());
    }

    #[test]
    fn a_damaged_or_foreign_spool_line_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, format!("not json\n{{\"kind\":\"usage\",\"name\":\"app_start\",\"props\":{{}}}}\n{}\n", sample().event())).unwrap();
        assert_eq!(spool_drain(&path).len(), 1);
    }

    #[test]
    fn a_huge_event_loses_its_log_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        let mut r = sample();
        r.log = vec!["y".repeat(LOG_LINE_CHARS); LOG_LINES]; // 8 KiB, fits
        spool_append(&path, &r.event()).unwrap();
        let mut event = r.event();
        event["props"]["log"] = json!(vec!["z".repeat(30_000)]);
        spool_append(&path, &event).unwrap();
        let drained = spool_drain(&path);
        assert_eq!(drained.len(), 2);
        assert!(drained[1]["props"].get("log").is_none());
    }

    #[test]
    fn a_panic_report_never_holds_the_message() {
        // the hook reads only the location of the panic; the payload (which may carry a path, a name or a value) is never looked at
        let result = std::panic::catch_unwind(|| {
            panic!("secret C:\\Users\\Sky Hua\\x password=hunter2");
        });
        assert!(result.is_err());
        let report = panic_report(Some((file!(), line!())), "worker", vec![], vec![], vec![], "b");
        let text = report.event().to_string();
        assert!(!text.contains("hunter2") && !text.contains("Sky Hua"));
        assert_eq!(report.code, "panic");
    }
}
