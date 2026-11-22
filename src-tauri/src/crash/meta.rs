//! Crash report meta (telemetry v1 `POST /v1/crash`): frames, signature, the JSON, consent and the pending folder.
//!
//! Pure and platform-free so the unit tests run anywhere. The renderer works on fixed buffers and allocates nothing: the native crash
//! handler calls it while the process may have a broken heap. Same rules as the host's `src/crash/crash_core.cpp`; the tests share the
//! signature vector with the host's so the two stay one definition.

use core::fmt::Write;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const MAX_FRAMES: usize = 64;
pub const SIGNATURE_FRAMES: usize = 8;
pub const MAX_PENDING: usize = 20;
pub const MAX_DUMP_BYTES: u64 = 20 * 1024 * 1024;
pub const META_BUFFER: usize = 16 * 1024;
const MODULE_CAP: usize = 64;

/// One stack frame: the module's file name (no directory: a user name can be in a path) and the address minus the module's base.
#[derive(Clone, Copy)]
pub struct Frame {
    module: [u8; MODULE_CAP],
    len: u8,
    pub offset: u64,
}

impl Frame {
    pub fn new(module: &str, offset: u64) -> Frame {
        let mut frame = Frame { module: [0; MODULE_CAP], len: 0, offset };
        frame.set_module(module.as_bytes());
        frame
    }

    pub const EMPTY: Frame = Frame { module: [0; MODULE_CAP], len: 0, offset: 0 };

    pub fn set_module(&mut self, bytes: &[u8]) {
        let mut n = 0;
        for &b in bytes.iter().take(MODULE_CAP - 1) {
            self.module[n] = if b.is_ascii_graphic() || b == b' ' { b } else { b'?' };
            n += 1;
        }
        self.len = n as u8;
    }

    pub fn module(&self) -> &str {
        core::str::from_utf8(&self.module[..self.len as usize]).unwrap_or("")
    }
}

/// A fixed-capacity text buffer: `write!` into it never allocates and never overruns (it remembers that it was full).
pub struct Buf<const N: usize> {
    bytes: [u8; N],
    len: usize,
    pub full: bool,
}

impl<const N: usize> Buf<N> {
    pub fn new() -> Self {
        Buf { bytes: [0; N], len: 0, full: false }
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }
}

impl<const N: usize> Default for Buf<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for Buf<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if self.len + s.len() > N {
            self.full = true;
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
}

