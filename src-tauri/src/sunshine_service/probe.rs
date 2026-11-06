//! Windows reads for the three status layers.

use super::scm;
use super::status::{ProcessLayer, Recovery, RecoveryAction, RecoveryKind, ScmLayer, ScmState, last_exit};
use std::path::Path;

fn recovery(service: &scm::ScHandle) -> Result<Recovery, u32> {
    use windows::Win32::System::Services::{SERVICE_CONFIG_FAILURE_ACTIONS, SERVICE_FAILURE_ACTIONSW};
    let buffer = scm::query_config2(service, SERVICE_CONFIG_FAILURE_ACTIONS)?;
    let info = unsafe { &*buffer.as_ptr().cast::<SERVICE_FAILURE_ACTIONSW>() };
    let actions = if info.lpsaActions.is_null() || info.cActions == 0 {
        Vec::new()
    } else {
        // lpsaActions points into `buffer`, which is alive for this scope.
        unsafe { std::slice::from_raw_parts(info.lpsaActions, info.cActions as usize) }
            .iter()
            .map(|action| RecoveryAction { kind: RecoveryKind::from_scm(action.Type.0), delay_ms: action.Delay })
            .collect()
    };
    Ok(Recovery::new(actions, info.dwResetPeriod))
}

/// SCM layer, from QueryServiceStatusEx / QueryServiceConfigW / QueryServiceConfig2W.
pub(crate) fn scm_layer(install_dir: &Path) -> ScmLayer {
    use windows::Win32::System::Services::{SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS};
    let service = match scm::open_service(SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS) {
        Ok(Some(service)) => service,
        Ok(None) => return ScmLayer::default(),
        Err(code) => {
            return ScmLayer {
                error: Some(format!("SVC-SCM-001: {}", super::model::win32_service_error(code))),
                ..Default::default()
            };
        }
    };
    let mut layer = ScmLayer { installed: true, ..Default::default() };
    match scm::query_start_type(&service) {
        Ok((start_type, config)) => {
            layer.start_type = Some(start_type);
            layer.belongs_to_install =
                super::model::service_binary_matches_install(&config.binary_path, &install_dir.to_string_lossy());
            layer.binary_path = config.binary_path;
            layer.account = config.account;
        }
        Err(error) => {
            // The binary path still tells whether the service is ours.
            if let Ok(config) = scm::query_config(&service) {
                layer.belongs_to_install =
                    super::model::service_binary_matches_install(&config.binary_path, &install_dir.to_string_lossy());
                layer.binary_path = config.binary_path;
                layer.account = config.account;
            }
            layer.error = Some(error);
        }
    }
    match scm::query_status(&service) {
        Ok(status) => {
            layer.state = Some(ScmState::from_scm(status.dwCurrentState.0));
            layer.last_exit = last_exit(status.dwWin32ExitCode, status.dwServiceSpecificExitCode);
            layer.service_pid = (status.dwProcessId != 0).then_some(status.dwProcessId);
        }
        Err(code) => {
            layer.error.get_or_insert(format!("SVC-SCM-001: QueryServiceStatusEx failed (Windows error {code})"));
        }
    }
    layer.recovery = recovery(&service).ok();
    layer
}

fn process_start_ms(pid: u32) -> Option<u64> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let (mut created, mut exited, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        let result = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
        let _ = CloseHandle(handle);
        result.ok()?;
        // FILETIME: 100 ns ticks since 1601-01-01 UTC.
        let ticks = ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64;
        (ticks / 10_000).checked_sub(11_644_473_600_000)
    }
}

fn process_image(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; 1024];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buffer.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        result.ok()?;
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }
}

/// sunshine.exe processes of this install as (pid, parent pid), from a Toolhelp snapshot. A process whose
/// image cannot be read (another session, protected) is counted as ours only when `include_unreadable`.
pub(crate) fn install_processes(install_dir: &Path, include_unreadable: bool) -> Result<Vec<(u32, u32)>, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    let expected = install_dir.join("sunshine.exe").to_string_lossy().to_ascii_lowercase();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
        .map_err(|error| format!("SVC-PROC-001: process snapshot failed: {error}"))?;
    let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    let mut found = Vec::new();
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        let length = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
        if name.eq_ignore_ascii_case("sunshine.exe") {
            let pid = entry.th32ProcessID;
            // another install's sunshine.exe is not this host
            let ours = match process_image(pid) {
                Some(image) => image.to_ascii_lowercase() == expected,
                None => include_unreadable,
            };
            if ours {
                found.push((pid, entry.th32ParentProcessID));
            }
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    Ok(found)
}

/// Process layer: the sunshine.exe of this install, its start time, and whether the service process is its parent.
pub(crate) fn process_layer(install_dir: &Path, service_pid: Option<u32>) -> ProcessLayer {
    let processes = match install_processes(install_dir, true) {
        Ok(processes) => processes,
        Err(error) => return ProcessLayer { error: Some(error), ..Default::default() },
    };
    // prefer the child of the service when there are several
    let chosen = processes
        .iter()
        .find(|(_, parent)| Some(*parent) == service_pid)
        .or_else(|| processes.first())
        .copied();
    match chosen {
        Some((pid, parent)) => ProcessLayer {
            running: true,
            pid: Some(pid),
            started_at_ms: process_start_ms(pid),
            child_of_service: service_pid.is_some_and(|service| service == parent),
            error: None,
        },
        None => ProcessLayer::default(),
    }
}

/// End every sunshine.exe of this install (TerminateProcess, then wait up to 5 s for each). Only images that
/// read as `<install_dir>\sunshine.exe` are ended. `Err(win32 code)` on the first one that cannot be ended.
pub(crate) fn end_install_processes(install_dir: &Path) -> Result<(), u32> {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
    };
    const ERROR_INVALID_PARAMETER: u32 = 87; // the process is already gone
    for (pid, _) in install_processes(install_dir, false).map_err(|_| 1u32)? {
        unsafe {
            let handle = match OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) {
                Ok(handle) => handle,
                Err(error) if super::scm::win32_code(&error) == ERROR_INVALID_PARAMETER => continue,
                Err(error) => return Err(super::scm::win32_code(&error)),
            };
            let ended = TerminateProcess(handle, 1).map_err(|error| super::scm::win32_code(&error));
            let waited = WaitForSingleObject(handle, 5000);
            let _ = CloseHandle(handle);
            ended?;
            if waited != WAIT_OBJECT_0 {
                return Err(1053);
            }
        }
    }
    Ok(())
}

/// Start `<install_dir>\sunshine.exe` in its folder, hidden, as the calling process's user and token.
pub(crate) fn start_install_process(install_dir: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new(install_dir.join("sunshine.exe"))
        .current_dir(install_dir)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("SVC-PROC-003: sunshine.exe did not start: {error}"))
}
