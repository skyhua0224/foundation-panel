//! Read-only VB-CABLE probe. The Panel runs it as a child so a stalled Windows
//! device or service query can be terminated independently of driver maintenance.

use std::path::Path;
use tokio::time::Instant;
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationKind};
use windows::Foundation::IPropertyValue;
use windows::Win32::Devices::DeviceAndDriverInstallation::{CM_PROB_DISABLED, DN_STARTED};
use windows::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use windows::Win32::System::Services::{
    OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT, SERVICE_DRIVER,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
};
use windows::core::{Array, HSTRING, Interface, Owned, PCWSTR, w};
use windows_collections::IIterable;

const PROBE_ARG: &str = "--vb-cable-probe";
const HARDWARE_IDS: &str = "System.Devices.HardwareIds";
const MANUFACTURER: &str = "System.Devices.DeviceManufacturer";
// DEVPKEY_Device_Class, DEVPKEY_Device_ProblemCode and DEVPKEY_Device_DevNodeStatus.
const CLASS: &str = "{A45C254E-DF1C-4EFD-8020-67D146A850E0} 9";
const PROBLEM: &str = "{4340A6C5-93FA-4706-972C-7B648008A5A7} 3";
const STATUS: &str = "{4340A6C5-93FA-4706-972C-7B648008A5A7} 2";

fn driver_running() -> windows::core::Result<Option<bool>> {
    unsafe {
        let manager = Owned::new(OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_CONNECT,
        )?);
        let service = match OpenServiceW(*manager, w!("VBAudioVACMME"), SERVICE_QUERY_STATUS) {
            Ok(handle) => Owned::new(handle),
            Err(error) if error.code() == ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let mut status = SERVICE_STATUS::default();
        QueryServiceStatus(*service, &mut status)?;
        Ok((status.dwServiceType.0 & SERVICE_DRIVER.0 != 0)
            .then_some(status.dwCurrentState == SERVICE_RUNNING))
    }
}

enum Node {
    Media { problem: u32, started: bool },
    Unverified,
}

fn has_hardware_id(ids: &[HSTRING]) -> bool {
    ids.iter()
        .any(|id| id.to_string_lossy().eq_ignore_ascii_case("VBAudioVACWDM"))
}

fn inspect_candidate(candidate: &DeviceInformation) -> windows::core::Result<Option<Node>> {
    let props = candidate.Properties()?;
    let property = |key: &str| props.Lookup(&HSTRING::from(key))?.cast::<IPropertyValue>();
    let mut ids = Array::new();
    property(HARDWARE_IDS)?.GetStringArray(&mut ids)?;
    if !has_hardware_id(&ids) {
        return Ok(None);
    }
    let matches = |key, expected| {
        property(key)
            .and_then(|p| p.GetString())
            .is_ok_and(|s| s.to_string_lossy().eq_ignore_ascii_case(expected))
    };
    let node = if !matches(CLASS, "MEDIA") || !matches(MANUFACTURER, "VB-Audio Software") {
        Node::Unverified
    } else {
        let problem = property(PROBLEM)?.GetUInt32()?;
        let started = problem == 0 && property(STATUS)?.GetUInt32()? & DN_STARTED.0 != 0;
        Node::Media { problem, started }
    };
    Ok(Some(node))
}

fn classify(nodes: &[Node], running: Option<bool>) -> &'static str {
    match (nodes, running) {
        ([], None) => "check-store",
        ([], Some(_)) => "pending",
        ([Node::Media { problem, .. }], _) if *problem == CM_PROB_DISABLED.0 => "disabled",
        (
            [
                Node::Media {
                    problem: 0,
                    started: true,
                },
            ],
            Some(true),
        ) => "installed",
        _ => "unavailable",
    }
}

fn live_state() -> windows::core::Result<&'static str> {
    let properties: IIterable<HSTRING> = [HARDWARE_IDS, MANUFACTURER, CLASS, PROBLEM, STATUS]
        .map(HSTRING::from)
        .to_vec()
        .into();
    // HardwareIds is a list. AQS finds candidates with a matching substring;
    // inspect_candidate checks for an exact list element before using the node.
    let candidates = DeviceInformation::FindAllAsyncWithKindAqsFilterAndAdditionalProperties(
        &HSTRING::from(r#"System.Devices.HardwareIds:~~"VBAudioVACWDM" AND System.Devices.Present:=System.StructuredQueryType.Boolean#True"#),
        &properties,
        DeviceInformationKind::Device,
    )?.join()?;
    let mut nodes = Vec::new();
    for index in 0..candidates.Size()? {
        if let Some(node) = inspect_candidate(&candidates.GetAt(index)?)? {
            nodes.push(node);
        }
    }
    Ok(classify(&nodes, driver_running()?))
}

