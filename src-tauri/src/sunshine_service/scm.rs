//! Direct Service Control Manager calls for SunshineService.
//!
//! Reads need only SC_MANAGER_CONNECT and SERVICE_QUERY_CONFIG /
//! SERVICE_QUERY_STATUS, which the default service DACL grants to
//! interactive users. Changes (SERVICE_CHANGE_CONFIG, SERVICE_START,
//! SERVICE_STOP) are only attempted by the elevated helper.

use super::model::{Operation, SERVICE_NAME, StartType};
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW,
    QueryServiceConfig2W, QueryServiceConfigW, QueryServiceStatusEx, SC_HANDLE,
    SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CONFIG, SERVICE_STATUS,
    SERVICE_STATUS_PROCESS,
};
use windows::core::{HSTRING, PCWSTR};

pub(crate) const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
pub(crate) const ERROR_SERVICE_ALREADY_RUNNING: u32 = 1056;
pub(crate) const ERROR_SERVICE_DOES_NOT_EXIST: u32 = 1060;
pub(crate) const ERROR_SERVICE_NOT_ACTIVE: u32 = 1062;

/// Win32 code of a windows-rs error (HRESULT_FROM_WIN32 keeps it in the low word).
pub(crate) fn win32_code(error: &windows::core::Error) -> u32 {
    (error.code().0 as u32) & 0xFFFF
}

pub(crate) struct ScHandle(SC_HANDLE);

impl Drop for ScHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

/// Open SunshineService with `access`. `Ok(None)`: the service is not installed.
pub(crate) fn open_service(access: u32) -> Result<Option<ScHandle>, u32> {
    unsafe {
        let manager = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
            .map_err(|error| win32_code(&error))?;
        let manager = ScHandle(manager);
        match OpenServiceW(manager.0, &HSTRING::from(SERVICE_NAME), access) {
            Ok(service) => Ok(Some(ScHandle(service))),
            Err(error) if win32_code(&error) == ERROR_SERVICE_DOES_NOT_EXIST => Ok(None),
            Err(error) => Err(win32_code(&error)),
        }
    }
}

fn wide_to_string(text: windows::core::PWSTR) -> String {
    if text.is_null() {
        return String::new();
    }
    unsafe { text.to_string().unwrap_or_default() }
}

/// An 8-byte aligned buffer for the variable-size SCM query structures.
fn aligned_buffer(bytes: u32) -> Vec<u64> {
    vec![0u64; (bytes as usize).div_ceil(8).max(1)]
}

fn as_bytes(buffer: &mut [u64]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), buffer.len() * 8) }
}

pub(crate) struct ServiceConfig {
    pub(crate) start_type: u32,
    pub(crate) binary_path: String,
    pub(crate) account: String,
}

/// QueryServiceConfigW, sized with the documented two-call pattern.
pub(crate) fn query_config(service: &ScHandle) -> Result<ServiceConfig, u32> {
    unsafe {
        let mut needed = 0u32;
        match QueryServiceConfigW(service.0, None, 0, &mut needed) {
            Ok(()) => {}
            Err(error) if win32_code(&error) == ERROR_INSUFFICIENT_BUFFER => {}
            Err(error) => return Err(win32_code(&error)),
        }
        let mut buffer = aligned_buffer(needed);
        let config = buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
        QueryServiceConfigW(service.0, Some(config), needed, &mut needed)
            .map_err(|error| win32_code(&error))?;
        let config = &*config;
        Ok(ServiceConfig {
            start_type: config.dwStartType.0,
            binary_path: wide_to_string(config.lpBinaryPathName),
            account: wide_to_string(config.lpServiceStartName),
        })
    }
}

