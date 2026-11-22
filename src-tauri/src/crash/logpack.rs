//! The session log that goes with a crash report: found by time, anonymised, compressed (the Rust side of the host's `src/crash/crash_log.cpp`).
//!
//! The panel keeps one log file between runs (`panel.log`, rotated to `panel.1.log` at 5 MB, UTF-8, one line per record starting with
//! `[YYYY-MM-DD HH:MM:SS.mmm]` in local time). A report is sent on a later start, so the log of the run that crashed is cut out of that file by
//! time: every line up to 30 seconds after the report, the last 2 MiB of them. The text is anonymised (`logscrub`) before anything is
//! compressed or leaves the computer. What goes up: the gzip file, and the first and the last 64 KiB of the same anonymised text as plain UTF-8,
//! which the service spot-checks for personal data and checks against the start of the gzip file.

use super::logscrub;
use chrono::{Local, NaiveDateTime, TimeZone};
use flate2::{write::GzEncoder, Compression};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Size of `log_head` and `log_tail`.
pub(crate) const EDGE_BYTES: usize = 64 * 1024;
/// What the service accepts.
pub(crate) const MAX_GZ_BYTES: usize = 2 * 1024 * 1024;
/// How much raw log is read from the end of one file, and how much is kept after the cut by time.
const READ_BYTES: u64 = 3 * 1024 * 1024;
const KEEP_BYTES: usize = 2 * 1024 * 1024;
/// A line written this long after the report belongs to the next run.
const SLACK_SECONDS: i64 = 30;
/// A single line is cut here, like the host does.
const MAX_LINE: usize = 600;

pub(crate) struct Packed {
    pub gz: Vec<u8>,
    pub head: String,
    pub tail: String,
    pub lines: usize,
    #[allow(dead_code)]
    pub truncated: bool,
}

pub(crate) fn gzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::with_capacity(data.len() / 4 + 64), Compression::best());
    encoder.write_all(data).ok()?;
    encoder.finish().ok()
}

/// Anonymises `raw`, compresses it and cuts the head and the tail. When the result is over `max_gz` the oldest lines are dropped until it fits.
/// None for an empty log.
pub(crate) fn pack(raw: &[u8]) -> Option<Packed> {
    pack_with(raw, MAX_GZ_BYTES)
}

pub(crate) fn pack_with(raw: &[u8], max_gz: usize) -> Option<Packed> {
    let scrubbed = logscrub::scrub(raw, 2_000_000, MAX_LINE);
    if scrubbed.text.trim().is_empty() {
        return None;
    }
    let mut truncated = scrubbed.truncated;
    let mut text = scrubbed.text;
    let mut gz = gzip(text.as_bytes())?;
    // over the limit: drop the oldest half of the lines until it fits
    for _ in 0..12 {
        if gz.len() <= max_gz || text.len() <= 1024 {
            break;
        }
        let mut half = text.len() / 2;
        while !text.is_char_boundary(half) {
            half += 1;
        }
        let from = text[half..].find('\n').map_or(half, |i| half + i + 1);
        text.drain(..from);
        truncated = true;
        gz = gzip(text.as_bytes())?;
    }
    if gz.is_empty() || gz.len() > max_gz {
        return None;
    }
    let lines = text.matches('\n').count() + usize::from(!text.is_empty() && !text.ends_with('\n'));
    // head: the first EDGE_BYTES, whole characters; tail: the last EDGE_BYTES, from the start of a line
    let mut head_len = text.len().min(EDGE_BYTES);
    while !text.is_char_boundary(head_len) {
        head_len -= 1;
    }
    let head = text[..head_len].to_string();
    let tail = if text.len() > EDGE_BYTES {
        let mut from = text.len() - EDGE_BYTES;
        while !text.is_char_boundary(from) {
            from += 1;
        }
        from = text[from..].find('\n').map_or(from, |i| from + i + 1);
        text[from..].to_string()
    } else {
        text.clone()
    };
    Some(Packed { gz, head, tail, lines, truncated })
}

