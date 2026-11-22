//! Transport status probing.

use super::{REQUIRED_VERSION, UsbipStatus};

/// Extracts the structured "USBIP-XXX-NNN" code from a detail string.
/// Returns an empty string when the prefix is not a well-shaped code, so
/// consumers can branch on `error_code` instead of parsing detail text.
pub(super) fn detail_error_code(detail: &str) -> String {
    let code = detail.split(':').next().unwrap_or_default().trim();
    let mut segments = code.split('-');
    let shaped = matches!(
        (segments.next(), segments.next(), segments.next(), segments.next()),
        (Some(area), Some(kind), Some(number), None)
            if area == "USBIP"
                && (3..=8).contains(&kind.len())
                && kind.bytes().all(|b| b.is_ascii_uppercase())
                && number.len() == 3
                && number.bytes().all(|b| b.is_ascii_digit())
    );
    if shaped {
        code.to_string()
    } else {
        String::new()
    }
}

/// A command that changes the installation or the attached devices: the kept answers of the two slow probes (the driver
/// signature, the host helper's `replaceable`; memo.rs) are dropped when it starts and again when it ends, however it
/// ends, so a read in between cannot keep the old state.
pub(super) struct Fresh;

fn drop_kept_answers() {
    super::install::forget_replaceable();
    super::signature::forget();
}

/// Drops the kept answers again after the elevated step, before the status that is returned is read.
pub(super) fn forget_kept() {
    drop_kept_answers();
}

pub(super) fn fresh() -> Fresh {
    drop_kept_answers();
    Fresh
}

impl Drop for Fresh {
    fn drop(&mut self) {
        drop_kept_answers();
    }
}

/// Spawns a `usbip.exe port` subprocess to enumerate attached devices. The two PowerShell-backed parts (the driver
/// signature, the host helper's `replaceable`) are kept while their inputs are unchanged (memo.rs).
#[tauri::command]
pub async fn usbip_get_status() -> Result<UsbipStatus, String> {
    #[cfg(not(target_os = "windows"))]
    return Ok(UsbipStatus {
        externally_managed: false,
        replaceable: false,
        supported: false,
        installed: false,
        ready: false,
        version: String::new(),
        version_valid: false,
        reboot_recommended: false,
        vhci_residual: false,
        residual_kind: String::new(),
        cleanup_pending_restart: false,
        cleanup_resume: false,
        cleanup_resume_op: String::new(),
        last_cleanup_error: String::new(),
        last_install_error: String::new(),
        install_pending_restart: false,
        install_resume: false,
        error_code: String::new(),
        attached_devices: Vec::new(),
        detail: "USB/IP passthrough is only supported on Windows".to_string(),
    });

    #[cfg(target_os = "windows")]
    {
        let (status, kind) = probe_status().await?;
        Ok(settle(status, kind))
    }
}

/// What the last cleanup left behind, from the person's own journal: the restart marker says whether the removal waits
/// for a restart, or Windows has restarted and the leftover is still there (continue once), and the last error stays for
/// the diagnostics. A marker whose leftover is gone is dropped.
#[cfg(target_os = "windows")]
fn settle(mut status: UsbipStatus, kind: super::device::Residual) -> UsbipStatus {
    use super::journal;

    status.residual_kind = if status.vhci_residual {
        kind.as_str().to_string()
    } else {
        String::new()
    };
    let mut kept = journal::read();
    let mut journal_dirty = false;
    if let Some(marker) = kept.marker.clone() {
        let restarted = marker.restarted_since(journal::boot_secs());
        if marker.op == super::nodes::MarkerOp::Install {
            if !restarted {
                status.install_pending_restart = true;
                status.reboot_recommended = true;
            } else if status.ready || status.externally_managed || status.vhci_residual {
                kept.marker = None;
                journal_dirty = true;
            } else {
                status.install_resume = marker.may_resume();
            }
        } else if !restarted {
            status.cleanup_pending_restart = true;
            status.reboot_recommended = true;
        } else if !status.vhci_residual {
            kept.marker = None;
            journal_dirty = true;
        } else {
            status.cleanup_resume = marker.may_resume();
            if status.cleanup_resume {
                status.cleanup_resume_op = marker.op.label().to_string();
            }
        }
    }
    // The last error stays until a later cleanup replaces it or a quarter of an hour has passed with nothing left over: a
    // refused or cancelled attempt must still be exportable right after the status refresh that follows it.
    const KEEP_ERROR_SECS: u64 = 15 * 60;
    if !status.vhci_residual
        && kept.marker.is_none()
        && !kept.last_error.is_empty()
        && journal::now_secs().saturating_sub(kept.error_at) > KEEP_ERROR_SECS
    {
        kept.last_error.clear();
        kept.error_at = 0;
        journal_dirty = true;
    }
    if status.ready
        && kept.marker.is_none()
        && !kept.last_install_error.is_empty()
        && journal::now_secs().saturating_sub(kept.install_error_at) > KEEP_ERROR_SECS
    {
        kept.last_install_error.clear();
        kept.install_error_at = 0;
        journal_dirty = true;
    }
    if journal_dirty {
        journal::write(&kept);
    }
    status.last_cleanup_error = kept.last_error;
    status.last_install_error = kept.last_install_error;
    status
}

