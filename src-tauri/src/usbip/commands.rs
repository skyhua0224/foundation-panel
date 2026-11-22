//! Tauri command surface for the Device Hub USB/IP panel.

use super::elevated::run_elevated;
use super::exec::{CommandScope, parse_remote_devices, require_success, run_usbip};
use super::{
    ElevatedRequest, OPERATION_LOCK, UsbipRemoteDevice, UsbipStatus, validate_bus_id,
    validate_remote, validate_tcp_port,
};

/// Validates the exporter address and port, then lists devices using the verified shared tools.
#[tauri::command]
pub async fn usbip_list_remote(
    remote: String,
    tcp_port: Option<u16>,
) -> Result<Vec<UsbipRemoteDevice>, String> {
    let remote = validate_remote(&remote)?;
    let tcp_port = validate_tcp_port(tcp_port)?;
    let output = run_usbip(vec![
        "--tcp-port".to_string(),
        tcp_port.to_string(),
        "list".to_string(),
        "--remote".to_string(),
        remote,
    ])
    .await?;
    let text = require_success(output, "discovering remote devices", CommandScope::Remote)?;
    Ok(parse_remote_devices(&text))
}

#[tauri::command]
pub async fn usbip_install_transport() -> Result<UsbipStatus, String> {
    let _guard = OPERATION_LOCK.lock().await;
    #[cfg(target_os = "windows")]
    super::elevated::check_driver_operation()?;
    let _fresh = super::status::fresh();
    #[cfg(target_os = "windows")]
    let mut kept = {
        let mut kept = super::journal::read();
        if let Some(marker) = kept.marker.as_mut() {
            if !marker.restarted_since(super::journal::boot_secs()) {
                let error = if marker.op == super::nodes::MarkerOp::Install {
                    "USBIP-SETUP-015: a Windows restart is pending before USB/IP can finish installing".to_string()
                } else {
                    "USBIP-SETUP-015: a Windows restart is pending from an earlier USB/IP cleanup; restart Windows first".to_string()
                };
                kept.last_install_error = error.clone();
                kept.install_error_at = super::journal::now_secs();
                super::journal::write(&kept);
                return Err(error);
            }
            if marker.op == super::nodes::MarkerOp::Install {
                marker.resumed += 1;
            }
        }
        kept
    };
    let outcome = run_elevated(ElevatedRequest::Install).await;
    super::status::forget_kept();
    #[cfg(target_os = "windows")]
    {
        match &outcome {
            Ok(_) => {
                kept.last_install_error.clear();
                kept.install_error_at = 0;
                if matches!(kept.marker.as_ref().map(|m| m.op), Some(super::nodes::MarkerOp::Install)) {
                    kept.marker = None;
                }
            }
            Err(error) => {
                kept.last_install_error = error.clone();
                kept.install_error_at = super::journal::now_secs();
                if super::install::install_error_has_pending_restart(error) {
                    mark_restart_pending(&mut kept, super::nodes::MarkerOp::Install);
                }
            }
        }
        super::journal::write(&kept);
    }
    let response = outcome?;
    let reboot_recommended = keep_restart_answer(&response.message);
    let mut status = super::status::usbip_get_status().await?;
    status.reboot_recommended |= reboot_recommended;
    Ok(status)
}

/// Removes a broken USB/IP installation plus every leftover VHCI device node
/// so the pinned installer can run from a clean state. The elevated helper
/// uninstalls the registered transport (when present), then sweeps devnodes
/// that the vendor uninstaller failed to remove. A removal that Windows finishes
/// at the next restart is kept in the person's journal and continued after it.
#[tauri::command]
pub async fn usbip_cleanup_transport() -> Result<UsbipStatus, String> {
    let _guard = OPERATION_LOCK.lock().await;
    #[cfg(target_os = "windows")]
    super::elevated::check_driver_operation()?;
    let _fresh = super::status::fresh();
    run_cleanup(ElevatedRequest::Cleanup, super::nodes::MarkerOp::Cleanup).await
}

/// Removes the older controllers that stand beside the working one and keeps the live controller: no reinstall needed.
#[tauri::command]
pub async fn usbip_remove_stale_nodes() -> Result<UsbipStatus, String> {
    let _guard = OPERATION_LOCK.lock().await;
    #[cfg(target_os = "windows")]
    super::elevated::check_driver_operation()?;
    let _fresh = super::status::fresh();
    run_cleanup(ElevatedRequest::RemoveStale, super::nodes::MarkerOp::RemoveStale).await
}

/// Writes the USB/IP diagnostics file (device tree, services, driver packages, Windows version, status, last cleanup error)
/// and shows it in Explorer. Returns the path so the person can attach it to a report.
#[tauri::command]
pub async fn usbip_export_diagnostics() -> Result<String, String> {
    super::diagnostics::export().await
}

