//! What the panel knows about the Sunshine host from the operating system, as plain data: which process is the host, what the service manager says, and whether the panel
//! itself has asked for a stop. No window system in it, so it is tested on any machine (host_facts.rs reads the system and hands the data over).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    /// creation time, Unix milliseconds
    pub created_ms: u64,
}

/// The host process: an image called exactly `sunshine.exe`. Not the service wrapper (`sunshinesvc.exe`), not the panel (`sunshine-gui.exe`) and not a tool that has the word in its
/// name. If several remain (a start that overlaps the end of the old one), the oldest.
pub fn pick_host(processes: &[Proc]) -> Option<&Proc> {
    processes.iter().filter(|p| p.name.eq_ignore_ascii_case("sunshine.exe")).min_by_key(|p| p.created_ms)
}

/// The state of a service as the service manager reports it (SERVICE_STATUS.dwCurrentState).
pub fn service_label(state: u32) -> &'static str {
    match state {
        1 => "stopped",
        2 => "start_pending",
        3 => "stop_pending",
        4 => "running",
        5 => "start_pending", // continue pending
        6 => "stop_pending",  // pause pending
        7 => "paused",
        _ => "unknown",
    }
}

/// The panel (a restart it did, an update, the Web UI's restart button) has asked for a stop and the time it gave has not run out.
pub fn expected_now(until_ms: u64, now_ms: u64) -> bool {
    until_ms != 0 && now_ms < until_ms
}

/// A new window of expectation never shortens one that is open.
pub fn extended(current_until_ms: u64, now_ms: u64, for_ms: u64) -> u64 {
    current_until_ms.max(now_ms.saturating_add(for_ms))
}

/// Paths of the Web UI through the proxy that stop or restart the host.
pub fn is_stop_request(method: &str, path: &str) -> bool {
    // 恢复默认设置 and 恢复上一次备份 restart the host as well: the panel expects that stop, it is not a crash
    method.eq_ignore_ascii_case("POST") && matches!(path, "/api/restart" | "/api/quit" | "/api/config/reset" | "/api/config/reset/restore")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, name: &str, created_ms: u64) -> Proc {
        Proc { pid, name: name.to_string(), created_ms }
    }

    #[test]
    fn the_host_is_sunshine_exe_and_nothing_else_with_the_word_in_it() {
        let list = [proc(9952, "sunshinesvc.exe", 100), proc(44156, "sunshine-gui.exe", 120), proc(7, "sunshine.exe.bak", 90), proc(10056, "Sunshine.exe", 200)];
        assert_eq!(pick_host(&list).map(|p| p.pid), Some(10056));
        assert_eq!(pick_host(&list[..3]), None, "the wrapper and the panel alone are no host");
        assert_eq!(pick_host(&[]), None);
    }

    #[test]
    fn of_two_hosts_the_older_one_is_the_host() {
        let list = [proc(20404, "sunshine.exe", 5_000), proc(10056, "sunshine.exe", 1_000)];
        assert_eq!(pick_host(&list).map(|p| p.pid), Some(10056));
    }

    #[test]
    fn the_states_of_the_service_manager() {
        assert_eq!(service_label(4), "running");
        assert_eq!(service_label(1), "stopped");
        assert_eq!(service_label(3), "stop_pending");
        assert_eq!(service_label(2), "start_pending");
        assert_eq!(service_label(7), "paused");
        assert_eq!(service_label(99), "unknown");
    }

    #[test]
    fn a_stop_the_panel_asked_for_is_expected_until_its_window_runs_out() {
        assert!(!expected_now(0, 1_000), "nothing was asked");
        assert!(expected_now(181_000, 1_000));
        assert!(!expected_now(181_000, 181_000));
        let first = extended(0, 1_000, 180_000);
        assert_eq!(first, 181_000);
        assert_eq!(extended(first, 2_000, 10_000), 181_000, "a shorter ask does not shorten the window");
        assert_eq!(extended(first, 100_000, 180_000), 280_000, "a later one extends it");
    }

    #[test]
    fn the_requests_of_the_web_ui_that_stop_the_host() {
        assert!(is_stop_request("POST", "/api/restart"));
        assert!(is_stop_request("post", "/api/quit"));
        assert!(!is_stop_request("GET", "/api/restart"));
        assert!(!is_stop_request("POST", "/api/pairing/pending"));
        assert!(!is_stop_request("POST", "/api/restart/other"));
        // the reset and the restore of the host settings restart it; the list of backups does not
        assert!(is_stop_request("POST", "/api/config/reset"));
        assert!(is_stop_request("post", "/api/config/reset/restore"));
        assert!(!is_stop_request("GET", "/api/config/reset"));
        assert!(!is_stop_request("POST", "/api/config"));
    }
}