/// QueryServiceConfig2W into an aligned buffer; returns the buffer that holds
/// the structure of `level`.
pub(crate) fn query_config2(service: &ScHandle, level: SERVICE_CONFIG) -> Result<Vec<u64>, u32> {
    unsafe {
        let mut needed = 0u32;
        match QueryServiceConfig2W(service.0, level, None, &mut needed) {
            Ok(()) => {}
            Err(error) if win32_code(&error) == ERROR_INSUFFICIENT_BUFFER => {}
            Err(error) => return Err(win32_code(&error)),
        }
        let mut buffer = aligned_buffer(needed);
        QueryServiceConfig2W(service.0, level, Some(&mut as_bytes(&mut buffer)[..needed as usize]), &mut needed)
            .map_err(|error| win32_code(&error))?;
        Ok(buffer)
    }
}

pub(crate) fn query_delayed_auto_start(service: &ScHandle) -> Result<bool, u32> {
    use windows::Win32::System::Services::{
        SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_DELAYED_AUTO_START_INFO,
    };
    let buffer = query_config2(service, SERVICE_CONFIG_DELAYED_AUTO_START_INFO)?;
    let info = unsafe { &*buffer.as_ptr().cast::<SERVICE_DELAYED_AUTO_START_INFO>() };
    Ok(info.fDelayedAutostart.as_bool())
}

/// The start type as the panel names it, read from the SCM (not the registry).
pub(crate) fn query_start_type(service: &ScHandle) -> Result<(StartType, ServiceConfig), String> {
    let config = query_config(service)
        .map_err(|code| format!("SVC-SCM-001: QueryServiceConfigW failed (Windows error {code})"))?;
    let delayed = if config.start_type == super::model::SERVICE_AUTO_START {
        query_delayed_auto_start(service).map_err(|code| {
            format!("SVC-SCM-001: QueryServiceConfig2W failed (Windows error {code})")
        })?
    } else {
        false
    };
    Ok((StartType::from_scm(config.start_type, delayed)?, config))
}

pub(crate) fn query_status(service: &ScHandle) -> Result<SERVICE_STATUS_PROCESS, u32> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut needed = 0u32;
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(
            std::ptr::from_mut(&mut status).cast::<u8>(),
            std::mem::size_of::<SERVICE_STATUS_PROCESS>(),
        );
        QueryServiceStatusEx(service.0, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed)
            .map_err(|error| win32_code(&error))?;
    }
    Ok(status)
}

// ---- changes: only the elevated helper (or an already elevated panel) calls these ----

/// ChangeServiceConfigW for dwStartType, then ChangeServiceConfig2W for the
/// delayed flag. Every other field stays SERVICE_NO_CHANGE / NULL.
pub(crate) fn set_start_type(service: &ScHandle, start_type: StartType) -> Result<(), u32> {
    use windows::Win32::System::Services::{
        ChangeServiceConfig2W, ChangeServiceConfigW, ENUM_SERVICE_TYPE, SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
        SERVICE_DELAYED_AUTO_START_INFO, SERVICE_ERROR, SERVICE_NO_CHANGE, SERVICE_START_TYPE,
    };
    unsafe {
        ChangeServiceConfigW(
            service.0,
            ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
            SERVICE_START_TYPE(start_type.scm_start_type()),
            SERVICE_ERROR(SERVICE_NO_CHANGE),
            PCWSTR::null(),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        )
        .map_err(|error| win32_code(&error))?;
        // The delayed flag is documented as ignored for non-auto services, so
        // it is cleared for them too: a later switch to Auto never inherits a
        // stale delayed flag.
        let info = SERVICE_DELAYED_AUTO_START_INFO {
            fDelayedAutostart: start_type.delayed().into(),
        };
        ChangeServiceConfig2W(
            service.0,
            SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
            Some(std::ptr::from_ref(&info).cast()),
        )
        .map_err(|error| win32_code(&error))?;
    }
    Ok(())
}