/// Runtime and system modules whose frames sit on top of every abort or throw and say nothing about the caller.
pub fn is_runtime_module(name: &str) -> bool {
    const NAMES: [&str; 11] = [
        "ntdll.dll", "kernelbase.dll", "kernel32.dll", "ucrtbase.dll", "msvcrt.dll", "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll",
        "libstdc++-6.dll", "libgcc_s_seh-1.dll", "libwinpthread-1.dll",
    ];
    NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// Index of the first frame that is not a runtime/system frame (0 when every frame is one).
pub fn first_caller_frame(frames: &[Frame]) -> usize {
    frames.iter().position(|f| !is_runtime_module(f.module())).unwrap_or(0)
}

/// SHA-256 over the first 8 frames from the first caller frame on, each "<module lower case>+0x<offset lower hex>" joined by '\n'; 64 hex digits.
pub fn signature(frames: &[Frame]) -> Buf<64> {
    let mut text = Buf::<{ SIGNATURE_FRAMES * (MODULE_CAP + 24) }>::new();
    let from = first_caller_frame(frames);
    for (n, frame) in frames.iter().skip(from).take(SIGNATURE_FRAMES).enumerate() {
        if n > 0 {
            let _ = text.write_char('\n');
        }
        for c in frame.module().chars() {
            let _ = text.write_char(c.to_ascii_lowercase());
        }
        let _ = write!(text, "+0x{:x}", frame.offset);
    }
    let digest = Sha256::digest(text.as_bytes());
    let mut out = Buf::<64>::new();
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// ISO-8601 UTC, "2026-11-12T09:30:00Z", no allocation (civil-from-days).
pub fn iso_utc(unix_seconds: i64) -> Buf<24> {
    let mut days = unix_seconds.div_euclid(86400);
    let rem = unix_seconds.rem_euclid(86400);
    days += 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year + i64::from(month <= 2);
    let mut out = Buf::<24>::new();
    let _ = write!(
        out,
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    );
    out
}

pub struct MetaInput<'a> {
    pub product: &'a str,
    pub version: &'a str,
    pub os_version: &'a str,
    pub install_id: &'a str,
    pub kind: &'a str,
    pub code: &'a str,
    pub time: i64,
    pub uptime_s: u64,
    pub frames: &'a [Frame],
}

fn json_str<const N: usize>(out: &mut Buf<N>, s: &str) {
    let _ = out.write_char('"');
    for c in s.chars() {
        match c {
            '"' => {
                let _ = out.write_str("\\\"");
            }
            '\\' => {
                let _ = out.write_str("\\\\");
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => {
                let _ = out.write_char(c);
            }
        }
    }
    let _ = out.write_char('"');
}

/// The meta JSON of the contract. `None` when it does not fit the buffer.
pub fn render_meta(input: &MetaInput) -> Option<Buf<META_BUFFER>> {
    let mut out = Buf::<META_BUFFER>::new();
    let frames = &input.frames[..input.frames.len().min(MAX_FRAMES)];
    let caller = first_caller_frame(frames);
    let (module, offset) = frames.get(caller).map_or(("", 0), |f| (f.module(), f.offset));
    let _ = out.write_str("{\"v\":1,\"product\":");
    json_str(&mut out, input.product);
    if !input.install_id.is_empty() {
        let _ = out.write_str(",\"install_id\":");
        json_str(&mut out, input.install_id);
    }
    let _ = out.write_str(",\"version\":");
    json_str(&mut out, input.version);
    let _ = out.write_str(",\"os_version\":");
    json_str(&mut out, input.os_version);
    let _ = out.write_str(",\"time\":");
    json_str(&mut out, iso_utc(input.time).as_str());
    let _ = write!(out, ",\"uptime_s\":{},\"exception_code\":", input.uptime_s);
    json_str(&mut out, input.code);
    let _ = out.write_str(",\"module\":");
    json_str(&mut out, module);
    let _ = write!(out, ",\"module_offset\":\"0x{offset:x}\",\"frames\":[");
    for (i, frame) in frames.iter().enumerate() {
        if i > 0 {
            let _ = out.write_char(',');
        }
        let _ = out.write_str("{\"module\":");
        json_str(&mut out, frame.module());
        let _ = write!(out, ",\"offset\":\"0x{:x}\"}}", frame.offset);
    }
    let _ = write!(out, "],\"signature\":\"{}\",\"kind\":", signature(frames).as_str());
    json_str(&mut out, input.kind);
    let _ = out.write_char('}');
    if out.full { None } else { Some(out) }
}

/// The value of `key` in a sunshine.conf text (the last one wins), empty when absent.
pub fn conf_value(text: &str, key: &str) -> String {
    let mut found = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        if k.trim() != key {
            continue;
        }
        let v = v.trim();
        let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
        found = v.to_string();
    }
    found
}

/// Error reports (so crash reports) may leave the computer unless the person turned them off: `telemetry_errors` decides when present,
/// else an old `telemetry_consent = off` still means off, else on (foundation-ui/telemetry's rule).
pub fn errors_enabled(conf_text: &str) -> bool {
    let off = |v: &str| matches!(v.to_ascii_lowercase().as_str(), "off" | "false" | "0");
    let own = conf_value(conf_text, "telemetry_errors");
    if !own.is_empty() {
        return !off(&own);
    }
    conf_value(conf_text, "telemetry_consent") != "off"
}

/// <base>/AlkaidLab/Sunshine Panel/crashes/pending
pub fn pending_dir_in(base: &Path) -> PathBuf {
    base.join("AlkaidLab").join("Sunshine Panel").join("crashes").join("pending")
}

pub const DEFAULT_ENDPOINT: &str = "https://api.alkaidlab.com";

/// env SUNSHINE_TELEMETRY_ENDPOINT, else the config key telemetry_endpoint, else the default.
pub fn endpoint(conf_text: &str) -> String {
    if let Ok(v) = std::env::var("SUNSHINE_TELEMETRY_ENDPOINT") {
        if !v.is_empty() {
            return v;
        }
    }
    let v = conf_value(conf_text, "telemetry_endpoint");
    if v.is_empty() { DEFAULT_ENDPOINT.to_string() } else { v }
}

/// The one-line UUID the host keeps in `<config>/telemetry/install_id`; read-only here (the host creates it when consent allows).
pub fn read_install_id(config_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(config_dir.join("telemetry").join("install_id")).ok()?;
    let id = text.lines().next()?.trim().to_string();
    let valid = id.len() == 36
        && id.chars().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() });
    valid.then_some(id)
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub meta: PathBuf,
    pub dump: Option<PathBuf>,
    pub modified: SystemTime,
}

