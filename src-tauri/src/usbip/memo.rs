//! Answers of the two probes that start PowerShell (the signature of the registered driver and the host helper's
//! `status`), kept so that a status read does not start a process every time (a probe takes about twelve seconds on a
//! busy machine, and the page waits for it).
//!
//! Each answer is stored with a fingerprint of everything it was derived from (the driver file's size and time, the
//! registered installations, the helper script's size and time). A read with the same fingerprint reuses the answer (a day at most); a
//! different one probes again at once, so an installation changed outside the panel is still noticed. The answer is
//! also written to a small file next to the panel's settings, so the first read after the panel starts does not start
//! PowerShell either. A command that changes the installation calls `forget_all()` through
//! `status::fresh()`, before and after it runs. Failed or unreadable probes are never kept.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::time::Duration;

use crate::probe_cache::ProbeCache;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The store is one file for every answer: a read-modify-write of it is one step at a time.
static STORE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// How old an answer may be, kept in the process or stored: a day. What it was derived from is in its fingerprint, so a changed
/// installation is noticed at once whatever the age; the age only bounds what the fingerprint cannot see. A probe takes seconds
/// (PowerShell) and the page that asks waits for it, so it is not repeated every few minutes.
const MEMO_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const STORED_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const STORE_FILE: &str = "probe-cache.txt";

/// Creation flags of a probe that nobody waits for on screen: no window, below normal priority.
#[cfg(target_os = "windows")]
pub(super) const BACKGROUND_PROBE_FLAGS: u32 = windows::Win32::System::Threading::CREATE_NO_WINDOW.0
    | windows::Win32::System::Threading::BELOW_NORMAL_PRIORITY_CLASS.0;

fn store_path() -> Option<std::path::PathBuf> {
    crate::test_isolation::config_dir().map(|dir| dir.join("Sunshine GUI").join(STORE_FILE))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// The fingerprint is kept as a hash: it only has to be equal or not.
fn digest(fingerprint: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    fingerprint.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// One line per answer: `name TAB digest TAB seconds TAB value` (a value has no line break or tab).
fn read_store() -> Vec<[String; 4]> {
    let Some(path) = store_path() else {
        return Vec::new();
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            Some([
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ])
        })
        .collect()
}

fn write_store(rows: &[[String; 4]]) {
    let Some(path) = store_path() else { return };
    let text: String = rows.iter().map(|row| row.join("\t") + "\n").collect();
    let temporary = path.with_extension("txt.tmp");
    // Best effort: a store that cannot be written only means the next start probes once more.
    if std::fs::write(&temporary, text).is_ok() && std::fs::rename(&temporary, &path).is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
}

fn recall(name: &str, fingerprint: &str) -> Option<String> {
    let wanted = digest(fingerprint);
    read_store().into_iter().find_map(|[row_name, row_digest, at, value]| {
        let age = now_secs().saturating_sub(at.parse().ok()?);
        (row_name == name && row_digest == wanted && age <= STORED_MAX_AGE_SECS).then_some(value)
    })
}

fn keep(name: &str, fingerprint: &str, value: &str) {
    let mut rows = read_store();
    rows.retain(|row| row[0] != name);
    rows.push([
        name.to_string(),
        digest(fingerprint),
        now_secs().to_string(),
        value.replace(['\t', '\r', '\n'], " "),
    ]);
    write_store(&rows);
}

fn forget(name: &str) {
    let mut rows = read_store();
    let before = rows.len();
    rows.retain(|row| row[0] != name);
    if rows.len() != before {
        write_store(&rows);
    }
}

/// Size and modification time of a file: changes when the file is replaced.
pub(super) fn file_identity(path: &std::path::Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |elapsed| elapsed.as_nanos());
            format!("{}:{}:{}", path.display(), meta.len(), modified)
        }
        Err(_) => format!("{}:absent", path.display()),
    }
}

/// A kept answer as text (the callers write and read their own short forms).
pub(super) struct Memo {
    name: &'static str,
    cache: ProbeCache<(String, String)>,
    /// Moves when the answer is dropped: a probe that started before cannot be kept after.
    generation: AtomicU64,
    /// The stored answer is read once, when the panel starts; later the five minutes end in a new probe.
    restored: AtomicBool,
}

