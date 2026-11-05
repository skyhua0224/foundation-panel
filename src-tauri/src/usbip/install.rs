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
        Err("USBIP-SETUP-006: finish the active Sunshine stream before changing the transport"
            .to_string())
    }
}

/// Reuses a healthy pinned transport or installs the pinned release.
/// An existing incompatible installation needs the shared cleanup first.
#[cfg(target_os = "windows")]
pub(crate) async fn ensure_pinned_usbip(
    staging_root: &std::path::Path,
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

    let installer = tempfile::Builder::new()
        .prefix("usbip-installer-")
        .suffix(".exe")
        .tempfile_in(staging_root)
        .map_err(|error| {
            format!("USBIP-SETUP-010: unable to stage the USB/IP installer: {error}")
        })?
        .into_temp_path();
    let installer_path = installer.to_path_buf();

    download_to_file_with_fallbacks(
        DownloadRequest {
            url: USBIP_URL,
            destination: &installer_path,
            expected_size: None,
            expected_sha256: None,
            max_size: Some(MAX_INSTALLER_BYTES),
            user_agent: "foundation-sunshine-control-panel",
            connect_timeout: std::time::Duration::from_secs(10),
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            overall_timeout: Some(INSTALLER_DOWNLOAD_TIMEOUT),
        },
        |_| {},
    )
    .await
    .map_err(|error| format!("USBIP-SETUP-010: download failed: {error}"))?;

    let actual = crate::native_components::install::sha256_file(&installer_path, MAX_INSTALLER_BYTES)
        .map_err(|error| format!("USBIP-SETUP-010: cannot hash the USB/IP installer: {error}"))?;
    if actual != USBIP_SHA256 {
        return Err(format!(
            "USBIP-SETUP-010: expected USB/IP SHA-256 {USBIP_SHA256}, got {actual}"
        ));
    }

    let mut command = tokio::process::Command::new(&installer_path);
    command
        .args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/RESTARTEXITCODE=3010",
            "/SP-",
        ])
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("USBIP-SETUP-010: the USB/IP installer did not start: {error}"))?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(600), child.wait())
        .await
        .map_err(|_| "USBIP-SETUP-010: the USB/IP installer timed out".to_string())?
        .map_err(|error| format!("USBIP-SETUP-010: the USB/IP installer failed: {error}"))?;
    let result = classify_installer_exit_code(status.code())?;
    if super::installed_usbip_version()?.as_deref() != Some(USBIP_VERSION) {
        return Err(format!(
            "USBIP-SETUP-010: the installer finished but version {USBIP_VERSION} is not registered"
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_exit_codes_map_to_ready_or_reboot() {
        assert_eq!(classify_installer_exit_code(Some(0)), Ok(InstallResult::Ready));
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
