//! Which crash reports the person has already been asked about.
//!
//! The window asks once, after a start that finds a report of an earlier run, whether to tell the developers. A report that could not be uploaded (no network, 429, 5xx, no install id) stays in the
//! pending folder, so the next start finds it again: without a record the person would be asked about the same crash at every start. The record is one file in the pending folder, a line per report
//! (the file stem); it holds only the reports that are still pending, so it never grows, and a new report is asked about once.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) const FILE: &str = "recovery-asked.txt";

/// The stems (file names without the extension) of the reports in a list of pending reports.
pub(crate) fn stems(pending: &[super::meta::Pending]) -> Vec<String> {
    pending.iter().filter_map(|p| p.meta.file_stem().and_then(|s| s.to_str()).map(str::to_owned)).collect()
}

pub(crate) fn path_in(pending_dir: &Path) -> PathBuf {
    pending_dir.join(FILE)
}

pub(crate) fn load(file: &Path) -> HashSet<String> {
    std::fs::read_to_string(file).map(|t| t.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect()).unwrap_or_default()
}

/// What a start finds: whether a report exists that the person has not answered the question about, and the reports (stems) that are pending now.
pub(crate) fn unasked_at_start(pending_dir: &Path) -> (bool, Vec<String>) {
    let left = stems(&super::meta::list_pending(pending_dir));
    (has_new(&left, &load(&path_in(pending_dir))), left)
}

/// The person answered (sent feedback or let it go): from now on these reports do not bring the question back.
pub(crate) fn answered(pending_dir: &Path, stems: &[String]) {
    record(&path_in(pending_dir), stems);
}

/// Whether a report exists that the person has not been asked about.
pub(crate) fn has_new(stems: &[String], asked: &HashSet<String>) -> bool {
    stems.iter().any(|s| !asked.contains(s))
}

/// Records `stems` as asked (replacing the record: only what is still pending is kept).
pub(crate) fn record(file: &Path, stems: &[String]) {
    let mut text = stems.join("\n");
    text.push('\n');
    let temp = file.with_extension("txt.tmp");
    if std::fs::write(&temp, text).is_ok() {
        let _ = std::fs::rename(&temp, file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(dir: &Path, names: &[&str]) -> Vec<super::super::meta::Pending> {
        for n in names {
            std::fs::write(dir.join(format!("{n}.json")), "{}").unwrap();
        }
        super::super::meta::list_pending(dir)
    }

    #[test]
    fn a_report_is_asked_about_once_even_when_it_stays_pending_for_several_starts() {
        let dir = tempfile::tempdir().unwrap();
        let list = pending(dir.path(), &["1700000000-panel-panic-p00000001"]);
        let file = path_in(dir.path());
        let s = stems(&list);
        assert!(has_new(&s, &load(&file)), "first start: the report is new");
        record(&file, &s);
        assert!(!has_new(&s, &load(&file)), "the next start: the upload failed, the report is still there, nobody asks again");
    }

    #[test]
    fn leaving_before_the_answer_asks_again_at_the_next_start_and_an_answer_stops_it() {
        let dir = tempfile::tempdir().unwrap();
        pending(dir.path(), &["1700000000-panel-panic-p00000001"]);
        let (ask, left) = unasked_at_start(dir.path());
        assert!(ask, "the first start after the failure asks");
        // the program ends before the person answers: nothing was written, so the next start asks again (the report may depend on this answer to be sent)
        let (again, _) = unasked_at_start(dir.path());
        assert!(again, "no answer, no record: asked again");
        answered(dir.path(), &left);
        let (after, _) = unasked_at_start(dir.path());
        assert!(!after, "after an answer a restart does not ask again, although the report is still pending");
    }

    #[test]
    fn a_new_report_is_asked_about_and_the_record_keeps_only_what_is_pending() {
        let dir = tempfile::tempdir().unwrap();
        let file = path_in(dir.path());
        record(&file, &["old-report".to_string()]);
        let list = pending(dir.path(), &["new-report"]);
        let s = stems(&list);
        assert!(has_new(&s, &load(&file)), "another report came since");
        record(&file, &s);
        let asked = load(&file);
        assert!(asked.contains("new-report") && !asked.contains("old-report"), "what was uploaded and is gone leaves the record");
    }

    #[test]
    fn the_record_file_is_not_a_report_and_nothing_pending_means_nothing_to_ask() {
        let dir = tempfile::tempdir().unwrap();
        let file = path_in(dir.path());
        record(&file, &[]);
        assert!(file.is_file());
        assert!(super::super::meta::list_pending(dir.path()).is_empty(), "the record is not listed as a pending report");
        assert!(!has_new(&[], &load(&file)));
        assert!(load(&dir.path().join("missing.txt")).is_empty());
    }
}