/// An install or a replacement answers `reboot_recommended` (the installer asks for a restart) and/or the restart-pending answer (the
/// controllers an older version left are accepted for removal at the restart). Only the second leaves a marker, so a restart that
/// only the installer asked for does not bring a cleanup back. Returns whether a restart is advised at all.
fn keep_restart_answer(message: &str) -> bool {
    #[cfg(target_os = "windows")]
    if message == super::elevated::RESTART_PENDING {
        let mut kept = super::journal::read();
        mark_restart_pending(&mut kept, super::nodes::MarkerOp::RemoveStale);
        super::journal::write(&kept);
        return true;
    }
    message == "reboot_recommended"
}

/// Keeps what an operation learned about the restart: Windows accepted a removal that finishes at the next restart, so a marker
/// says which operation to continue afterwards. `resumed` carries over, so a node that never goes does not loop.
#[cfg(target_os = "windows")]
fn mark_restart_pending(kept: &mut super::journal::Journal, op: super::nodes::MarkerOp) {
    kept.marker = Some(super::nodes::RestartMarker {
        written: super::journal::now_secs(),
        resumed: kept.marker.as_ref().map_or(0, |marker| marker.resumed),
        op,
    });
}

/// Runs one cleanup operation in the elevated helper and keeps what it answered: the error (for the diagnostics) and the
/// restart marker. The helper always runs elevated (a UAC prompt when the panel is not), so a refused removal is a real
/// refusal and is reported with the code Windows gave, never retried silently.
async fn run_cleanup(
    request: ElevatedRequest,
    #[cfg_attr(not(target_os = "windows"), allow(unused_variables))] op: super::nodes::MarkerOp,
) -> Result<UsbipStatus, String> {
    #[cfg(target_os = "windows")]
    let mut kept = super::journal::read();
    #[cfg(target_os = "windows")]
    if let Some(marker) = kept.marker.as_mut() {
        // Continuing after a restart counts against the cap, so a node that never goes does not loop.
        if marker.restarted_since(super::journal::boot_secs()) {
            marker.resumed += 1;
        }
    }
    let outcome = run_elevated(request).await;
    super::status::forget_kept();
    #[cfg(target_os = "windows")]
    {
        match &outcome {
            Ok(response) => {
                kept.last_error.clear();
                kept.error_at = 0;
                if response.message == super::elevated::RESTART_PENDING {
                    mark_restart_pending(&mut kept, op);
                } else {
                    kept.marker = None;
                }
            }
            Err(error) => {
                kept.last_error = error.clone();
                kept.error_at = super::journal::now_secs();
                // Some nodes were accepted for the restart while others failed: the marker is kept for the first, the error shows the second.
                if super::nodes::error_has_pending_restart(error) {
                    mark_restart_pending(&mut kept, op);
                }
            }
        }
        super::journal::write(&kept);
    }
    outcome?;
    super::status::usbip_get_status().await
}

/// Replaces an external, fully installed usbip-win2 with the host's package after the person confirmed it. The host's
/// helper decides again whether the installation may be replaced; this side only refuses what it can see is unsafe.
#[tauri::command]
pub async fn usbip_replace_legacy_transport() -> Result<UsbipStatus, String> {
    let _guard = OPERATION_LOCK.lock().await;
    #[cfg(target_os = "windows")]
    super::elevated::check_driver_operation()?;
    let _fresh = super::status::fresh();
    let before = super::status::usbip_get_status().await?;
    if !before.replaceable {
        return Err("USBIP-REPLACE-001: this USB/IP installation cannot be replaced from here; nothing was changed".to_string());
    }
    if !before.attached_devices.is_empty() {
        return Err(
            "USBIP-REPLACE-005: detach the attached USB devices first; nothing was changed"
                .to_string(),
        );
    }
    let outcome = run_elevated(ElevatedRequest::ReplaceLegacy).await;
    super::status::forget_kept();
    #[cfg(target_os = "windows")]
    {
        let mut kept = super::journal::read();
        match &outcome {
            Ok(_) => {
                kept.last_install_error.clear();
                kept.install_error_at = 0;
            }
            Err(error) => {
                kept.last_install_error = error.clone();
                kept.install_error_at = super::journal::now_secs();
            }
        }
        super::journal::write(&kept);
    }
    let response = outcome?;
    let reboot_recommended = keep_restart_answer(&response.message);
    let mut status = super::status::usbip_get_status().await?;
    status.reboot_recommended |= reboot_recommended;
    Ok(status)
}

#[tauri::command]
pub async fn usbip_attach(
    remote: String,
    tcp_port: Option<u16>,
    bus_id: String,
) -> Result<u8, String> {
    let request = ElevatedRequest::Attach {
        remote: validate_remote(&remote)?,
        tcp_port: validate_tcp_port(tcp_port)?,
        bus_id: validate_bus_id(&bus_id)?,
    };
    let _guard = OPERATION_LOCK.lock().await;
    let response = run_elevated(request).await?;
    response
        .port
        .ok_or_else(|| "USBIP-EXEC-005: attach succeeded without returning a port".to_string())
}

#[tauri::command]
pub async fn usbip_detach(port: u8) -> Result<(), String> {
    if port == 0 {
        return Err("USBIP-INPUT-004: the local USB/IP port is invalid".to_string());
    }
    let _guard = OPERATION_LOCK.lock().await;
    run_elevated(ElevatedRequest::Detach { port }).await?;
    Ok(())
}