const STATE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Poll until the service reports `target` (SERVICE_STOPPED = 1, SERVICE_RUNNING = 4).
fn wait_for_state(service: &ScHandle, target: u32) -> Result<(), i32> {
    let deadline = std::time::Instant::now() + STATE_WAIT;
    loop {
        let status = query_status(service).map_err(super::model::exit::from_win32)?;
        if status.dwCurrentState.0 == target {
            return Ok(());
        }
        // A start that fell back to STOPPED will not reach RUNNING.
        if target == 4 && status.dwCurrentState.0 == 1 {
            let code = if status.dwWin32ExitCode != 0 { status.dwWin32ExitCode } else { 1053 };
            return Err(super::model::exit::from_win32(code));
        }
        if std::time::Instant::now() >= deadline {
            return Err(super::model::exit::TIMEOUT);
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn stop(service: &ScHandle) -> Result<(), i32> {
    let mut status = SERVICE_STATUS::default();
    unsafe {
        match ControlService(service.0, windows::Win32::System::Services::SERVICE_CONTROL_STOP, &mut status) {
            Ok(()) => {}
            Err(error) if win32_code(&error) == ERROR_SERVICE_NOT_ACTIVE => return Ok(()),
            Err(error) => return Err(super::model::exit::from_win32(win32_code(&error))),
        }
    }
    wait_for_state(service, 1)
}

fn start(service: &ScHandle) -> Result<(), i32> {
    unsafe {
        match windows::Win32::System::Services::StartServiceW(service.0, None) {
            Ok(()) => {}
            Err(error) if win32_code(&error) == ERROR_SERVICE_ALREADY_RUNNING => return Ok(()),
            Err(error) => return Err(super::model::exit::from_win32(win32_code(&error))),
        }
    }
    wait_for_state(service, 4)
}

/// Stop the service and end whatever sunshine.exe of the install is left (the wrapper ends its child; a
/// user-mode or orphan one is ended here), as `net stop` plus `taskkill /F` did.
fn stop_all(service: &ScHandle, install_dir: &std::path::Path) -> Result<(), i32> {
    stop(service)?;
    super::probe::end_install_processes(install_dir).map_err(super::model::exit::from_win32)
}

/// `<root>` of the panel executable `<root>\assets\gui\<exe>` (checked by panel_belongs_to_service first).
fn install_dir_of(executable: &std::path::Path) -> Option<std::path::PathBuf> {
    Some(executable.parent()?.parent()?.parent()?.to_path_buf())
}

/// Run one allowlisted operation; the result is a helper exit code.
pub(crate) fn apply(operation: Operation) -> i32 {
    use super::model::exit;
    use windows::Win32::System::Services::{
        SERVICE_CHANGE_CONFIG, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_START, SERVICE_STOP,
    };
    let access = SERVICE_QUERY_CONFIG
        | SERVICE_QUERY_STATUS
        | match operation {
            Operation::SetStartType(_) => SERVICE_CHANGE_CONFIG,
            Operation::Start => SERVICE_START,
            Operation::Stop => SERVICE_STOP,
            Operation::Restart | Operation::UserMode => SERVICE_START | SERVICE_STOP,
        };
    let service = match open_service(access) {
        Ok(Some(service)) => service,
        Ok(None) => return exit::from_win32(ERROR_SERVICE_DOES_NOT_EXIST),
        Err(code) => return exit::from_win32(code),
    };
    // The service must be the one of the install this executable belongs to.
    let install_dir = match (query_config(&service), std::env::current_exe()) {
        (Ok(config), Ok(executable))
            if super::model::panel_belongs_to_service(&executable.to_string_lossy(), &config.binary_path) =>
        {
            match install_dir_of(&executable) {
                Some(dir) => dir,
                None => return exit::FOREIGN_SERVICE,
            }
        }
        (Err(code), _) => return exit::from_win32(code),
        _ => return exit::FOREIGN_SERVICE,
    };
    let result = match operation {
        Operation::SetStartType(start_type) => set_start_type(&service, start_type).map_err(exit::from_win32),
        Operation::Start => start(&service),
        Operation::Stop => stop(&service),
        Operation::Restart => stop_all(&service, &install_dir).and_then(|()| start(&service)),
        Operation::UserMode => stop_all(&service, &install_dir).and_then(|()| {
            // the helper is elevated, so sunshine.exe runs with administrator rights as the old RunAs start did
            super::probe::start_install_process(&install_dir).map_err(|_| exit::PROCESS_START)
        }),
    };
    match result {
        Ok(()) => exit::OK,
        Err(code) => code,
    }
}