/// The unix time of a log line `[YYYY-MM-DD HH:MM:SS.mmm] ...` (local time), or None for a line that carries none.
fn line_time(line: &[u8]) -> Option<i64> {
    if line.len() < 25 || line[0] != b'[' || line[24] != b']' {
        return None;
    }
    let stamp = std::str::from_utf8(&line[1..24]).ok()?;
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S%.3f").ok()?;
    Some(Local.from_local_datetime(&naive).earliest()?.timestamp())
}

/// The last `READ_BYTES` of a file, from the start of a line.
fn read_tail(path: &Path) -> Vec<u8> {
    let Ok(mut file) = std::fs::File::open(path) else { return Vec::new() };
    let len = file.metadata().map_or(0, |m| m.len());
    let from = len.saturating_sub(READ_BYTES);
    if file.seek(SeekFrom::Start(from)).is_err() {
        return Vec::new();
    }
    let mut data = Vec::with_capacity((len - from) as usize);
    if file.take(READ_BYTES).read_to_end(&mut data).is_err() {
        return Vec::new();
    }
    if from > 0 {
        let start = data.iter().position(|&b| b == b'\n').map_or(data.len(), |i| i + 1);
        data.drain(..start);
    }
    data
}

/// The files that make up the log, oldest first: `panel.1.log`, then `panel.log`.
fn candidates(log_file: &Path) -> Vec<PathBuf> {
    let older = log_file.with_extension("1.log");
    [older, log_file.to_path_buf()].into_iter().filter(|p| p.is_file()).collect()
}

