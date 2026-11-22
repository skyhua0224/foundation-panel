//! What the last transport cleanup left behind, kept between runs of the panel: the error it ended with (so the
//! diagnostics can say what Windows answered) and the restart marker (a removal that Windows finishes at the next restart).
//!
//! The file belongs to the signed-in person. The elevated helper never writes it: it answers over the pipe and the
//! unelevated side keeps what it was told, because the helper may run as another account.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use super::nodes::{MarkerOp, RestartMarker};

const FILE: &str = "usbip-cleanup.txt";
/// The kept error is for the diagnostics, not for a log: it is cut here.
const MAX_ERROR_CHARS: usize = 1500;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Journal {
    pub marker: Option<RestartMarker>,
    pub last_error: String,
    /// When `last_error` was kept (seconds since the Unix epoch); 0 when there is none.
    pub error_at: u64,
    pub last_install_error: String,
    /// When `last_install_error` was kept (seconds since the Unix epoch); 0 when there is none.
    pub install_error_at: u64,
}

fn path() -> Option<std::path::PathBuf> {
    crate::test_isolation::data_local_dir().map(|dir| dir.join("Sunshine GUI").join(FILE))
}

fn clean_line(text: &str) -> String {
    text.chars()
        .map(|character| if character.is_control() { ' ' } else { character })
        .take(MAX_ERROR_CHARS)
        .collect()
}

/// Line 1: the marker (`written TAB resumed TAB op`) or empty. Line 2: the last cleanup error, one line.
/// Line 3: when it was kept. Line 4: the last install error, one line. Line 5: when it was kept.
pub(super) fn encode(journal: &Journal) -> String {
    let marker = journal
        .marker
        .as_ref()
        .map(|marker| marker.encode().trim_end().to_string())
        .unwrap_or_default();
    let error = clean_line(&journal.last_error);
    let install_error = clean_line(&journal.last_install_error);
    format!(
        "{marker}\n{error}\n{}\n{install_error}\n{}\n",
        journal.error_at, journal.install_error_at
    )
}

pub(super) fn decode(text: &str) -> Journal {
    let mut lines = text.lines();
    let marker = lines.next().and_then(RestartMarker::decode);
    let last_error = lines.next().unwrap_or_default().trim().to_string();
    let error_at = lines.next().and_then(|line| line.trim().parse().ok()).unwrap_or(0);
    let last_install_error = lines.next().unwrap_or_default().trim().to_string();
    let install_error_at = lines.next().and_then(|line| line.trim().parse().ok()).unwrap_or(0);
    Journal {
        marker,
        last_error,
        error_at,
        last_install_error,
        install_error_at,
    }
}

pub(super) fn read() -> Journal {
    path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| decode(&text))
        .unwrap_or_default()
}

/// Best effort: a journal that cannot be written only means the next start knows less.
pub(super) fn write(journal: &Journal) {
    let Some(path) = path() else { return };
    if *journal == Journal::default() {
        let _ = std::fs::remove_file(path);
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temporary = path.with_extension("txt.tmp");
    if std::fs::write(&temporary, encode(journal)).is_ok()
        && std::fs::rename(&temporary, &path).is_err()
    {
        let _ = std::fs::remove_file(&temporary);
    }
}

pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// When this Windows session started (seconds since the Unix epoch): now minus the uptime.
#[cfg(target_os = "windows")]
pub(super) fn boot_secs() -> u64 {
    let uptime = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() } / 1000;
    now_secs().saturating_sub(uptime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_round_trips_and_keeps_one_line_of_error() {
        let journal = Journal {
            marker: Some(RestartMarker {
                written: 42,
                resumed: 1,
                op: MarkerOp::RemoveStale,
            }),
            last_error: "USBIP-CLEAN-008: in use [node=ROOT\\USB\\0000 cr=23]".to_string(),
            error_at: 99,
            last_install_error: "USBIP-SETUP-016: blocked [0xe0000247]".to_string(),
            install_error_at: 123,
        };
        assert_eq!(decode(&encode(&journal)), journal);
        // 3-line journals written by older builds decode with empty install error fields.
        assert_eq!(
            decode("42\t1\tstale\nUSBIP-CLEAN-008: in use\n99\n"),
            Journal {
                marker: Some(RestartMarker {
                    written: 42,
                    resumed: 1,
                    op: MarkerOp::RemoveStale,
                }),
                last_error: "USBIP-CLEAN-008: in use".to_string(),
                error_at: 99,
                last_install_error: String::new(),
                install_error_at: 0,
            }
        );
        let noisy = Journal {
            marker: None,
            last_error: "first\nsecond\tthird".to_string(),
            error_at: 0,
            last_install_error: String::new(),
            install_error_at: 0,
        };
        assert_eq!(decode(&encode(&noisy)).last_error, "first second third");
        assert_eq!(decode(""), Journal::default());
        assert_eq!(decode("garbage"), Journal::default());
        let long = Journal {
            marker: None,
            last_error: "x".repeat(5000),
            error_at: 0,
            last_install_error: "y".repeat(5000),
            install_error_at: 0,
        };
        let decoded_long = decode(&encode(&long));
        assert_eq!(decoded_long.last_error.len(), MAX_ERROR_CHARS);
        assert_eq!(decoded_long.last_install_error.len(), MAX_ERROR_CHARS);
    }
}
