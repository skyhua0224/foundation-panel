//! SunshineService: its start type and the elevated path that changes it.
//!
//! The tray and the settings pages share this back end. Reads go to the SCM
//! (QueryServiceConfigW / QueryServiceConfig2W), not the registry and not
//! `sc query` text. Changes run in an elevated copy of the installed panel
//! with one fixed verb and an allowlisted operation that calls
//! ChangeServiceConfigW / ChangeServiceConfig2W itself (see `elevated`).
//! Microsoft documents followed: docs/sunshine-service-control.md.

mod model;
#[cfg(target_os = "windows")]
mod elevated;
#[cfg(target_os = "windows")]
mod scm;

pub(crate) use model::StartType;
#[allow(unused_imports)]
pub(crate) use model::{Operation, require_reboot_acknowledgement, service_binary_matches_install};

#[cfg(target_os = "windows")]
pub(crate) use elevated::try_handle_elevated_command;

/// The start type of the registered service and whether it belongs to the
/// install this panel manages. `Ok(None)`: no SunshineService.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegisteredService {
    pub(crate) start_type: StartType,
    pub(crate) binary_path: String,
    pub(crate) account: String,
}

#[cfg(target_os = "windows")]
pub(crate) fn registered_service() -> Result<Option<RegisteredService>, String> {
    use windows::Win32::System::Services::SERVICE_QUERY_CONFIG;
    let Some(service) = scm::open_service(SERVICE_QUERY_CONFIG)
        .map_err(|code| format!("SVC-SCM-001: {}", model::win32_service_error(code)))?
    else {
        return Ok(None);
    };
    let (start_type, config) = scm::query_start_type(&service)?;
    Ok(Some(RegisteredService {
        start_type,
        binary_path: config.binary_path,
        account: config.account,
    }))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn registered_service() -> Result<Option<RegisteredService>, String> {
    Ok(None)
}

/// Change the start type after the reboot acknowledgement check. Blocking:
/// UAC consent and the helper run on the calling thread.
pub(crate) fn set_start_type_blocking(target: StartType, acknowledged: bool) -> Result<(), String> {
    require_reboot_acknowledgement(target, acknowledged)?;
    #[cfg(target_os = "windows")]
    {
        elevated::run_blocking(Operation::SetStartType(target))?;
        // Read back: the SCM is the authority on what was stored.
        match registered_service()? {
            Some(service) if service.start_type == target => Ok(()),
            Some(service) => Err(format!(
                "SVC-SCM-006: the service start type is {} after the change, expected {}",
                service.start_type.token(),
                target.token()
            )),
            None => Err(format!("SVC-SCM-002: {}", model::win32_service_error(1060))),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("SVC-OS-001: the Sunshine service exists only on Windows".to_string())
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTypeChange {
    pub start_type: StartType,
    pub starts_at_boot: bool,
}

/// Settings page and tray: set the start type of SunshineService.
/// `acknowledge_unreachable_after_reboot` must be true for Manual and Disabled.
#[tauri::command]
pub async fn set_sunshine_service_start_type(
    start_type: StartType,
    acknowledge_unreachable_after_reboot: bool,
) -> Result<StartTypeChange, String> {
    tokio::task::spawn_blocking(move || {
        set_start_type_blocking(start_type, acknowledge_unreachable_after_reboot)
    })
    .await
    .map_err(|error| format!("SVC-UAC-002: service task failed: {error}"))??;
    Ok(StartTypeChange {
        start_type,
        starts_at_boot: start_type.starts_at_boot(),
    })
}