pub(crate) fn handle_command() -> Option<i32> {
    if !std::env::args_os().skip(1).eq([PROBE_ARG]) {
        return None;
    }
    Some(match live_state() {
        Ok(state) => {
            println!("{state}");
            0
        }
        Err(error) => {
            eprintln!("VB-CABLE: {error}");
            1
        }
    })
}

// Match the entire original INF name, independent of the language of pnputil's field labels.
fn has_package(output: &[u8]) -> bool {
    static INF: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)^vbmmecable(?:64)?_(?:win[0-9]+|vista|xp|2003)\.inf$").unwrap()
    });
    crate::text_encoding::decode_console_bytes(output)
        .lines()
        .any(|line| {
            line.split_once(':')
                .is_some_and(|(_, value)| INF.is_match(value.trim()))
        })
}

async fn run(exe: &Path, arg: &str, deadline: Instant) -> Result<Vec<u8>, String> {
    use std::process::Stdio;
    let child = tokio::process::Command::new(exe)
        .arg(arg)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("VB-CABLE: could not start device check: {error}"))?;
    super::wait_for_device_probe(child, deadline.saturating_duration_since(Instant::now())).await
}

pub(super) async fn probe_device() -> Result<String, String> {
    let deadline = Instant::now() + super::DEVICE_PROBE_TIMEOUT;
    let exe = std::env::current_exe()
        .map_err(|error| format!("VB-CABLE: cannot locate Panel: {error}"))?;
    let stdout = run(&exe, PROBE_ARG, deadline).await?;
    match std::str::from_utf8(&stdout).map(str::trim) {
        Ok(state @ ("installed" | "disabled" | "unavailable" | "pending")) => Ok(state.into()),
        Ok("check-store") => {
            let tool = crate::elevation::system_binary("pnputil.exe");
            let packages = run(&tool, "/enum-drivers", deadline).await?;
            Ok(if has_package(&packages) {
                "pending"
            } else {
                "missing"
            }
            .into())
        }
        _ => Err("VB-CABLE: unexpected device check result".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_status_requires_the_live_driver_and_detects_disabled_devices() {
        let ready = || Node::Media {
            problem: 0,
            started: true,
        };
        assert_eq!(classify(&[], None), "check-store");
        assert_eq!(classify(&[], Some(false)), "pending");
        assert_eq!(classify(&[ready()], Some(true)), "installed");
        assert_eq!(classify(&[ready()], None), "unavailable");
        assert_eq!(classify(&[ready()], Some(false)), "unavailable");
        assert_eq!(
            classify(
                &[Node::Media {
                    problem: 22,
                    started: false
                }],
                None
            ),
            "disabled"
        );
        assert_eq!(
            classify(
                &[Node::Media {
                    problem: 28,
                    started: false
                }],
                Some(true)
            ),
            "unavailable"
        );
        assert_eq!(
            classify(
                &[Node::Media {
                    problem: 0,
                    started: false
                }],
                Some(true)
            ),
            "unavailable"
        );
        assert_eq!(classify(&[Node::Unverified], Some(true)), "unavailable");
        assert_eq!(classify(&[ready(), ready()], Some(true)), "unavailable");
    }

    #[test]
    fn hardware_id_is_an_exact_list_element_independent_of_audio_endpoints() {
        assert!(has_hardware_id(&[
            HSTRING::from("other"),
            HSTRING::from("vbaudiovacwdm")
        ]));
        for unrelated in [
            "VBAudioVACWDM-extra",
            "other-VBAudioVACWDM",
            "SWD\\MMDEVAPI",
        ] {
            assert!(!has_hardware_id(&[HSTRING::from(unrelated)]));
        }
    }

    #[test]
    fn original_inf_names_are_matched_as_whole_fields() {
        assert!(has_package(b"Original Name: vbMmeCable64_win7.inf\r\n"));
        assert!(has_package("原始名称: vbMmeCable64_win10.inf\n".as_bytes()));
        assert!(!has_package(b"Original Name: vendor-vbmmecable64_win10.inf\nOriginal Name: vbmmecable64_win10.inf.fake\n"));
        assert!(!has_package(b"Original Name: unrelated.inf\n"));
    }

    #[tokio::test]
    async fn driver_store_reads_the_last_package_and_rejects_failed_enumerations() {
        use std::process::Stdio;
        for code in [0, 1] {
            let child = tokio::process::Command::new(crate::elevation::system_binary(
                r"WindowsPowerShell\v1.0\powershell.exe",
            ))
            .args(["-NoProfile", "-NonInteractive", "-Command",
                &format!("[Console]::WriteLine('x' * 65536); [Console]::WriteLine('Original Name: vbMmeCable64_win7.inf'); exit {code}")])
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
            let output =
                super::super::wait_for_device_probe(child, super::super::DEVICE_PROBE_TIMEOUT)
                    .await;
            if code == 0 {
                assert!(has_package(&output.unwrap()));
            } else {
                assert!(output.is_err());
            }
        }
    }
}