/// The text of the run that wrote a report at `report_unix`: lines up to 30 seconds after it, the last 2 MiB. Empty when no line is that old.
pub(crate) fn collect(log_file: &Path, report_unix: i64) -> Vec<u8> {
    let cutoff = report_unix + SLACK_SECONDS;
    let mut out = Vec::new();
    let mut started = false;
    'files: for path in candidates(log_file) {
        let data = read_tail(&path);
        for line in data.split_inclusive(|&b| b == b'\n') {
            match line_time(line) {
                Some(t) if t > cutoff => break 'files,
                Some(_) => started = true,
                // a continuation line belongs to the record above it; before the first record there is nothing to belong to
                None if !started => continue,
                None => {}
            }
            out.extend_from_slice(line);
        }
    }
    if out.len() > KEEP_BYTES {
        let from = out.len() - KEEP_BYTES;
        let start = out[from..].iter().position(|&b| b == b'\n').map_or(out.len(), |i| from + i + 1);
        out.drain(..start);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;

    fn gunzip(bytes: &[u8]) -> String {
        let mut out = String::new();
        GzDecoder::new(bytes).read_to_string(&mut out).unwrap();
        out
    }

    fn stamp(unix: i64) -> String {
        Local.timestamp_opt(unix, 0).unwrap().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    fn line(unix: i64, text: &str) -> String {
        format!("[{}] [INFO] [main.rs:1] {text}\n", stamp(unix))
    }

    #[test]
    fn the_gzip_file_round_trips_and_starts_with_the_head() {
        let mut raw = String::new();
        for i in 0..4000 {
            raw.push_str(&line(1_700_000_000 + i, &format!("record {i} on C:\\Users\\Alice\\Documents")));
        }
        let packed = pack(raw.as_bytes()).unwrap();
        let text = gunzip(&packed.gz);
        assert!(!text.contains("Alice"));
        assert!(text.contains("record 3999"));
        assert!(text.starts_with(&packed.head));
        assert!(text.ends_with(&packed.tail));
        assert!(packed.head.len() <= EDGE_BYTES && packed.tail.len() <= EDGE_BYTES);
        assert!(packed.tail.starts_with('['), "the tail starts at a line start");
        assert_eq!(packed.lines, 4000);
        assert!(!packed.truncated);
    }

    #[test]
    fn an_empty_log_is_nothing() {
        assert!(pack(b"").is_none());
        assert!(pack(b" \n\t\n").is_none());
    }

    #[test]
    fn the_oldest_lines_go_first_when_it_does_not_fit() {
        let mut raw = String::new();
        // text that does not compress well: a xorshift sequence in hex
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        for i in 0..6000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            raw.push_str(&line(1_700_000_000 + i, &format!("{x:016x} payload {i}")));
        }
        let packed = pack_with(raw.as_bytes(), 40 * 1024).unwrap();
        assert!(packed.gz.len() <= 40 * 1024);
        assert!(packed.truncated);
        let text = gunzip(&packed.gz);
        assert!(text.contains("payload 5999"));
        assert!(!text.contains("payload 0\n"));
        assert!(text.starts_with('['), "the cut falls on a line start");
    }

    #[test]
    fn multi_byte_text_is_cut_on_characters() {
        let mut raw = String::new();
        for i in 0..6000 {
            raw.push_str(&line(1_700_000_000 + i, "串流开始，编码器初始化完成，分辨率 1920x1080"));
        }
        let packed = pack(raw.as_bytes()).unwrap();
        assert!(String::from_utf8(packed.head.clone().into_bytes()).is_ok());
        assert!(packed.head.chars().count() > 1000);
        assert!(gunzip(&packed.gz).starts_with(&packed.head));
    }

    #[test]
    fn the_log_is_cut_at_the_report_time_with_a_slack() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("panel.log");
        let mut text = String::new();
        let t0 = 1_700_000_000;
        text.push_str(&line(t0, "before"));
        text.push_str("  continuation of before\n");
        text.push_str(&line(t0 + 10, "at the report"));
        text.push_str(&line(t0 + 25, "just after (inside the slack)"));
        text.push_str(&line(t0 + 90, "next run"));
        text.push_str("  continuation of next run\n");
        std::fs::write(&log, text).unwrap();
        let got = String::from_utf8(collect(&log, t0 + 10)).unwrap();
        assert!(got.contains("before") && got.contains("continuation of before") && got.contains("at the report") && got.contains("inside the slack"));
        assert!(!got.contains("next run"), "{got}");
        // a report older than everything in the file has no log
        assert!(collect(&log, t0 - 1000).is_empty());
    }

    #[test]
    fn the_rotated_copy_comes_first() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("panel.log");
        let t0 = 1_700_000_000;
        std::fs::write(dir.path().join("panel.1.log"), line(t0, "old file")).unwrap();
        std::fs::write(&log, line(t0 + 5, "new file")).unwrap();
        let got = String::from_utf8(collect(&log, t0 + 6)).unwrap();
        assert!(got.find("old file").unwrap() < got.find("new file").unwrap());
    }

    #[test]
    fn a_missing_log_is_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(collect(&dir.path().join("panel.log"), 1_700_000_000).is_empty());
    }

    #[test]
    fn only_the_last_two_mebibytes_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("panel.log");
        let t0 = 1_700_000_000;
        let mut text = String::new();
        let mut i = 0;
        while text.len() < 2_800_000 {
            text.push_str(&line(t0, &format!("row {i} {}", "x".repeat(60))));
            i += 1;
        }
        std::fs::write(&log, &text).unwrap();
        let got = collect(&log, t0);
        assert!(got.len() <= KEEP_BYTES);
        assert!(got.starts_with(b"["));
        assert!(String::from_utf8(got).unwrap().contains(&format!("row {} ", i - 1)));
    }

    #[test]
    fn line_time_reads_only_the_panel_format() {
        let t0 = 1_700_000_000;
        assert_eq!(line_time(line(t0, "x").as_bytes()), Some(t0));
        assert_eq!(line_time(b"  continuation line that is long enough to pass"), None);
        assert_eq!(line_time(b"[not a stamp at all here....] x"), None);
    }
}
