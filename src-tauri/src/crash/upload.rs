//! Upload of the pending crash reports: `POST <endpoint>/v1/crash`, multipart "meta" JSON + optional "dump".
//!
//! A report is deleted after a 2xx and after a 4xx the server will never accept (not 408/429); a network failure or 5xx leaves it for the
//! next pass. Never blocks the panel: it runs on its own thread with its own small runtime.

use super::logpack::{self, Packed};
use super::meta;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// A report the server will not take later either.
pub(crate) fn settled(status: u16) -> bool {
    (200..300).contains(&status) || ((400..500).contains(&status) && status != 408 && status != 429)
}

/// Failed passes in a row, and how long the service asked the uploader to wait (seconds, 0 = no hint).
static FAILURES: AtomicU32 = AtomicU32::new(0);
static WAIT_HINT_S: AtomicU64 = AtomicU64::new(0);

/// How long the uploader waits before the next pass: what the service asked for (Retry-After), else after a failure 15 minutes doubling up to
/// 4 hours (the numbers Crashpad uses for its retries), else the regular period.
pub(crate) fn next_wait(regular: Duration) -> Duration {
    let hint = WAIT_HINT_S.load(Ordering::Relaxed);
    if hint > 0 {
        return Duration::from_secs(hint);
    }
    regular
}

/// Seconds to wait after the `failures`-th failure in a row: Retry-After (30 s to 4 h), else for no answer or a server error 15 minutes doubling up to 4 hours.
fn wait_seconds(failures: u32, status: u16, retry_after: Option<u64>) -> u64 {
    match retry_after {
        Some(s) if s > 0 => s.clamp(30, 4 * 3600),
        _ if status == 0 || status >= 500 => (4 * 3600).min(15 * 60u64 << failures.saturating_sub(1).min(5)),
        _ => 0,
    }
}

fn note_failure(status: u16, retry_after: Option<u64>) {
    let failures = FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
    WAIT_HINT_S.store(wait_seconds(failures, status, retry_after), Ordering::Relaxed);
}

fn note_success() {
    FAILURES.store(0, Ordering::Relaxed);
    WAIT_HINT_S.store(0, Ordering::Relaxed);
}

/// The body of a multipart/form-data request with the "meta" part and, when there are some, the "dump" part and the session log
/// (`log` = gzip, `log_head` and `log_tail` = the first and the last 64 KiB as plain text, `log_lines`).
pub(crate) fn multipart_body(boundary: &str, meta_json: &[u8], dump: Option<&[u8]>, log: Option<&Packed>) -> Vec<u8> {
    let mut body = Vec::with_capacity(meta_json.len() + dump.map_or(0, <[u8]>::len) + log.map_or(0, |l| l.gz.len() + l.head.len() + l.tail.len()) + 1024);
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"meta\"\r\nContent-Type: application/json\r\n\r\n").as_bytes());
    body.extend_from_slice(meta_json);
    body.extend_from_slice(b"\r\n");
    if let Some(dump) = dump {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"dump\"; filename=\"crash.dmp\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(dump);
        body.extend_from_slice(b"\r\n");
    }
    if let Some(log) = log {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"log\"; filename=\"panel.log.gz\"\r\nContent-Type: application/gzip\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(&log.gz);
        body.extend_from_slice(b"\r\n");
        for (name, text) in [("log_head", &log.head), ("log_tail", &log.tail)] {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n").as_bytes());
            body.extend_from_slice(text.as_bytes());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"log_lines\"\r\n\r\n{}\r\n", log.lines).as_bytes());
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// The unix time a report was written: the `time` of its meta, else the file's last write.
fn report_time(meta_json: &[u8], modified: std::time::SystemTime) -> i64 {
    serde_json::from_slice::<serde_json::Value>(meta_json)
        .ok()
        .and_then(|v| v.get("time").and_then(|t| t.as_str()).and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| t.timestamp()))
        .unwrap_or_else(|| modified.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
}

/// `install_id` goes into the meta when the writer left it out.
pub(crate) fn meta_with_identity(text: &[u8], install_id: &str) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(text).ok()?;
    let object = value.as_object_mut()?;
    if !object.contains_key("install_id") && !install_id.is_empty() {
        object.insert("install_id".into(), install_id.into());
    }
    object.entry("v").or_insert(1.into());
    serde_json::to_vec(&value).ok()
}

/// One pass over the pending folder; returns how many reports were delivered or rejected for good.
pub(crate) async fn upload_pending_once(dir: &Path, endpoint: &str, install_id: &str) -> usize {
    upload_pending_with(dir, endpoint, install_id, Some(&crate::logger::panel_log_path())).await
}

