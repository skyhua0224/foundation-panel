//! The elevated path for service changes.
//!
//! The elevated process is this panel executable, started by ShellExecuteExW
//! with the `runas` verb from its install folder, with [`HELPER_VERB`] and one
//! allowlisted operation. It writes no file and runs no shell: it calls the
//! SCM itself (`scm::apply`). Before asking for consent the unelevated parent
//! checks that
//! - the executable sits in `<install>\assets\gui\` of the install whose
//!   `tools\sunshinesvc.exe` is the registered service binary, and
//! - the current (unelevated) user can neither modify the executable nor
//!   create files in its folder, so no same-user process can swap the image
//!   or plant a DLL the elevated process would load from its own folder.
//!
//! The helper re-parses its command line against the same allowlist, checks
//! that it really is elevated and that the service belongs to its install,
//! and reports through its exit code only.

use super::model::{HELPER_VERB, Operation, describe_helper_exit, exit, parse_helper_arguments};
use std::path::{Path, PathBuf};

const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);
const ERROR_ACCESS_DENIED: u32 = 5;

/// Whether the current user can create a file in `directory`. Fails closed:
/// any answer other than "access denied" counts as writable.
fn current_user_can_create_in(directory: &Path) -> bool {
    use windows::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_DELETE_ON_CLOSE, FILE_GENERIC_WRITE,
        FILE_SHARE_NONE,
    };
    let probe = directory.join(format!(".sunshine-service-probe-{}", uuid::Uuid::new_v4()));
    let opened = unsafe {
        CreateFileW(
            &windows::core::HSTRING::from(probe.as_os_str()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            CREATE_NEW,
            FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE,
            None,
        )
    };
    match opened {
        Ok(handle) => {
            // DELETE_ON_CLOSE removes the probe with the handle.
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            true
        }
        Err(error) => super::scm::win32_code(&error) != ERROR_ACCESS_DENIED,
    }
}

/// Whether the current user may modify, replace or re-permission `file`.
/// The access check runs before the sharing check, so a running image the
/// user may write answers with a sharing violation (counted as writable).
fn current_user_can_modify(file: &Path) -> bool {
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, DELETE, FILE_APPEND_DATA, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_WRITE_DATA, OPEN_EXISTING, WRITE_DAC, WRITE_OWNER,
    };
    let access = FILE_WRITE_DATA.0 | FILE_APPEND_DATA.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0;
    let opened = unsafe {
        CreateFileW(
            &windows::core::HSTRING::from(file.as_os_str()),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    };
    match opened {
        Ok(handle) => {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            true
        }
        Err(error) => super::scm::win32_code(&error) != ERROR_ACCESS_DENIED,
    }
}

/// The executable to elevate, after the location checks of the module doc.
fn trusted_helper_executable() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("SVC-UAC-003: cannot resolve the panel executable: {error}"))?;
    let service = super::scm::open_service(windows::Win32::System::Services::SERVICE_QUERY_CONFIG)
        .map_err(|code| format!("SVC-SCM-001: {}", super::model::win32_service_error(code)))?
        .ok_or_else(|| format!("SVC-SCM-002: {}", super::model::win32_service_error(1060)))?;
    let config = super::scm::query_config(&service)
        .map_err(|code| format!("SVC-SCM-001: QueryServiceConfigW failed (Windows error {code})"))?;
    if !super::model::panel_belongs_to_service(&executable.to_string_lossy(), &config.binary_path) {
        return Err(
            "SVC-UAC-003: this panel is not the one installed with SunshineService; open the installed panel to change the service"
                .to_string(),
        );
    }
    let folder = executable
        .parent()
        .ok_or_else(|| "SVC-UAC-003: the panel executable has no folder".to_string())?;
    if current_user_can_modify(&executable) || current_user_can_create_in(folder) {
        return Err(format!(
            "SVC-UAC-003: {} can be modified without administrator rights, so it is not elevated; reinstall Sunshine into a protected folder such as Program Files",
            folder.display()
        ));
    }
    Ok(executable)
}

/// Run `operation` with administrator rights and wait for its result.
/// Blocking: call from a blocking thread.
pub(crate) fn run_blocking(operation: Operation) -> Result<(), String> {
    if crate::elevation::is_elevated() {
        // Already elevated: no consent and no second process; the same SCM
        // code (including the same-install check) runs here.
        return describe_helper_exit(super::scm::apply(operation));
    }
    let executable = trusted_helper_executable()?;
    let mut arguments = vec![HELPER_VERB];
    arguments.extend(operation.arguments());
    log::info!("Requesting administrator consent for Sunshine service operation {arguments:?}");
    let process = crate::elevation::launch_elevated(
        executable.as_os_str(),
        &arguments,
        windows::Win32::UI::WindowsAndMessaging::SW_HIDE.0,
    )
    .map_err(|error| format!("SVC-UAC-001: administrator authorization was canceled: {error}"))?;
    let code = process
        .wait_for_exit_blocking(HELPER_TIMEOUT)?
        .ok_or_else(|| "SVC-SCM-005: the service helper did not finish in time".to_string())?;
    describe_helper_exit(code)
}

pub(crate) async fn run(operation: Operation) -> Result<(), String> {
    tokio::task::spawn_blocking(move || run_blocking(operation))
        .await
        .map_err(|error| format!("SVC-UAC-002: service helper task failed: {error}"))?
}

/// Entry point of the helper process, checked first thing in `main`.
pub(crate) fn try_handle_elevated_command() -> Option<i32> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let operation = match parse_helper_arguments(&arguments)? {
        Ok(operation) => operation,
        Err(error) => {
            log::warn!("{error}");
            return Some(exit::BAD_ARGUMENTS);
        }
    };
    if !crate::elevation::is_elevated() {
        return Some(exit::NOT_ELEVATED);
    }
    Some(super::scm::apply(operation))
}