/// Reports in a pending folder, oldest first. A stray .dmp without a .json is removed.
pub fn list_pending(dir: &Path) -> Vec<Pending> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut reports = Vec::new();
    let mut dumps = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        match path.extension().and_then(|e| e.to_str()) {
            Some("dmp") => dumps.push(path),
            Some("json") => {
                let modified = entry.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
                let dump = path.with_extension("dmp");
                reports.push(Pending { dump: dump.is_file().then_some(dump), meta: path, modified });
            }
            _ => {}
        }
    }
    for dump in dumps {
        if !dump.with_extension("json").exists() {
            let _ = std::fs::remove_file(dump);
        }
    }
    reports.sort_by(|a, b| a.modified.cmp(&b.modified).then_with(|| a.meta.cmp(&b.meta)));
    reports
}

fn remove(report: &Pending) {
    let _ = std::fs::remove_file(&report.meta);
    if let Some(dump) = &report.dump {
        let _ = std::fs::remove_file(dump);
    }
}

/// Keeps the newest `keep` reports and drops the rest and everything older than `max_age_days` (0: no age limit); returns how many went.
pub fn trim_pending(dir: &Path, keep: usize, max_age_days: u64) -> usize {
    let mut removed = 0;
    let now = SystemTime::now();
    let mut alive = Vec::new();
    for report in list_pending(dir) {
        let old = max_age_days > 0
            && now.duration_since(report.modified).map_or(false, |age| age > Duration::from_secs(max_age_days * 86400));
        if old {
            remove(&report);
            removed += 1;
        } else {
            alive.push(report);
        }
    }
    while alive.len() > keep {
        remove(&alive.remove(0));
        removed += 1;
    }
    removed
}

pub fn clear_pending(dir: &Path) {
    trim_pending(dir, 0, 0);
}

