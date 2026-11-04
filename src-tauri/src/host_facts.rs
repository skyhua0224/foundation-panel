//! The facts of the operating system about the Sunshine host that decide whether the panel may say 「Sunshine 意外停止」: the host process (id and start), the service manager's
//! state of the Sunshine service, and whether the panel has asked for a stop (a restart it did, an update, the Web UI's restart button through the proxy). A failed or slow HTTP call
//! says nothing about any of this (a pairing long poll that ran out its limit once made the panel announce a stop of a host that had run for hours). The decision is made in the
//! renderer (m3e status/hostLoss.js) on these facts; the pure parts are in host_facts_math.rs.
use crate::host_facts_math::{self, Proc};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static EXPECTED_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// The panel is about to stop or restart the host (or the Web UI does it through the proxy): a stop in the next `for_ms` is asked for, not a surprise.
pub fn expect_stop(for_ms: u64) {
    let until = host_facts_math::extended(EXPECTED_UNTIL_MS.load(Ordering::Relaxed), now_ms(), for_ms);
    EXPECTED_UNTIL_MS.fetch_max(until, Ordering::Relaxed);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFacts {
    pub running: bool,
    pub pid: u32,
    pub started_ms: u64,
    /// 'running' | 'stopped' | 'start_pending' | 'stop_pending' | 'paused' | 'missing' | 'unknown'
    pub service: String,
    /// the process id of the service wrapper (sunshinesvc) that runs the host, 0 when there is none: a service that was stopped and started is another process, a host that crashed and was started
    /// again by the wrapper is the same one (the renderer tells a restart of the service from a crash with it, however late it looks)
    pub service_pid: u32,
    pub expected_stop: bool,
}

#[cfg(target_os = "windows")]
fn service_state() -> (String, u32) {
    use windows::Win32::System::Services::{OpenSCManagerW, OpenServiceW, QueryServiceStatusEx, SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_QUERY_STATUS, SERVICE_STATUS_PROCESS};
    use windows::core::{Owned, PCWSTR, w};
    unsafe {
        let Ok(manager) = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) else {
            return ("unknown".to_string(), 0);
        };
        let manager = Owned::new(manager);
        let service = match OpenServiceW(*manager, w!("SunshineService"), SERVICE_QUERY_STATUS) {
            Ok(service) => Owned::new(service),
            // ERROR_SERVICE_DOES_NOT_EXIST (1060): Sunshine runs in user mode, without the service
            Err(error) if error.code().0 as u32 & 0xFFFF == 1060 => return ("missing".to_string(), 0),
            Err(_) => return ("unknown".to_string(), 0),
        };
        let mut status = SERVICE_STATUS_PROCESS::default();
        let mut needed = 0;
        let buffer = std::slice::from_raw_parts_mut(std::ptr::from_mut(&mut status).cast::<u8>(), std::mem::size_of::<SERVICE_STATUS_PROCESS>());
        if QueryServiceStatusEx(*service, SC_STATUS_PROCESS_INFO, Some(buffer), &mut needed).is_err() {
            return ("unknown".to_string(), 0);
        }
        (host_facts_math::service_label(status.dwCurrentState.0).to_string(), status.dwProcessId)
    }
}

#[cfg(target_os = "windows")]
fn read() -> Result<HostFacts, String> {
    let processes: Vec<Proc> = crate::device_probe::processes_named("sunshine")?
        .into_iter()
        .map(|p| Proc { pid: p.pid, name: p.name, created_ms: crate::device_probe::unix_ms_from_filetime(p.created).unwrap_or(0) })
        .collect();
    let host = host_facts_math::pick_host(&processes);
    let (service, service_pid) = service_state();
    Ok(HostFacts {
        running: host.is_some(),
        pid: host.map_or(0, |p| p.pid),
        started_ms: host.map_or(0, |p| p.created_ms),
        service,
        service_pid,
        expected_stop: host_facts_math::expected_now(EXPECTED_UNTIL_MS.load(Ordering::Relaxed), now_ms()),
    })
}

#[cfg(not(target_os = "windows"))]
fn read() -> Result<HostFacts, String> {
    Ok(HostFacts { running: false, pid: 0, started_ms: 0, service: "unknown".to_string(), service_pid: 0, expected_stop: host_facts_math::expected_now(EXPECTED_UNTIL_MS.load(Ordering::Relaxed), now_ms()) })
}

/// Read fresh every time (a few milliseconds: one process list and one service query); never cached, a cache would answer for a host that is gone.
#[tauri::command]
pub async fn get_sunshine_host_facts() -> Result<HostFacts, String> {
    tokio::task::spawn_blocking(read).await.map_err(|e| e.to_string())?
}
