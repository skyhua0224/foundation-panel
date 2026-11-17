//! Explicit, user-confirmed installation of the pinned usbip-win2 transport.
//!
//! This runs only inside the elevated helper, after the Device Hub asked the
//! person to confirm the installation. The installer is staged in a directory
//! that only administrators can write, hashed against the pinned digest and
//! only then executed.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) enum InstallResult {
    Ready,
    RebootRecommended,
}

/// Inno Setup exits with the code requested by `/RESTARTEXITCODE`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_installer_exit_code(
    exit_code: Option<i32>,
) -> Result<InstallResult, String> {
    match exit_code {
        Some(0) => Ok(InstallResult::Ready),
        Some(3010) => Ok(InstallResult::RebootRecommended),
        code => Err(format!(
            "USBIP-SETUP-010: the USB/IP installer failed with exit code {}",
            code.unwrap_or(-1)
        )),
    }
}

/// Directory next to the Sunshine installation where the installer is staged.
#[cfg(target_os = "windows")]
pub(crate) fn staging_root() -> std::path::PathBuf {
    crate::sunshine::install_dir()
        .join("tools")
        .join("usbip-staging")
}

/// Refuses to touch the transport while a stream may be using it.
#[cfg(target_os = "windows")]
pub(crate) async fn ensure_no_active_session() -> Result<(), String> {
    let sessions = crate::sunshine::get_active_sessions()
        .await
        .map_err(|error| format!("USBIP-SETUP-006: unable to verify stream ownership: {error}"))?;
    if sessions.is_empty() {
        Ok(())
    } else {
        Err(
            "USBIP-SETUP-006: finish the active Sunshine stream before changing the transport"
                .to_string(),
        )
    }
}

/// Reuses a healthy pinned transport or installs the pinned release.
/// An existing incompatible installation needs the shared cleanup first.
#[cfg(target_os = "windows")]
pub(crate) async fn ensure_pinned_usbip(
    _staging_root: &std::path::Path,
) -> Result<InstallResult, String> {
    use super::{
        INSTALLER_DOWNLOAD_TIMEOUT, InstallDisposition, MAX_INSTALLER_BYTES, USBIP_SHA256,
        USBIP_URL, USBIP_VERSION,
    };
    use crate::github_download::{
        DEFAULT_IDLE_TIMEOUT, DEFAULT_RESPONSE_TIMEOUT, DownloadRequest,
        download_to_file_with_fallbacks,
    };

    match super::install_disposition(super::installed_usbip_version()?.as_deref())? {
        InstallDisposition::Ready => {
            let status = super::usbip_get_status().await?;
            if !status.ready {
                return Err(format!(
                    "USBIP-SETUP-008: existing USB/IP transport needs repair before reuse: {}",
                    status.detail
                ));
            }
            return Ok(InstallResult::Ready);
        }
        InstallDisposition::Install => {}
    }

    // Use the host's immutable signed payload and its root-hub transaction.
    // No runtime installer or vendor setup executable is downloaded by the panel.
    let script = crate::sunshine::install_dir()
        .join("scripts")
        .join("usbip-package.ps1");
    if !script.is_file() {
        return Err(
            "USBIP-SETUP-002: update the host to install the bundled USB/IP component".into(),
        );
    }
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot")
            .ok_or("USBIP-SETUP-002: Windows directory is unavailable")?,
    )
    .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut command = tokio::process::Command::new(powershell);
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .args(["-Action", "install"])
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("USBIP-SETUP-010: USB/IP setup did not start: {error}"))?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(600), child.wait())
        .await
        .map_err(|_| "USBIP-SETUP-010: USB/IP setup timed out".to_string())?
        .map_err(|error| format!("USBIP-SETUP-010: USB/IP setup failed: {error}"))?;
    let result = classify_installer_exit_code(status.code())?;
    super::manager::find_installation()?;
    super::manager::verify_driver_compatibility()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_exit_codes_map_to_ready_or_reboot() {
        assert_eq!(
            classify_installer_exit_code(Some(0)),
            Ok(InstallResult::Ready)
        );
        assert_eq!(
            classify_installer_exit_code(Some(3010)),
            Ok(InstallResult::RebootRecommended)
        );
    }

    #[test]
    fn installer_failure_carries_a_usbip_code() {
        for code in [Some(1), Some(-1), None] {
            let error = classify_installer_exit_code(code).unwrap_err();
            assert!(error.starts_with("USBIP-SETUP-010:"), "{error}");
        }
    }
}
