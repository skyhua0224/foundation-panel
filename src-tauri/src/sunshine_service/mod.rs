//! SunshineService: its state in three layers (SCM, process, API), its start
//! type, and the elevated path that changes them.
//!
//! The tray and the settings pages share this back end. Reads go to the SCM
//! (QueryServiceConfigW / QueryServiceConfig2W), not the registry and not
//! `sc query` text. Changes run in an elevated copy of the installed panel
//! with one fixed verb and an allowlisted operation that calls
//! ChangeServiceConfigW / ChangeServiceConfig2W itself (see `elevated`).
//! Microsoft documents followed: docs/sunshine-service-control.md.

mod model;
mod status;
#[cfg(target_os = "windows")]
mod elevated;
#[cfg(target_os = "windows")]
mod probe;
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

/// API layer: does the host web server answer at all.
async fn api_layer() -> status::ApiLayer {
    let started = std::time::Instant::now();
    let request = async {
        let url = crate::sunshine::get_sunshine_url().await?;
        let client = crate::sunshine::create_https_client()?;
        client
            .get(url.trim_end_matches('/').to_string() + "/")
            .send()
            .await
            .map(|response| response.status().as_u16())
            .map_err(|error| error.to_string())
    };
    match tokio::time::timeout(std::time::Duration::from_secs(3), request).await {
        Ok(Ok(code)) => status::ApiLayer {
            answers: status::api_answers(code),
            http_status: Some(code),
            latency_ms: Some(started.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(Err(error)) => status::ApiLayer { error: Some(error), ..Default::default() },
        Err(_) => status::ApiLayer { error: Some("no answer within 3 s".to_string()), ..Default::default() },
    }
}

async fn live_streams() -> Option<usize> {
    let sessions = tokio::time::timeout(std::time::Duration::from_secs(3), crate::sunshine::get_active_sessions())
        .await
        .ok()?
        .ok()?;
    Some(sessions.len())
}

/// 系统 › Sunshine 服务: the SCM record, the sunshine.exe process and the API
/// answer, each on its own, plus the verdict and the live streams.
#[tauri::command]
pub async fn get_sunshine_service_status() -> Result<status::ServiceStatus, String> {
    #[cfg(target_os = "windows")]
    let (scm, process) = tokio::task::spawn_blocking(local_layers)
        .await
        .map_err(|error| format!("SVC-SCM-001: status task failed: {error}"))?;
    #[cfg(not(target_os = "windows"))]
    let (scm, process) = (status::ScmLayer::default(), status::ProcessLayer::default());
    let (api, live_streams) = tokio::join!(api_layer(), live_streams());
    let (verdict, failing_layer) = status::summarize(&scm, &process, &api);
    Ok(status::ServiceStatus { verdict, failing_layer, scm, process, api, live_streams })
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

/// Start, stop or restart SunshineService through the elevated helper.
/// Stop and restart end live streams: with any live stream the caller must
/// pass `acknowledge_live_stream` (SVC-CONFIRM-002 otherwise).
#[tauri::command]
pub async fn control_sunshine_service(action: ServiceAction, acknowledge_live_stream: bool) -> Result<(), String> {
    let operation = match action {
        ServiceAction::Start => Operation::Start,
        ServiceAction::Stop => Operation::Stop,
        ServiceAction::Restart => Operation::Restart,
    };
    let streams = if matches!(operation, Operation::Start) { None } else { live_streams().await };
    status::require_live_stream_acknowledgement(operation, streams, acknowledge_live_stream)?;
    #[cfg(target_os = "windows")]
    {
        elevated::run(operation).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("SVC-OS-001: the Sunshine service exists only on Windows".to_string())
    }
}

/// The SCM and process layers (blocking).
#[cfg(target_os = "windows")]
fn local_layers() -> (status::ScmLayer, status::ProcessLayer) {
    let install_dir = crate::sunshine::install_dir();
    let scm = probe::scm_layer(&install_dir);
    let service_pid = scm.service_pid.filter(|_| scm.belongs_to_install);
    let process = probe::process_layer(&install_dir, service_pid);
    (scm, process)
}

/// Whether Sunshine runs in user mode: sunshine.exe of this install runs and the service of this install is
/// not running (SCM and process snapshot; no `sc query` or `tasklist` text). Blocking.
pub(crate) fn running_in_user_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        let (scm, process) = local_layers();
        let service_running = scm.installed && scm.belongs_to_install && scm.state == Some(status::ScmState::Running);
        process.running && !service_running
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// End every sunshine.exe of this install. An update calls this after the service has stopped,
/// so a user-mode copy is not left beside the next service child.
#[cfg(target_os = "windows")]
pub(crate) fn end_host_processes(install_dir: &std::path::Path) -> Result<(), u32> {
    probe::end_install_processes(install_dir)
}

/// 重启 Sunshine for the offline page, the tray recovery, the home tile and the old pages: through the elevated
/// helper (stop the service, end any leftover sunshine.exe of this install, start the service) when the install
/// has its service, else ending and starting sunshine.exe of this install as the current user. No PowerShell,
/// no `net stop`. With live streams the caller must pass `acknowledge_live_stream` (SVC-CONFIRM-002).
pub(crate) async fn restart_host(acknowledge_live_stream: bool) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let (scm, _) = tokio::task::spawn_blocking(local_layers)
            .await
            .map_err(|error| format!("SVC-SCM-001: status task failed: {error}"))?;
        let plan = status::plan_restart(&scm);
        let operation = match plan {
            status::HostPlan::Elevated(operation) => operation,
            status::HostPlan::UserProcess => Operation::Restart,
        };
        status::require_live_stream_acknowledgement(operation, live_streams().await, acknowledge_live_stream)?;
        match plan {
            status::HostPlan::Elevated(operation) => elevated::run(operation).await.map(|()| "success".to_string()),
            status::HostPlan::UserProcess => tokio::task::spawn_blocking(|| {
                let install_dir = crate::sunshine::install_dir();
                probe::end_install_processes(&install_dir).map_err(|code| {
                    format!(
                        "SVC-PROC-002: sunshine.exe could not be ended ({}); it may run with administrator rights",
                        model::win32_service_error(code)
                    )
                })?;
                probe::start_install_process(&install_dir).map(|()| "success".to_string())
            })
            .await
            .map_err(|error| format!("SVC-PROC-001: restart task failed: {error}"))?,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = acknowledge_live_stream;
        Err("SVC-OS-001: the Sunshine service exists only on Windows".to_string())
    }
}

/// The tray's mode switch (用户模式 ↔ 服务模式) through the elevated helper. Returns the mode switched to.
pub(crate) async fn toggle_host_mode(acknowledge_live_stream: bool) -> Result<&'static str, String> {
    #[cfg(target_os = "windows")]
    {
        let (scm, process) = tokio::task::spawn_blocking(local_layers)
            .await
            .map_err(|error| format!("SVC-SCM-001: status task failed: {error}"))?;
        let operation = status::plan_toggle(&scm, &process)?;
        status::require_live_stream_acknowledgement(operation, live_streams().await, acknowledge_live_stream)?;
        elevated::run(operation).await?;
        Ok(if operation == Operation::UserMode { "user" } else { "service" })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = acknowledge_live_stream;
        Err("SVC-OS-001: the Sunshine service exists only on Windows".to_string())
    }
}