/// "<unix seconds>-<product>-<kind>-p<pid hex>", no allocation.
pub fn stem(unix_seconds: i64, product: &str, kind: &str, pid: u32) -> Buf<112> {
    let mut out = Buf::<112>::new();
    let clean = |s: &str, out: &mut Buf<112>| {
        for c in s.chars() {
            let _ = out.write_char(if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' });
        }
    };
    let _ = write!(out, "{unix_seconds}-");
    clean(product, &mut out);
    let _ = out.write_char('-');
    clean(kind, &mut out);
    let _ = write!(out, "-p{pid:08x}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_hex(text: &str) -> String {
        Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn frames(items: &[(&str, u64)]) -> Vec<Frame> {
        items.iter().map(|(m, o)| Frame::new(m, *o)).collect()
    }

    #[test]
    fn signature_matches_the_host_vector() {
        // the same input as the host's CrashSignature.MatchesAnIndependentComputation
        let f = frames(&[("Sunshine.EXE", 0x1234), ("sunshine.exe", 0x77)]);
        assert_eq!(signature(&f).as_str(), digest_hex("sunshine.exe+0x1234\nsunshine.exe+0x77"));
    }

    #[test]
    fn runtime_frames_on_top_do_not_count() {
        let a = frames(&[("ntdll.dll", 0x10), ("ucrtbase.dll", 0x20), ("sunshine-gui.exe", 0x500), ("sunshine-gui.exe", 0x600)]);
        let b = frames(&[("kernelbase.dll", 0x99), ("sunshine-gui.exe", 0x500), ("sunshine-gui.exe", 0x600)]);
        assert_eq!(signature(&a).as_str(), signature(&b).as_str());
        assert_eq!(first_caller_frame(&a), 2);
        let all = frames(&[("ntdll.dll", 1), ("kernel32.dll", 2)]);
        assert_eq!(first_caller_frame(&all), 0);
        assert_eq!(signature(&all).as_str().len(), 64);
    }

    #[test]
    fn only_the_first_eight_frames_count() {
        let mut many: Vec<Frame> = (0..12).map(|i| Frame::new("app.exe", 0x100 + i)).collect();
        let a = signature(&many).as_str().to_string();
        many[10].offset = 0xdead;
        assert_eq!(a, signature(&many).as_str());
        many[7].offset = 0xbeef;
        assert_ne!(a, signature(&many).as_str());
    }

    #[test]
    fn time_formats_utc() {
        assert_eq!(iso_utc(0).as_str(), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_700_000_000).as_str(), "2023-11-14T22:13:20Z");
        assert_eq!(iso_utc(1_709_164_800).as_str(), "2024-02-29T00:00:00Z");
        assert_eq!(iso_utc(1_793_000_000).as_str(), "2026-10-26T07:33:20Z");
    }

    #[test]
    fn meta_has_the_contract_fields() {
        let f = frames(&[("ntdll.dll", 0x10), ("sunshine-gui.exe", 0xabc), ("libfoo.dll", 1)]);
        let meta = render_meta(&MetaInput {
            product: "panel",
            version: "2026.1112.1.杂鱼",
            os_version: "Windows 11 10.0.26100",
            install_id: "11111111-2222-4333-8444-555555555555",
            kind: "panic",
            code: "panic",
            time: 1_700_000_000,
            uptime_s: 42,
            frames: &f,
        })
        .unwrap();
        let j: serde_json::Value = serde_json::from_str(meta.as_str()).unwrap();
        assert_eq!(j["v"], 1);
        assert_eq!(j["product"], "panel");
        assert_eq!(j["install_id"], "11111111-2222-4333-8444-555555555555");
        assert_eq!(j["version"], "2026.1112.1.杂鱼");
        assert_eq!(j["time"], "2023-11-14T22:13:20Z");
        assert_eq!(j["uptime_s"], 42);
        assert_eq!(j["module"], "sunshine-gui.exe");
        assert_eq!(j["module_offset"], "0xabc");
        assert_eq!(j["frames"].as_array().unwrap().len(), 3);
        assert_eq!(j["frames"][0]["offset"], "0x10");
        assert_eq!(j["kind"], "panic");
        assert_eq!(j["signature"], signature(&f).as_str());
    }

    #[test]
    fn meta_escapes_and_bounds_frames() {
        let f: Vec<Frame> = (0..100).map(|_| Frame::new("a\"b\\c.dll", 1)).collect();
        let meta = render_meta(&MetaInput { product: "panel", version: "", os_version: "", install_id: "", kind: "crash", code: "x\ny", time: 0, uptime_s: 0, frames: &f }).unwrap();
        let j: serde_json::Value = serde_json::from_str(meta.as_str()).unwrap();
        assert_eq!(j["frames"].as_array().unwrap().len(), MAX_FRAMES);
        assert_eq!(j["frames"][0]["module"], "a\"b\\c.dll");
        assert_eq!(j["exception_code"], "x\ny");
        assert!(j.get("install_id").is_none());
    }

    #[test]
    fn consent_follows_telemetry_errors() {
        assert!(errors_enabled(""));
        assert!(errors_enabled("port = 47989\n"));
        assert!(!errors_enabled("telemetry_errors = off\n"));
        assert!(!errors_enabled("telemetry_errors=false"));
        assert!(errors_enabled("telemetry_errors = on\n"));
        assert!(!errors_enabled("telemetry_errors = on\ntelemetry_errors = off\n"));
        assert!(!errors_enabled("# telemetry_errors = on\ntelemetry_errors = \"off\"\r\n"));
        // an old no stays no until errors are turned on
        assert!(!errors_enabled("telemetry_consent = off\n"));
        assert!(errors_enabled("telemetry_consent = off\ntelemetry_errors = on\n"));
        assert!(errors_enabled("telemetry_consent = ask\n"));
    }

    #[test]
    fn endpoint_default_and_config_key() {
        // SUNSHINE_TELEMETRY_ENDPOINT is not set in the test environment
        if std::env::var("SUNSHINE_TELEMETRY_ENDPOINT").is_err() {
            assert_eq!(endpoint(""), DEFAULT_ENDPOINT);
            assert_eq!(endpoint("telemetry_endpoint = http://127.0.0.1:9/\n"), "http://127.0.0.1:9/");
        }
    }

    #[test]
    fn stems_are_file_safe() {
        assert_eq!(stem(1700000000, "panel", "panic", 0x1f).as_str(), "1700000000-panel-panic-p0000001f");
        assert_eq!(stem(1, "a/b", "c d", 1).as_str(), "1-a_b-c_d-p00000001");
    }

    #[test]
    fn a_report_left_behind_by_an_earlier_run_is_listed_and_nothing_else_is() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_pending(dir.path()).is_empty(), "an empty folder: the earlier run did not fail");
        std::fs::write(dir.path().join("stray.dmp"), "d").unwrap();
        std::fs::write(dir.path().join("note.txt"), "keep").unwrap();
        assert!(list_pending(dir.path()).is_empty(), "a dump without a meta and other files are not a report");
        std::fs::write(dir.path().join("1700000000-panel-panic-p0000001f.json"), "{}").unwrap();
        assert_eq!(list_pending(dir.path()).len(), 1);
    }

    #[test]
    fn pending_keeps_the_newest_twenty_and_drops_strays() {
        let dir = tempfile::tempdir().unwrap();
        let base = SystemTime::now() - Duration::from_secs(3600);
        for i in 0..25u64 {
            let name = format!("r{}", 100 + i);
            std::fs::write(dir.path().join(format!("{name}.json")), "{}").unwrap();
            std::fs::write(dir.path().join(format!("{name}.dmp")), "d").unwrap();
            let f = std::fs::File::options().write(true).open(dir.path().join(format!("{name}.json"))).unwrap();
            f.set_modified(base + Duration::from_secs(i)).unwrap();
        }
        std::fs::write(dir.path().join("stray.dmp"), "d").unwrap();
        std::fs::write(dir.path().join("note.txt"), "keep").unwrap();
        assert_eq!(list_pending(dir.path()).len(), 25);
        assert!(!dir.path().join("stray.dmp").exists());
        assert_eq!(trim_pending(dir.path(), MAX_PENDING, 30), 5);
        let left = list_pending(dir.path());
        assert_eq!(left.len(), MAX_PENDING);
        assert_eq!(left[0].meta.file_name().unwrap(), "r105.json");
        assert!(!dir.path().join("r100.dmp").exists());
        assert!(dir.path().join("note.txt").exists());
    }

    #[test]
    fn old_reports_age_out() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.json"), "{}").unwrap();
        std::fs::write(dir.path().join("new.json"), "{}").unwrap();
        let f = std::fs::File::options().write(true).open(dir.path().join("old.json")).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(40 * 86400)).unwrap();
        assert_eq!(trim_pending(dir.path(), MAX_PENDING, 30), 1);
        assert!(!dir.path().join("old.json").exists());
        assert!(dir.path().join("new.json").exists());
    }

    #[test]
    fn install_id_is_read_only_and_validated() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_install_id(dir.path()), None);
        std::fs::create_dir_all(dir.path().join("telemetry")).unwrap();
        std::fs::write(dir.path().join("telemetry").join("install_id"), "not-a-uuid\n").unwrap();
        assert_eq!(read_install_id(dir.path()), None);
        std::fs::write(dir.path().join("telemetry").join("install_id"), "11111111-2222-4333-8444-555555555555\r\n").unwrap();
        assert_eq!(read_install_id(dir.path()).as_deref(), Some("11111111-2222-4333-8444-555555555555"));
    }
}