impl Memo {
    pub(super) const fn new(name: &'static str) -> Self {
        Self {
            name,
            cache: ProbeCache::new(name, MEMO_TTL),
            generation: AtomicU64::new(0),
            restored: AtomicBool::new(false),
        }
    }

    pub(super) fn forget(&self) {
        // after an explicit drop the stored answer is never read again, whether or not the file could be changed
        self.restored.store(true, Ordering::Release);
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.cache.invalidate();
        let _step = STORE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        forget(self.name);
    }

    /// The answer for `fingerprint`: the kept one, the stored one (the first time only), or what `probe` returns now
    /// (`None` when the probe failed; nothing is kept then).
    pub(super) async fn get<F, Fut>(&self, fingerprint: &str, probe: F) -> Option<String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        for _ in 0..2 {
            let loaded = self
                .cache
                .get(|| async {
                    if !self.restored.swap(true, Ordering::AcqRel) {
                        if let Some(value) = recall(self.name, fingerprint) {
                            return Ok((fingerprint.to_string(), value));
                        }
                    }
                    let started = self.generation.load(Ordering::Acquire);
                    match probe().await {
                        Some(value) => {
                            let _step = STORE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                            // dropped while the probe ran: its answer is used once and not kept
                            if self.generation.load(Ordering::Acquire) == started {
                                keep(self.name, fingerprint, &value);
                            }
                            Ok((fingerprint.to_string(), value))
                        }
                        None => Err(String::new()),
                    }
                })
                .await;
            match loaded {
                Ok((kept_for, value)) if kept_for == fingerprint => return Some(value),
                // Kept for another fingerprint: what it was derived from has changed.
                Ok(_) => self.cache.invalidate(),
                Err(_) => return None,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn unique(prefix: &str) -> &'static str {
        // The store belongs to the real settings directory: a unique name per test keeps each test private.
        Box::leak(format!("{prefix}-{}-{}", std::process::id(), now_secs()).into_boxed_str())
    }

    #[test]
    fn the_same_fingerprint_probes_once_and_a_new_one_probes_again() {
        run(async {
            let memo: Memo = Memo::new(unique("memo-test-same"));
            let runs = AtomicUsize::new(0);
            let probe = || async {
                runs.fetch_add(1, Ordering::SeqCst);
                Some("7".to_string())
            };
            assert_eq!(memo.get("one", probe).await, Some("7".to_string()));
            assert_eq!(memo.get("one", probe).await, Some("7".to_string()));
            assert_eq!(runs.load(Ordering::SeqCst), 1);
            assert_eq!(memo.get("two", probe).await, Some("7".to_string()));
            assert_eq!(runs.load(Ordering::SeqCst), 2);
            memo.forget();
            assert_eq!(memo.get("two", probe).await, Some("7".to_string()));
            assert_eq!(runs.load(Ordering::SeqCst), 3);
            memo.forget();
        });
    }

    #[test]
    fn an_answer_dropped_before_the_first_read_is_not_restored_from_the_store() {
        run(async {
            let memo = Memo::new(unique("memo-test-dropped"));
            keep(memo.name, "fp", "old");
            memo.forget();
            // the store still has (or got back) an old answer: after the drop it must not be used
            keep(memo.name, "fp", "old");
            let answer = memo.get("fp", || async { Some("new".to_string()) }).await;
            assert_eq!(answer.as_deref(), Some("new"));
            memo.forget();
        });
    }

    #[test]
    fn a_failed_probe_is_not_kept() {
        run(async {
            let memo: Memo = Memo::new(unique("memo-test-failure"));
            assert_eq!(memo.get("f", || async { None }).await, None);
            assert_eq!(memo.get("f", || async { Some("1".to_string()) }).await, Some("1".to_string()));
            memo.forget();
        });
    }

    #[test]
    fn a_file_identity_changes_with_the_file() {
        let directory = std::env::temp_dir().join(format!("memo-identity-{}", now_secs()));
        std::fs::create_dir_all(&directory).unwrap();
        let file = directory.join("driver.sys");
        assert!(file_identity(&file).ends_with(":absent"));
        std::fs::write(&file, b"one").unwrap();
        let first = file_identity(&file);
        std::fs::write(&file, b"three").unwrap();
        assert_ne!(first, file_identity(&file));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