/// `log_file` is the panel's own log; a report goes up with the part of it that was written until the report. A log that cannot be made never
/// holds a report back.
pub(crate) async fn upload_pending_with(dir: &Path, endpoint: &str, install_id: &str, log_file: Option<&Path>) -> usize {
    meta::trim_pending(dir, meta::MAX_PENDING, 30);
    let Ok(client) = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("FoundationSunshinePanel-crash/1")
        .build()
    else {
        return 0;
    };
    let url = format!("{}/v1/crash", endpoint.trim_end_matches('/'));
    let mut done = 0;
    let own_tag = format!("-p{:08x}.", std::process::id());
    for report in meta::list_pending(dir) {
        // one this process is still writing
        let young = report.modified.elapsed().map_or(true, |age| age < Duration::from_secs(10));
        if young && report.meta.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(&own_tag)) {
            continue;
        }
        let Ok(text) = std::fs::read(&report.meta) else { continue };
        let Some(body_meta) = meta_with_identity(&text, install_id) else {
            let _ = std::fs::remove_file(&report.meta);
            if let Some(d) = &report.dump {
                let _ = std::fs::remove_file(d);
            }
            continue;
        };
        let dump = report
            .dump
            .as_ref()
            .filter(|d| std::fs::metadata(d).map_or(false, |m| m.len() <= meta::MAX_DUMP_BYTES))
            .and_then(|d| std::fs::read(d).ok())
            .map(|mut bytes| {
                // the dump type keeps heap and data out; this blanks the account and computer names that a path or a stack word still carries
                super::redact::redact_private_names(&mut bytes, &super::redact::private_names());
                bytes
            });
        let log = log_file.and_then(|file| {
            let raw = logpack::collect(file, report_time(&text, report.modified));
            if raw.is_empty() { None } else { logpack::pack(&raw) }
        });
        let boundary = format!("alk-crash-{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        let body = multipart_body(&boundary, &body_meta, dump.as_deref(), log.as_ref());
        let (status, retry_after) = match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
            .body(body)
            .send()
            .await
        {
            Ok(response) => {
                let wait = response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
                (response.status().as_u16(), wait)
            }
            Err(_) => (0, None),
        };
        // 429 (a limit; wait Retry-After), 503 (storage failed, nothing counted), a server error or no answer: keep everything and stop this pass
        if status == 429 || status == 503 || status == 0 || status >= 500 {
            note_failure(status, retry_after);
            break;
        }
        note_success();
        if !settled(status) {
            continue;
        }
        let _ = std::fs::remove_file(&report.meta);
        if let Some(d) = &report.dump {
            let _ = std::fs::remove_file(d);
        }
        done += 1;
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[test]
    fn what_the_server_cannot_take_yet_stays() {
        for status in [0u16, 408, 429, 500, 502, 503] {
            assert!(!settled(status), "{status}");
        }
        for status in [200u16, 204, 400, 404, 413, 422] {
            assert!(settled(status), "{status}");
        }
    }

    #[test]
    fn multipart_has_the_named_parts() {
        let body = multipart_body("B", b"{\"v\":1}", Some(b"MDMPxx"), None);
        let text = String::from_utf8_lossy(&body);
        assert!(text.starts_with("--B\r\n"));
        assert!(text.contains("name=\"meta\""));
        assert!(text.contains("name=\"dump\""));
        assert!(text.contains("MDMPxx"));
        assert!(text.ends_with("--B--\r\n"));
        assert!(!String::from_utf8_lossy(&multipart_body("B", b"{}", None, None)).contains("name=\"dump\""));
    }

    #[test]
    fn identity_is_added_once() {
        let out = meta_with_identity(b"{\"product\":\"panel\"}", "id-1").unwrap();
        let j: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(j["install_id"], "id-1");
        assert_eq!(j["v"], 1);
        let kept = meta_with_identity(b"{\"install_id\":\"mine\"}", "id-1").unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&kept).unwrap()["install_id"], "mine");
        assert!(meta_with_identity(b"not json", "id-1").is_none());
    }

    /// A one-shot HTTP server that answers every request with  and keeps the bodies.
    fn mock(status: u16) -> (String, Arc<Mutex<Vec<Vec<u8>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let kept = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut data = Vec::new();
                let mut chunk = [0u8; 8192];
                let mut header_end = None;
                let mut length = 0usize;
                loop {
                    let Ok(n) = stream.read(&mut chunk) else { break };
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&chunk[..n]);
                    if header_end.is_none() {
                        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = Some(at + 4);
                            let head = String::from_utf8_lossy(&data[..at]).to_ascii_lowercase();
                            length = head.lines().find_map(|l| l.strip_prefix("content-length:")).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                        }
                    }
                    if header_end.is_some_and(|h| data.len() >= h + length) {
                        break;
                    }
                }
                if let Some(h) = header_end {
                    kept.lock().unwrap().push(data[h..].to_vec());
                }
                let _ = stream.write_all(format!("HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
            }
        });
        (format!("http://127.0.0.1:{port}"), seen)
    }

    fn pending_report(dir: &Path, name: &str, dump: bool) {
        let frames = [meta::Frame::new("sunshine-gui.exe", 0x1234)];
        let json = meta::render_meta(&meta::MetaInput { product: "panel", version: "1", os_version: "w", install_id: "", kind: "panic", code: "panic", time: 1_700_000_000, uptime_s: 1, frames: &frames }).unwrap();
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.json")), json.as_bytes()).unwrap();
        if dump {
            std::fs::write(dir.join(format!("{name}.dmp")), b"MDMP-body").unwrap();
        }
        let f = std::fs::File::options().write(true).open(dir.join(format!("{name}.json"))).unwrap();
        f.set_modified(std::time::SystemTime::now() - Duration::from_secs(300)).unwrap();
    }

    fn run<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    /// One pass without a session log.
    async fn once(dir: &Path, url: &str, id: &str) -> usize {
        upload_pending_with(dir, url, id, None).await
    }

    #[test]
    fn delivers_and_deletes_after_2xx() {
        let (url, seen) = mock(200);
        let dir = tempfile::tempdir().unwrap();
        pending_report(dir.path(), "a", true);
        pending_report(dir.path(), "b", false);
        assert_eq!(run(once(dir.path(), &url, "11111111-2222-4333-8444-555555555555")), 2);
        assert!(meta::list_pending(dir.path()).is_empty());
        let bodies = seen.lock().unwrap();
        assert_eq!(bodies.len(), 2);
        let first = String::from_utf8_lossy(&bodies[0]);
        assert!(first.contains("name=\"meta\"") && first.contains("name=\"dump\"") && first.contains("MDMP-body"));
        assert!(first.contains("\"install_id\":\"11111111-2222-4333-8444-555555555555\""));
        assert!(!String::from_utf8_lossy(&bodies[1]).contains("name=\"dump\""));
    }

    #[test]
    fn a_server_error_keeps_the_report_and_a_rejection_drops_it() {
        let dir = tempfile::tempdir().unwrap();
        pending_report(dir.path(), "a", true);
        let (url, _) = mock(503);
        assert_eq!(run(once(dir.path(), &url, "x")), 0);
        assert_eq!(meta::list_pending(dir.path()).len(), 1);
        let (url, _) = mock(400);
        assert_eq!(run(once(dir.path(), &url, "x")), 1);
        assert!(meta::list_pending(dir.path()).is_empty());
    }

    #[test]
    fn no_server_keeps_the_report() {
        let dir = tempfile::tempdir().unwrap();
        pending_report(dir.path(), "a", true);
        assert_eq!(run(once(dir.path(), "http://127.0.0.1:1", "x")), 0);
        assert_eq!(meta::list_pending(dir.path()).len(), 1);
    }

    #[test]
    fn the_log_parts_follow_the_service_contract() {
        let packed = logpack::pack(b"[2023-11-14 22:13:20.000] [INFO] [main.rs:1] hello\n").unwrap();
        let body = multipart_body("B", b"{}", None, Some(&packed));
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"log\"; filename=\"panel.log.gz\"") && text.contains("Content-Type: application/gzip"));
        assert!(text.contains("name=\"log_head\"") && text.contains("name=\"log_tail\""));
        assert!(text.contains("name=\"log_lines\"\r\n\r\n1\r\n"));
        assert!(text.ends_with("--B--\r\n"));
    }

    #[test]
    fn a_report_goes_up_with_the_log_written_until_it() {
        let (url, seen) = mock(200);
        let dir = tempfile::tempdir().unwrap();
        pending_report(dir.path(), "a", false);
        // the meta of pending_report is dated 1_700_000_000 (2023-11-14T22:13:20Z)
        let log = dir.path().join("logs").join("panel.log");
        std::fs::create_dir_all(log.parent().unwrap()).unwrap();
        let stamp = |t: i64| chrono::Local.timestamp_opt(t, 0).unwrap().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        std::fs::write(&log, format!("[{}] [INFO] [a.rs:1] before the crash, user C:\\Users\\Alice\\x\n[{}] [INFO] [a.rs:2] next run\n", stamp(1_700_000_000 - 5), stamp(1_700_000_000 + 600))).unwrap();
        assert_eq!(run(upload_pending_with(dir.path(), &url, "x", Some(&log))), 1);
        let bodies = seen.lock().unwrap();
        let sent = String::from_utf8_lossy(&bodies[0]);
        assert!(sent.contains("name=\"log\"") && sent.contains("before the crash"));
        assert!(!sent.contains("Alice") && !sent.contains("next run"));
    }

    #[test]
    fn retry_after_and_the_backoff_set_the_wait() {
        assert_eq!(wait_seconds(1, 429, Some(120)), 120);
        assert_eq!(wait_seconds(1, 429, Some(5)), 30);
        assert_eq!(wait_seconds(1, 429, Some(999_999)), 4 * 3600);
        assert_eq!(wait_seconds(1, 503, None), 15 * 60);
        assert_eq!(wait_seconds(2, 0, None), 30 * 60);
        assert_eq!(wait_seconds(3, 500, None), 60 * 60);
        assert_eq!(wait_seconds(9, 502, None), 4 * 3600);
        assert_eq!(wait_seconds(1, 429, None), 0);
        assert_eq!(wait_seconds(1, 503, Some(0)), 15 * 60);
    }
}
