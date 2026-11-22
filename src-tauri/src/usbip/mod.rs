//! Narrow Windows USB/IP client management for the Device Hub.
//!
//! The renderer can discover devices from a standard USB/IP exporter, then
//! explicitly attach or detach them. Privileged operations are allowlisted and
//! revalidated in a short-lived elevated copy of this executable.

mod commands;
mod config;
#[cfg(target_os = "windows")]
mod devnode;
mod diagnostics;
mod device;
mod elevated;
mod exec;
mod install;
mod journal;
mod manager;
mod memo;
mod nodes;
mod signature;
mod status;

use serde::{Deserialize, Serialize};

use manager::REQUIRED_VERSION;
pub(crate) use signature::driver_signature;
const DEFAULT_TCP_PORT: u16 = 3240;
const MAX_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_ELEVATED_RESPONSE_BYTES: usize = 16 * 1024;
const ELEVATED_ARG: &str = "--elevated-usbip";
const ELEVATED_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const MAX_INSTALLER_BYTES: u64 = 48 * 1024 * 1024;
const INSTALLER_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

static OPERATION_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UsbipRemoteDevice {
    pub bus_id: String,
    pub description: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UsbipAttachedDevice {
    pub port: u8,
    pub speed: String,
    pub description: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub remote_bus_id: String,
    pub serial: String,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct UsbipStatus {
    pub supported: bool,
    pub installed: bool,
    pub ready: bool,
    pub version: String,
    pub version_valid: bool,
    pub reboot_recommended: bool,
    pub vhci_residual: bool,
    /// What the leftover is: "" (none), "stale" (a working controller with older ones beside it; removing those is
    /// enough) or "broken" (clean the transport and install it again).
    pub residual_kind: String,
    /// A cleanup was accepted by Windows and finishes at the next restart; the restart has not happened yet.
    pub cleanup_pending_restart: bool,
    /// Windows has restarted since a cleanup waited for it and the leftover is still there: continue the cleanup.
    pub cleanup_resume: bool,
    /// Which operation to continue: "cleanup" (the whole cleanup) or "stale" (the repair that keeps the live controller).
    pub cleanup_resume_op: String,
    /// The last cleanup's error line as the helper answered it (codes and all), kept for the diagnostics.
    pub last_cleanup_error: String,
    /// The last install/replace error line with the underlying classification and code, kept for the diagnostics.
    pub last_install_error: String,
    /// An install failed because Windows has a pending restart; retry after the restart.
    pub install_pending_restart: bool,
    /// Windows has restarted since an install waited for a pending reboot: retry the install once.
    pub install_resume: bool,
    /// 结构化错误码（"USBIP-SETUP-003" 等），取自 detail 的码前缀；
    /// 前端按码判断语义，不得解析 detail 文本。
    pub error_code: String,
    pub attached_devices: Vec<UsbipAttachedDevice>,
    pub detail: String,
    pub externally_managed: bool,
    /// An external installation the person may ask to replace with the host's own package (host `status`).
    pub replaceable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum ElevatedRequest {
    Install,
    Cleanup,
    /// Remove the older controllers beside the working one; the live controller stays.
    RemoveStale,
    /// The person confirmed replacing an external USB/IP installation with the host's package.
    ReplaceLegacy,
    Attach {
        remote: String,
        tcp_port: u16,
        bus_id: String,
    },
    Detach {
        port: u8,
    },
}

impl ElevatedRequest {
    fn timeout(&self) -> std::time::Duration {
        match self {
            // The pinned download has a 30-minute overall budget and the signed
            // installer has a separate 10-minute budget. Keep the IPC alive
            // through both, with a small hand-off margin.
            Self::Install => std::time::Duration::from_secs(45 * 60),
            // The silent uninstaller plus the devnode sweep stay well below a
            // single Inno install, but leave headroom for slow driver teardown.
            Self::Cleanup => std::time::Duration::from_secs(15 * 60),
            // Each node is asked for a few seconds over the polling budget of a cleanup, without an uninstaller.
            Self::RemoveStale => std::time::Duration::from_secs(5 * 60),
            // The host's helper allows its uninstall and install ten minutes in total; leave the same headroom.
            Self::ReplaceLegacy => std::time::Duration::from_secs(15 * 60),
            Self::Attach { .. } | Self::Detach { .. } => {
                COMMAND_TIMEOUT + std::time::Duration::from_secs(5)
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ElevatedResponse {
    success: bool,
    port: Option<u8>,
    message: String,
}

#[cfg(target_os = "windows")]
pub(crate) use manager::uninstall_entries as usbip_uninstall_entries;
pub(crate) use manager::{
    INSTALLER_SHA256 as USBIP_SHA256, INSTALLER_URL as USBIP_URL, InstallDisposition,
    PINNED_VERSION as USBIP_VERSION, install_disposition,
    installed_version as installed_usbip_version,
    supported_version_installed as supported_usbip_installed,
};

fn validate_remote(remote: &str) -> Result<String, String> {
    let value = remote.trim();
    if value.is_empty() || value.len() > 253 {
        return Err("USBIP-INPUT-001: enter a valid exporter hostname or IP address".to_string());
    }
    let unwrapped = value
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(value);
    if unwrapped.parse::<std::net::IpAddr>().is_ok() {
        return Ok(unwrapped.to_string());
    }
    let valid_hostname = !unwrapped.contains(':')
        && unwrapped.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && label
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
                && label
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
        });
    if !valid_hostname {
        return Err("USBIP-INPUT-001: exporter hostname or IP address is invalid".to_string());
    }
    Ok(unwrapped.to_string())
}

fn validate_bus_id(bus_id: &str) -> Result<String, String> {
    let value = bus_id.trim();
    if value.is_empty()
        || value.len() > 64
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | ':' | '_'))
        })
    {
        return Err("USBIP-INPUT-002: the remote USB bus ID is invalid".to_string());
    }
    Ok(value.to_string())
}

fn validate_tcp_port(tcp_port: Option<u16>) -> Result<u16, String> {
    let value = tcp_port.unwrap_or(DEFAULT_TCP_PORT);
    if value < 1024 {
        return Err("USBIP-INPUT-003: the TCP port must be between 1024 and 65535".to_string());
    }
    Ok(value)
}

// Re-export the tauri commands and the elevated-helper entry point so callers
// keep spelling `usbip::` with the same paths they used against the single-file
// module. Glob re-exports carry the `#[tauri::command]`-generated hidden items
// that generate_handler! resolves.
pub use commands::*;
pub use config::*;
#[cfg(target_os = "windows")]
pub(crate) use elevated::try_handle_elevated_command;
pub use status::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_only_host_and_bus_id_characters() {
        assert_eq!(validate_remote(" 192.0.2.20 ").unwrap(), "192.0.2.20");
        assert_eq!(validate_remote("[fe80::1]").unwrap(), "fe80::1");
        assert_eq!(validate_remote("usb-host.local").unwrap(), "usb-host.local");
        assert!(validate_remote("host name").is_err());
        assert!(validate_remote("host&calc").is_err());
        assert!(validate_remote("-host.local").is_err());
        assert_eq!(validate_bus_id("3-3.2").unwrap(), "3-3.2");
        assert!(validate_bus_id("3-3 /tmp").is_err());
    }

    #[test]
    fn replace_request_has_its_own_allowlisted_operation() {
        let encoded = serde_json::to_string(&ElevatedRequest::ReplaceLegacy).unwrap();
        assert_eq!(encoded, r#"{"operation":"replace_legacy"}"#);
        assert!(matches!(
            serde_json::from_str::<ElevatedRequest>(&encoded),
            Ok(ElevatedRequest::ReplaceLegacy)
        ));
        // Cleanup of an external installation stays refused; the replacement is a different, confirmed operation.
        assert!(serde_json::from_str::<ElevatedRequest>(r#"{"operation":"replace"}"#).is_err());
    }

    #[test]
    fn stale_removal_is_its_own_allowlisted_operation() {
        let encoded = serde_json::to_string(&ElevatedRequest::RemoveStale).unwrap();
        assert_eq!(encoded, r#"{"operation":"remove_stale"}"#);
        assert!(matches!(
            serde_json::from_str::<ElevatedRequest>(&encoded),
            Ok(ElevatedRequest::RemoveStale)
        ));
    }

    #[test]
    fn validates_usbip_win2_tcp_port_range() {
        assert_eq!(validate_tcp_port(None).unwrap(), DEFAULT_TCP_PORT);
        assert!(validate_tcp_port(Some(1023)).is_err());
        assert_eq!(validate_tcp_port(Some(1024)).unwrap(), 1024);
        assert_eq!(validate_tcp_port(Some(u16::MAX)).unwrap(), u16::MAX);
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "requires a supported USB/IP transport on the local Windows host"]
    async fn installed_transport_reports_ready() {
        let status = usbip_get_status().await.unwrap();
        assert!(status.installed);
        assert!(status.version_valid);
        assert!(status.ready, "{}", status.detail);
        println!("USB/IP installed status: {status:?}");
        if std::env::var("USBIP_EXPECT_EXTERNAL").as_deref() == Ok("1") {
            assert!(status.externally_managed);
            let error = device::cleanup_broken_transport().await.unwrap_err();
            assert!(error.starts_with("USBIP-CLEAN-005:"), "{error}");
        }
    }

    /// The 0.9.7.7 leftover: run in an elevated session after the old uninstaller left a host-controller node behind.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "changes the machine: removes usbip-win2 host-controller devnodes; needs an elevated session"]
    async fn sweep_removes_every_leftover_controller() {
        let interfaces = device::enumerate_vhci_interfaces().unwrap();
        let before = device::enumerate_transport_nodes().unwrap();
        println!("before: interfaces={interfaces:?} nodes={before:?}");
        assert!(
            !before.is_empty() || !interfaces.is_empty(),
            "the test needs a leftover controller to remove: install and uninstall 0.9.7.7 first"
        );
        let finish = device::sweep_devnodes(false).await;
        let after = device::enumerate_transport_nodes().unwrap();
        println!("sweep: {finish:?}; after: nodes={after:?}");
        let finish = finish.expect("the sweep must not fail");
        if finish == device::Finish::Done {
            assert!(after.is_empty(), "nothing may be left: {after:?}");
        } else {
            assert!(after.iter().all(|node| node.present), "{after:?}");
        }
    }

    /// A working install with an older controller beside it (both may be live): one controller stays, the rest go.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "changes the machine: removes usbip-win2 host-controller devnodes; needs an elevated session"]
    async fn removing_the_stale_controller_keeps_one_live_controller() {
        let interfaces = device::enumerate_vhci_interfaces().unwrap();
        assert!(!interfaces.is_empty(), "needs a live controller: {interfaces:?}");
        let before = device::enumerate_transport_nodes().unwrap();
        println!("before: interfaces={interfaces:?} nodes={before:?} kind={:?}", device::residual_kind(&interfaces, &before));
        assert_eq!(
            device::residual_kind(&interfaces, &before),
            device::Residual::Stale,
            "the test needs a live controller with an older one beside it"
        );
        let finish = device::remove_stale_nodes().await.expect("the repair must not fail");
        let interfaces_after = device::enumerate_vhci_interfaces().unwrap();
        let after = device::enumerate_transport_nodes().unwrap();
        println!("repair: {finish:?}; interfaces={interfaces_after:?} nodes={after:?}");
        assert_eq!(interfaces_after.len(), 1, "exactly one live controller must stay");
        assert!(!device::transport_not_ready(&interfaces_after, &after));
    }

    /// Writes the diagnostics file for real and checks that every section is there (read only; shows the file in Explorer).
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "reads the local USB/IP state and writes the diagnostics file"]
    async fn exports_the_diagnostics_file_with_every_section() {
        let path = diagnostics::export().await.expect("the export must work");
        let text = std::fs::read_to_string(&path).unwrap();
        println!("{path} ({} bytes)", text.len());
        println!("{text}");
        for section in [
            "== Windows",
            "== Panel status",
            "== Last cleanup",
            "== Last install",
            "== Host-controller devices",
            "== Driver services",
            "== Registered installations",
            "== Driver packages",
            "== Device installation log",
        ] {
            assert!(text.contains(section), "missing {section}");
        }
    }

    /// What the panel reports for the machine as it is (read only).
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "reads the local USB/IP state"]
    async fn prints_the_status_the_panel_would_show() {
        let status = usbip_get_status().await.unwrap();
        println!("{}", serde_json::to_string_pretty(&status).unwrap());
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "requires a supported USB/IP transport on the local Windows host"]
    async fn unreachable_exporter_fails_cleanly() {
        let error = usbip_list_remote("127.0.0.1".to_string(), Some(43241))
            .await
            .unwrap_err();
        assert!(error.starts_with("USBIP-EXEC-004:"), "{error}");
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "requires a supported USB/IP transport on the local Windows host"]
    async fn discovers_a_device_from_a_standard_usbip_exporter() {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let tcp_port = listener.local_addr().unwrap().port();
        let exporter = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 8];
            socket.read_exact(&mut request).unwrap();
            assert_eq!(&request[..4], &[0x01, 0x11, 0x80, 0x05]);

            let mut reply = Vec::new();
            reply.extend_from_slice(&0x0111u16.to_be_bytes());
            reply.extend_from_slice(&0x0005u16.to_be_bytes());
            reply.extend_from_slice(&0u32.to_be_bytes());
            reply.extend_from_slice(&1u32.to_be_bytes());
            let mut path = [0u8; 256];
            path[..18].copy_from_slice(b"/sys/devices/mock1");
            reply.extend_from_slice(&path);
            let mut bus_id = [0u8; 32];
            bus_id[..3].copy_from_slice(b"1-1");
            reply.extend_from_slice(&bus_id);
            reply.extend_from_slice(&1u32.to_be_bytes());
            reply.extend_from_slice(&2u32.to_be_bytes());
            reply.extend_from_slice(&3u32.to_be_bytes());
            reply.extend_from_slice(&0x054cu16.to_be_bytes());
            reply.extend_from_slice(&0x0ce6u16.to_be_bytes());
            reply.extend_from_slice(&0x0100u16.to_be_bytes());
            reply.extend_from_slice(&[0, 0, 0, 1, 1, 1]);
            reply.extend_from_slice(&[3, 0, 0, 0]);
            socket.write_all(&reply).unwrap();
        });

        let devices = usbip_list_remote("127.0.0.1".to_string(), Some(tcp_port))
            .await
            .unwrap();
        exporter.join().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].bus_id, "1-1");
        assert!(!devices[0].description.is_empty());
    }
}
