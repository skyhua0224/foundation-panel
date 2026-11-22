//! The choice to leave GPU acceleration out of the panel's window, kept between runs.
//!
//! When the renderer of the window dies again and again, or the GPU process cannot stay up (`recover.rs`), the panel starts again with the
//! WebView2 browser flag `--disable-gpu` ("Disables GPU hardware acceleration. If a software renderer isn't in place, the GPU process doesn't
//! launch", Microsoft Learn, WebView2 browser flags). That is a flag Microsoft documents for diagnosing, not for production, so it is only
//! the last step, and the choice does not last: it is dropped after fourteen days and as soon as the WebView2 Runtime has been updated, because
//! a newer runtime (or a newer driver, which a fortnight usually covers) may have fixed the cause. Setting `SUNSHINE_PANEL_GPU=1` keeps GPU
//! acceleration on whatever the file says.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

/// The flag the next runs pass to WebView2.
pub(crate) const BROWSER_ARGUMENT: &str = "--disable-gpu";
/// Set by the process that switches, so the process it starts (and a failed write of the file) cannot switch again.
pub(crate) const SOFTWARE_ENV: &str = "SUNSHINE_PANEL_SOFTWARE_RENDER";
/// The person keeps GPU acceleration on.
pub(crate) const FORCE_GPU_ENV: &str = "SUNSHINE_PANEL_GPU";
const FILE: &str = "render-fallback.txt";
const LIFETIME_SECS: u64 = 14 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Marker {
    /// Seconds since the Unix epoch.
    pub written: u64,
    /// The WebView2 Runtime version at that time (empty when unknown).
    pub runtime: String,
    /// What made the panel switch: the failure kinds and exit codes, for the log.
    pub reason: String,
}

impl Marker {
    pub(crate) fn encode(&self) -> String {
        let clean = |text: &str| text.replace(['\t', '\n', '\r'], " ");
        format!("{}\t{}\t{}\n", self.written, clean(&self.runtime), clean(&self.reason))
    }

    pub(crate) fn decode(text: &str) -> Option<Self> {
        let mut parts = text.trim_end().splitn(3, '\t');
        Some(Self {
            written: parts.next()?.parse().ok()?,
            runtime: parts.next()?.to_string(),
            reason: parts.next().unwrap_or_default().to_string(),
        })
    }

    /// Still in force: not older than the lifetime and written under the runtime that runs now. A runtime that cannot be read on either side
    /// (empty) is not a change: only a version that differs ends the choice.
    pub(crate) fn in_force(&self, now: u64, runtime: &str) -> bool {
        let age = now.saturating_sub(self.written);
        let runtime_changed = !self.runtime.is_empty() && !runtime.is_empty() && self.runtime != runtime;
        age < LIFETIME_SECS && !runtime_changed
    }
}

fn path() -> Option<std::path::PathBuf> {
    crate::test_isolation::data_local_dir().map(|dir| dir.join("Sunshine GUI").join(FILE))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// How this run renders: the person keeps the GPU on (`FORCE_GPU_ENV`), the process was told to leave it out (`SOFTWARE_ENV`), or
/// the file says so and is still in force; otherwise with the GPU.
pub(crate) fn render_mode() -> super::recover::RenderMode {
    use super::recover::RenderMode;
    if std::env::var_os(FORCE_GPU_ENV).is_some_and(|value| value == "1") {
        return RenderMode::GpuForced;
    }
    if std::env::var_os(SOFTWARE_ENV).is_some_and(|value| value == "1") {
        return RenderMode::Software;
    }
    let runtime = runtime_version();
    let in_force = path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| Marker::decode(&text))
        .is_some_and(|marker| marker.in_force(now_secs(), &runtime));
    if in_force { RenderMode::Software } else { RenderMode::Gpu }
}

/// Whether this run leaves GPU acceleration out.
pub(crate) fn software_rendering() -> bool {
    render_mode() == super::recover::RenderMode::Software
}

/// Keeps the choice for the next runs and tells this process's children (the restart). Best effort: a file that cannot be written still
/// holds for the restart through the environment.
pub(crate) fn remember(reason: &str) {
    // SAFETY: called from the WebView2 event on the UI thread right before the panel restarts; nothing else reads this variable meanwhile.
    unsafe { std::env::set_var(SOFTWARE_ENV, "1") };
    let Some(path) = path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let marker = Marker {
        written: now_secs(),
        runtime: runtime_version(),
        reason: reason.to_string(),
    };
    let _ = std::fs::write(path, marker.encode());
}

#[cfg(windows)]
fn runtime_version() -> String {
    crate::webview_runtime::installed_version().unwrap_or_default()
}

#[cfg(not(windows))]
fn runtime_version() -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(written: u64, runtime: &str) -> Marker {
        Marker {
            written,
            runtime: runtime.to_string(),
            reason: "webview2:render_exited 0xc0000005; webview2:render_exited 0xc0000005".to_string(),
        }
    }

    #[test]
    fn the_marker_round_trips_with_a_reason_that_has_spaces() {
        let kept = marker(1_000, "128.0.2739.42");
        assert_eq!(Marker::decode(&kept.encode()), Some(kept));
        assert_eq!(Marker::decode(""), None);
        assert_eq!(Marker::decode("soon\t1\tx"), None);
        // a reason with a tab or a line break stays one line
        let noisy = Marker {
            reason: "a\tb\nc".to_string(),
            ..marker(5, "1.2.3.4")
        };
        assert_eq!(Marker::decode(&noisy.encode()).unwrap().reason, "a b c");
    }

    #[test]
    fn the_choice_ends_after_two_weeks_and_with_a_new_runtime() {
        let kept = marker(1_000, "128.0.2739.42");
        assert!(kept.in_force(1_000 + 60, "128.0.2739.42"));
        assert!(kept.in_force(1_000 + LIFETIME_SECS - 1, "128.0.2739.42"));
        assert!(!kept.in_force(1_000 + LIFETIME_SECS, "128.0.2739.42"));
        assert!(!kept.in_force(1_000 + 60, "129.0.1.1"));
        // a runtime that cannot be read is not a change
        assert!(kept.in_force(1_000 + 60, ""));
        assert!(marker(1_000, "").in_force(1_000 + 60, "129.0.1.1"));
    }
}