#[cfg(target_os = "windows")]
async fn probe_status() -> Result<(UsbipStatus, super::device::Residual), String> {
    use super::device::Residual;

    // Fail closed: if the residual state cannot be determined, do not
    // report a clean transport. Propagating the error surfaces the probe
    // as unavailable, hiding install and attach actions.
    let externally_managed = super::manager::externally_managed();
    // Only an external installation can be offered for replacement; the host's helper decides which ones.
    let replaceable = externally_managed && super::install::host_replaceable().await;
    let interfaces = super::device::enumerate_vhci_interfaces()?;
    let nodes = super::device::enumerate_transport_nodes()?;
    let base = |installed: bool, version: String, version_valid: bool, vhci_residual: bool| UsbipStatus {
        externally_managed,
        replaceable,
        supported: true,
        installed,
        ready: false,
        version,
        version_valid,
        reboot_recommended: false,
        vhci_residual,
        residual_kind: String::new(),
        cleanup_pending_restart: false,
        cleanup_resume: false,
        cleanup_resume_op: String::new(),
        last_cleanup_error: String::new(),
        last_install_error: String::new(),
        install_pending_restart: false,
        install_resume: false,
        error_code: String::new(),
        attached_devices: Vec::new(),
        detail: String::new(),
    };
    let installation = match super::manager::find_installation() {
        Ok(installation) => installation,
        Err(detail) => {
            let registered_version = super::installed_usbip_version();
            let partially_installed = externally_managed || !matches!(registered_version, Ok(None));
            // A registration without a usable executable is also a partial uninstall and must go
            // through shared cleanup. A node that is only a ghost under Enum (an old uninstaller
            // leaves them) is not a controller: it blocks nothing and the next install sweeps it.
            let residual = !interfaces.is_empty()
                || nodes.iter().any(|node| node.present)
                || partially_installed;
            let mut status = base(
                partially_installed,
                registered_version.ok().flatten().unwrap_or_default(),
                false,
                residual,
            );
            status.error_code = detail_error_code(&detail);
            status.detail = detail;
            return Ok((status, Residual::Broken));
        }
    };
    let version_valid = super::supported_usbip_installed(Some(&installation.version));
    // Only a controller that is in the device tree can become a second one (nodes.rs): a ghost
    // of an older version and a disabled node under the other enumerator do not block. An enabled
    // present node beside the live controller, or a second live interface, does.
    let vhci_residual = super::device::transport_not_ready(&interfaces, &nodes);
    let kind = super::device::residual_kind(&interfaces, &nodes);
    if !version_valid {
        let detail = format!(
            "USBIP-SETUP-003: externally managed USB/IP is incompatible; choose whether to replace it in Windows Installed apps; only {REQUIRED_VERSION} is supported"
        );
        let mut status = base(true, installation.version, version_valid, vhci_residual);
        status.error_code = detail_error_code(&detail);
        status.detail = detail;
        return Ok((status, kind));
    }
    let mut status = base(true, installation.version, version_valid, vhci_residual);
    match super::exec::list_attached().await {
        Ok(attached_devices) => {
            // Residual instances break every usbip.exe operation; reflect that even if a race let
            // this probe succeed.
            status.ready = !vhci_residual;
            status.attached_devices = attached_devices;
        }
        Err(detail) => {
            status.error_code = detail_error_code(&detail);
            status.detail = detail;
        }
    }
    Ok((status, kind))
}

#[cfg(test)]
mod tests {
    use super::detail_error_code;

    #[test]
    fn extracts_only_well_shaped_codes() {
        assert_eq!(
            detail_error_code("USBIP-EXEC-006: the shared transport"),
            "USBIP-EXEC-006"
        );
        assert_eq!(
            detail_error_code("USBIP-SETUP-001: not installed"),
            "USBIP-SETUP-001"
        );
        assert_eq!(detail_error_code("USBIP-EXEC-006"), "USBIP-EXEC-006");
        // 非码前缀的 detail 必须得到空串，前端不得据此推断语义
        assert_eq!(
            detail_error_code("USB/IP passthrough is only supported"),
            ""
        );
        assert_eq!(detail_error_code("usbip-exec-006: lowercase"), "");
        assert_eq!(detail_error_code("USBIP-EXEC-6: short number"), "");
        assert_eq!(detail_error_code(""), "");
    }
}
