//! What the system and the host report about the native DualSense path.
//!
//! Nothing here installs or repairs anything. The host builds the DualSense
//! path in; the one thing it needs from the system is the signed usbip-win2
//! driver. The Panel reads that from the system (registered version, the
//! signature of the registered driver) and from the host (whether the native
//! path is compiled in and whether a controller is using it).

use serde::Serialize;

use super::tuning::{TuningSnapshot, fetch_snapshot};

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct HostStatus {
    pub reachable: bool,
    /// False for a host that answers but has no `/api/dualsense/tuning`.
    pub tuning_available: bool,
    pub native_compiled: bool,
    pub native_active: bool,
    pub active_controllers: u32,
    pub revision: u64,
    pub error_code: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct TransportStatus {
    pub supported: bool,
    pub installed: bool,
    pub version: String,
    pub pinned_version: String,
    pub version_supported: bool,
    pub ready: bool,
    /// `signed`, `unsigned`, `invalid`, `unknown`, or empty when not checked.
    pub signature: String,
    pub signer: String,
    pub error_code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DualSenseStatus {
    /// One of the states listed in `derive_state`.
    pub state: String,
    pub host: HostStatus,
    pub transport: TransportStatus,
}

/// Picks the single sentence the interface leads with. Order matters: the
/// first blocker the person can act on wins.
///
/// - `host_unreachable`: the Sunshine service does not answer.
/// - `host_unsupported`: the host is older than the native DualSense path.
/// - `native_absent`: this host build does not contain the native path.
/// - `transport_unsupported`: the operating system cannot host usbip-win2.
/// - `usbip_missing`, `usbip_version`, `usbip_unsigned`, `usbip_not_ready`.
/// - `active`: a controller is using the native path now.
/// - `ready`: everything is in place and no controller is connected.
pub(crate) fn derive_state(host: &HostStatus, transport: &TransportStatus) -> &'static str {
    if !host.reachable {
        "host_unreachable"
    } else if !host.tuning_available {
        "host_unsupported"
    } else if !host.native_compiled {
        "native_absent"
    } else if !transport.supported {
        "transport_unsupported"
    } else if !transport.installed {
        "usbip_missing"
    } else if !transport.version_supported {
        "usbip_version"
    } else if matches!(transport.signature.as_str(), "unsigned" | "invalid") {
        "usbip_unsigned"
    } else if !transport.ready {
        "usbip_not_ready"
    } else if host.native_active || host.active_controllers > 0 {
        "active"
    } else {
        "ready"
    }
}

/// Host fields from a tuning answer or the failure that replaced it.
pub(crate) fn host_status(answer: Result<TuningSnapshot, String>) -> HostStatus {
    match answer {
        Ok(snapshot) => HostStatus {
            reachable: true,
            tuning_available: true,
            native_compiled: snapshot.backend.native_path_compiled || snapshot.native_path_compiled,
            native_active: snapshot.backend.native_path_active,
            active_controllers: snapshot.backend.active_controllers,
            revision: snapshot.revision,
            error_code: String::new(),
        },
        Err(error) => {
            let code = error.split(':').next().unwrap_or_default().to_string();
            HostStatus {
                // An older host answers 404: it is there, it just lacks the route.
                reachable: code != "DS5-HOST-001",
                tuning_available: false,
                error_code: code,
                ..HostStatus::default()
            }
        }
    }
}

async fn transport_status() -> TransportStatus {
    let pinned_version = crate::usbip::USBIP_VERSION.to_string();
    let status = match crate::usbip::usbip_get_status().await {
        Ok(status) => status,
        Err(detail) => {
            return TransportStatus {
                supported: true,
                pinned_version,
                error_code: detail.split(':').next().unwrap_or_default().to_string(),
                ..TransportStatus::default()
            };
        }
    };
    let (signature, signer) = if status.installed && status.version_valid {
        let signature = crate::usbip::driver_signature().await;
        (signature.state, signature.signer)
    } else {
        (String::new(), String::new())
    };
    TransportStatus {
        supported: status.supported,
        installed: status.installed,
        version: status.version,
        pinned_version,
        version_supported: status.version_valid,
        ready: status.ready,
        signature,
        signer,
        error_code: status.error_code,
    }
}

#[tauri::command]
pub async fn dualsense_get_status() -> Result<DualSenseStatus, String> {
    let host = match (
        crate::sunshine::create_https_client(),
        crate::sunshine::get_sunshine_url().await,
    ) {
        (Ok(client), Ok(base)) => host_status(fetch_snapshot(&client, &base).await),
        _ => host_status(Err("DS5-HOST-001: the host address is unknown".to_string())),
    };
    let transport = transport_status().await;
    Ok(DualSenseStatus {
        state: derive_state(&host, &transport).to_string(),
        host,
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> HostStatus {
        HostStatus {
            reachable: true,
            tuning_available: true,
            native_compiled: true,
            ..HostStatus::default()
        }
    }

    fn transport() -> TransportStatus {
        TransportStatus {
            supported: true,
            installed: true,
            version: "0.9.8.1".into(),
            pinned_version: "0.9.8.1".into(),
            version_supported: true,
            ready: true,
            signature: "signed".into(),
            ..TransportStatus::default()
        }
    }

    #[test]
    fn a_complete_setup_is_ready_and_a_connected_pad_makes_it_active() {
        assert_eq!(derive_state(&host(), &transport()), "ready");
        let mut busy = host();
        busy.active_controllers = 1;
        assert_eq!(derive_state(&busy, &transport()), "active");
        let mut native = host();
        native.native_active = true;
        assert_eq!(derive_state(&native, &transport()), "active");
    }

    #[test]
    fn the_first_actionable_blocker_wins() {
        let mut unreachable = host();
        unreachable.reachable = false;
        assert_eq!(derive_state(&unreachable, &transport()), "host_unreachable");
        let mut old = host();
        old.tuning_available = false;
        assert_eq!(derive_state(&old, &TransportStatus::default()), "host_unsupported");
        let mut absent = host();
        absent.native_compiled = false;
        assert_eq!(derive_state(&absent, &transport()), "native_absent");

        let mut none = transport();
        none.installed = false;
        assert_eq!(derive_state(&host(), &none), "usbip_missing");
        let mut wrong = transport();
        wrong.version = "0.9.9.0".into();
        wrong.version_supported = false;
        assert_eq!(derive_state(&host(), &wrong), "usbip_version");
        for signature in ["unsigned", "invalid"] {
            let mut unsigned = transport();
            unsigned.signature = signature.into();
            assert_eq!(derive_state(&host(), &unsigned), "usbip_unsigned");
        }
        let mut broken = transport();
        broken.ready = false;
        assert_eq!(derive_state(&host(), &broken), "usbip_not_ready");
        let mut unsupported = transport();
        unsupported.supported = false;
        assert_eq!(derive_state(&host(), &unsupported), "transport_unsupported");
    }

    #[test]
    fn an_unreadable_signature_does_not_block_a_ready_transport() {
        let mut unknown = transport();
        unknown.signature = "unknown".into();
        assert_eq!(derive_state(&host(), &unknown), "ready");
    }

    #[test]
    fn host_status_tells_an_old_host_from_an_absent_one() {
        let old = host_status(Err("DS5-HOST-002: the host has no DualSense tuning".into()));
        assert!(old.reachable && !old.tuning_available);
        assert_eq!(old.error_code, "DS5-HOST-002");
        let gone = host_status(Err("DS5-HOST-001: the host is not reachable".into()));
        assert!(!gone.reachable);
        let ok = host_status(Ok(TuningSnapshot {
            revision: 4,
            native_path_compiled: true,
            backend: super::super::tuning::TuningBackend {
                native_path_compiled: true,
                native_path_active: true,
                active_controllers: 2,
            },
            ..TuningSnapshot::default()
        }));
        assert!(ok.reachable && ok.tuning_available && ok.native_compiled && ok.native_active);
        assert_eq!((ok.active_controllers, ok.revision), (2, 4));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "reads the real usbip-win2 installation of this Windows host"]
    async fn transport_status_reads_the_installed_driver() {
        let status = transport_status().await;
        println!("{status:?}");
        assert_eq!(status.pinned_version, crate::usbip::USBIP_VERSION);
    }
}
