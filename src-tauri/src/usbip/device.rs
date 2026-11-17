//! Residual VHCI device detection and cleanup.
//!
//! The pinned USB/IP transport leaves ghost VHCI devnodes behind when the
//! vendor uninstaller is interrupted or the upgrade path skips cleanup. This
//! module enumerates those devices the same way `usbip.exe` does and removes
//! the leftovers the installer tolerates but usbip.exe refuses to.

#[cfg(target_os = "windows")]
const VHCI_INTERFACE_GUID: windows::core::GUID =
    windows::core::GUID::from_u128(0xB4030C06_DC5F_4FCC_87EB_E5515A0935C0);

#[cfg(target_os = "windows")]
const CLEANUP_POLL_ATTEMPTS: usize = 30;
#[cfg(target_os = "windows")]
const INNO_UNINSTALL_FLAGS: &str = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART";

/// usbip.exe fails with "Multiple instances of VHCI device interface found"
/// when more than one present device interface answers this GUID. Enumerating
/// the same list here detects the leftover-devnode state without running it.
#[cfg(target_os = "windows")]
pub(super) fn enumerate_vhci_interfaces() -> Result<Vec<String>, String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CM_Get_Device_Interface_List_SizeW,
        CM_Get_Device_Interface_ListW, CR_BUFFER_SMALL, CR_SUCCESS,
    };

    // PRESENT matches what usbip.exe itself enumerates, so this reports exactly
    // the state that breaks it. Non-present interfaces are deliberately not
    // counted: they do not fail usbip.exe, and CM_Locate_DevNodeW cannot remove
    // them, so counting them would both disable a working transport and make
    // the sweep's final emptiness check unsatisfiable.
    // The size query and the list query are not atomic: interfaces arriving in
    // between make the list call report CR_BUFFER_SMALL. Device removal during
    // cleanup is exactly when this races, so retry instead of failing.
    const MAX_ENUM_RETRIES: usize = 5;
    for _ in 0..MAX_ENUM_RETRIES {
        let mut length = 0u32;
        let result = unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut length,
                &VHCI_INTERFACE_GUID,
                None,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if result != CR_SUCCESS {
            return Err(format!(
                "USBIP-CLEAN-002: unable to measure the VHCI device interface list (config error {})",
                result.0
            ));
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        let mut buffer = vec![0u16; length as usize];
        let result = unsafe {
            CM_Get_Device_Interface_ListW(
                &VHCI_INTERFACE_GUID,
                None,
                &mut buffer,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if result == CR_BUFFER_SMALL {
            continue;
        }
        if result != CR_SUCCESS {
            return Err(format!(
                "USBIP-CLEAN-002: unable to list the VHCI device interfaces (config error {})",
                result.0
            ));
        }
        return Ok(split_multi_sz(&buffer));
    }
    Err(
        "USBIP-CLEAN-002: the VHCI device interface list kept changing while enumerating"
            .to_string(),
    )
}

#[cfg(target_os = "windows")]
fn split_multi_sz(buffer: &[u16]) -> Vec<String> {
    buffer
        .split(|&unit| unit == 0)
        .filter(|unit| !unit.is_empty())
        .map(|unit| String::from_utf16_lossy(unit))
        .collect()
}

/// Device interface symbolic links are laid out as
/// `\\?\ROOT#USB#0001#{interface-guid}`; the portion before the GUID brace is
/// the device instance ID with `#` separators.
fn instance_id_from_interface_path(path: &str) -> Option<String> {
    let body = path.strip_prefix(r"\\?\")?;
    let head = body.split_once('{')?.0;
    let head = head.strip_suffix('#')?;
    if head.is_empty() {
        return None;
    }
    Some(head.replace('#', "\\"))
}

fn is_vhci_hardware_id(value: &str) -> bool {
    value.eq_ignore_ascii_case(r"ROOT\USBIP_WIN2\UDE")
}

/// CONFIGFLAG_DISABLED from the Windows device-installation flags. A disabled
/// devnode remains under Enum and does not publish a present VHCI interface, so
/// usbip.exe does not treat it as a second host controller.
const CONFIGFLAG_DISABLED: u32 = 0x0000_0001;

/// One usbip-win2 host-controller devnode. Older installs use ROOT\USB; a
/// later root uses ROOT\ALKAIDUSB. Both keep the same hardware ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransportNode {
    pub instance_id: String,
    pub disabled: bool,
}

/// Whether the transport must be reported not ready.
///
/// usbip.exe fails when more than one present VHCI interface exists. An enabled
/// registry node with the usbip-win2 hardware ID and no present interface is the
/// phantom an interrupted uninstall leaves behind, and that still blocks. A
/// disabled node does not: it is the previous enumerator (ROOT\USB after the
/// controller moved to ROOT\ALKAIDUSB, or the reverse) and the single present
/// interface is the live controller. No recorded node and no present interface
/// stays clear, matching the previous check.
pub(super) fn transport_not_ready(interfaces: &[String], nodes: &[TransportNode]) -> bool {
    if interfaces.len() > 1 {
        return true;
    }
    if interfaces.is_empty() {
        return !nodes.is_empty();
    }
    let live = instance_id_from_interface_path(&interfaces[0]);
    nodes.iter().any(|node| {
        !node.disabled
            && live
                .as_ref()
                .is_none_or(|id| !node.instance_id.eq_ignore_ascii_case(id))
    })
}

fn registry_optional<T>(result: std::io::Result<T>) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "USBIP-CLEAN-002: cannot inspect VHCI registry: {error}"
        )),
    }
}
/// Present interfaces are sufficient for normal operation checks, but an
/// interrupted uninstall can leave a phantom ROOT\USB node with no interface.
/// Read its immutable hardware ID so cleanup can remove only usbip-win2 nodes.
#[cfg(target_os = "windows")]
pub(super) fn enumerate_vhci_devnodes() -> Result<Vec<String>, String> {
    Ok(read_transport_nodes(r"ROOT\USB")?
        .into_iter()
        .map(|node| node.instance_id)
        .collect())
}

/// Host-controller nodes under either enumerator. Cleanup still removes only
/// ROOT\USB phantoms plus whatever interface is present; this list is for the
/// readiness decision and must see a controller that lives at ROOT\ALKAIDUSB.
#[cfg(target_os = "windows")]
pub(super) fn enumerate_transport_nodes() -> Result<Vec<TransportNode>, String> {
    let mut nodes = read_transport_nodes(r"ROOT\USB")?;
    nodes.extend(read_transport_nodes(r"ROOT\ALKAIDUSB")?);
    Ok(nodes)
}

#[cfg(target_os = "windows")]
fn read_transport_nodes(enumerator: &str) -> Result<Vec<TransportNode>, String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = format!(r"SYSTEM\CurrentControlSet\Enum\{enumerator}");
    let Some(root) = registry_optional(hklm.open_subkey_with_flags(&path, KEY_READ))? else {
        return Ok(Vec::new());
    };
    let mut nodes = Vec::new();
    for name in root.enum_keys() {
        let name = name
            .map_err(|error| format!("USBIP-CLEAN-002: cannot enumerate VHCI registry: {error}"))?;
        let Some(key) = registry_optional(root.open_subkey_with_flags(&name, KEY_READ))? else {
            continue;
        };
        let Some(hardware_ids) = registry_optional(key.get_value::<Vec<String>, _>("HardwareID"))?
        else {
            continue;
        };
        if hardware_ids.iter().any(|value| is_vhci_hardware_id(value)) {
            let flags = registry_optional(key.get_value::<u32, _>("ConfigFlags"))?.unwrap_or(0);
            nodes.push(TransportNode {
                instance_id: format!(r"{enumerator}\{name}"),
                disabled: (flags & CONFIGFLAG_DISABLED) != 0,
            });
        }
    }
    Ok(nodes)
}

/// Splits a registry command line such as `"C:\dir\unins000.exe" /flag` into
/// the executable and its arguments. The quoting rules (double quotes group
/// whitespace, a quote toggles quoting, backslashes immediately before a
/// quote are half-escaped) come from the OS `CommandLineToArgvW`, matching
/// what Inno Setup and Windows itself assume when parsing the same string.
#[cfg(target_os = "windows")]
fn split_executable_command(command: &str) -> Option<(String, Vec<String>)> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::UI::Shell::CommandLineToArgvW;
    use windows::core::PCWSTR;

    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let mut wide: Vec<u16> = command.encode_utf16().collect();
    wide.push(0);
    let mut argument_count = 0;
    let arguments_ptr =
        unsafe { CommandLineToArgvW(PCWSTR::from_raw(wide.as_ptr()), &mut argument_count) };
    if arguments_ptr.is_null() || argument_count <= 0 {
        return None;
    }
    let arguments: Vec<String> = unsafe {
        std::slice::from_raw_parts(arguments_ptr, argument_count as usize)
            .iter()
            .map(|argument| {
                let mut length = 0usize;
                while *argument.0.add(length) != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(argument.0, length))
            })
            .collect()
    };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(arguments_ptr.cast())));
    }
    let executable = arguments.first()?.clone();
    Some((executable, arguments[1..].to_vec()))
}

#[cfg(target_os = "windows")]
fn find_usbip_uninstall_string() -> Result<Option<(String, bool)>, String> {
    Ok(super::usbip_uninstall_entries()?
        .into_iter()
        .find_map(|entry| {
            entry
                .key
                .get_value::<String, _>("UninstallString")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|value| (value, entry.slim))
        }))
}

/// Runs inside the elevated helper. Order matters: the vendor uninstaller is
/// the only supported way to retire the driver packages, service registration,
/// and scheduled task; the devnode sweep afterwards only has to catch the
/// nodes the vendor uninstaller silently failed to remove. An uninstaller
/// failure never skips the sweep — leftover devnodes are the hard blocker for
/// the next install — but it is still reported so the user can retry.
#[cfg(target_os = "windows")]
pub(super) async fn cleanup_broken_transport() -> Result<(), String> {
    super::install::ensure_no_active_session()
        .await
        .map_err(|_| {
            "USBIP-CLEAN-001: finish the active Sunshine stream before cleaning up the transport"
                .to_string()
        })?;
    let helper = crate::sunshine::install_dir()
        .join("scripts")
        .join("usbip-package.ps1");
    if helper.is_file() {
        // The same helper serializes install/uninstall and completes the hub
        // guard on cancellation. Never start a native sweep beside that child.
        super::install::run_host_package("uninstall").await?;
        return Ok(());
    }
    let slim = super::usbip_uninstall_entries()?
        .iter()
        .any(|entry| entry.slim);
    let mut uninstall_error = None;
    if !super::usbip_uninstall_entries()?.is_empty() {
        if let Err(error) = run_registered_uninstaller().await {
            uninstall_error = Some(error);
        }
    }
    if slim {
        // A standalone slim package's registered helper performs the entire
        // cleanup. On timeout it still owns the lifecycle mutex.
        return uninstall_error.map_or(Ok(()), Err);
    }
    remove_vhci_devnodes().await?;
    remove_orphaned_driver_service("usbip2_ude", "usbip2_ude.sys")?;
    remove_orphaned_driver_service("usbip2_filter", "usbip2_filter.sys")?;
    uninstall_error.map_or(Ok(()), Err)
}

#[cfg(target_os = "windows")]
async fn run_registered_uninstaller() -> Result<(), String> {
    use std::process::Stdio;

    let (uninstall_string, slim) = find_usbip_uninstall_string()?.ok_or_else(|| {
        "USBIP-CLEAN-001: the USB/IP uninstaller registration is missing".to_string()
    })?;
    let (executable, arguments) = split_executable_command(&uninstall_string).ok_or_else(|| {
        "USBIP-CLEAN-001: the USB/IP uninstaller registration is invalid".to_string()
    })?;
    let mut command = tokio::process::Command::new(&executable);
    command.args(arguments);
    // PowerShell's registered slim command already includes -Action uninstall.
    // Inno flags are valid only for the legacy upstream uninstaller.
    if !slim {
        command.args(INNO_UNINSTALL_FLAGS.split_whitespace());
    }
    command
        // The slim helper must complete its guard transaction after UI cancellation.
        // panel-usbip-tokio-20261117, Command::kill_on_drop.
        .kill_on_drop(!slim)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    let child = command.spawn().map_err(|error| {
        format!("USBIP-CLEAN-001: unable to start the USB/IP uninstaller: {error}")
    })?;
    let finished = tokio::time::timeout(std::time::Duration::from_secs(10 * 60), async {
        let mut child = child;
        child.wait().await
    })
    .await;
    match finished {
        // Inno uninstallers exit 0 on success; any non-zero code means the
        // user cancelled or a fatal error occurred.
        Ok(Ok(status)) if status.success() || (slim && status.code() == Some(3010)) => {}
        Ok(Ok(status)) => {
            return Err(format!(
                "USBIP-CLEAN-001: the USB/IP uninstaller exited with {status}"
            ));
        }
        Ok(Err(error)) => {
            return Err(format!(
                "USBIP-CLEAN-001: the USB/IP uninstaller failed: {error}"
            ));
        }
        Err(_) => {
            return Err("USBIP-CLEAN-003: the USB/IP uninstaller timed out".to_string());
        }
    }
    wait_for_uninstall_registration_gone().await
}

#[cfg(target_os = "windows")]
async fn wait_for_uninstall_registration_gone() -> Result<(), String> {
    for _ in 0..CLEANUP_POLL_ATTEMPTS {
        if super::usbip_uninstall_entries()?.is_empty() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Err("USBIP-CLEAN-001: the USB/IP uninstaller did not remove its registration".to_string())
}

#[cfg(target_os = "windows")]
async fn remove_vhci_devnodes() -> Result<(), String> {
    // PnP teardown is asynchronous; reuse the full cleanup polling budget so a
    // slow but successful device removal is not reported as a failure.
    let mut last_error = None;
    for _ in 0..CLEANUP_POLL_ATTEMPTS {
        let interfaces = enumerate_vhci_interfaces()?;
        let mut instance_ids = enumerate_vhci_devnodes()?;
        instance_ids.extend(
            interfaces
                .iter()
                .filter_map(|path| instance_id_from_interface_path(path)),
        );
        instance_ids.sort_unstable();
        instance_ids.dedup();
        if instance_ids.is_empty() {
            return Ok(());
        }
        for instance_id in instance_ids {
            // Removal can be transiently vetoed while PnP is busy; keep
            // sweeping and let the closing emptiness check decide the
            // outcome instead of abandoning the remaining nodes and retries.
            if let Err(error) = uninstall_vhci_devnode(&instance_id) {
                last_error = Some(error);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    if enumerate_vhci_interfaces()?.is_empty() && enumerate_vhci_devnodes()?.is_empty() {
        Ok(())
    } else {
        Err(last_error.unwrap_or_else(|| {
            "USBIP-CLEAN-002: residual USB/IP host controller devices could not be removed"
                .to_string()
        }))
    }
}

#[cfg(target_os = "windows")]
fn uninstall_vhci_devnode(instance_id: &str) -> Result<(), String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_LOCATE_DEVNODE_PHANTOM, CM_Locate_DevNodeW, CM_Uninstall_DevNode, CR_SUCCESS,
    };
    use windows::core::PCWSTR;

    let encoded: Vec<u16> = instance_id.encode_utf16().chain(Some(0)).collect();
    let mut devinst = 0u32;
    let located = unsafe {
        CM_Locate_DevNodeW(
            &mut devinst,
            PCWSTR(encoded.as_ptr()),
            CM_LOCATE_DEVNODE_PHANTOM,
        )
    };
    if located != CR_SUCCESS {
        // The devnode can disappear between enumeration and this lookup while
        // asynchronous PnP teardown completes. Treat it as already removed so
        // the sweep continues; the sweep's final emptiness check still fails
        // the cleanup if real leftovers remain.
        return Ok(());
    }
    let uninstalled = unsafe { CM_Uninstall_DevNode(devinst, 0) };
    if uninstalled != CR_SUCCESS {
        return Err(format!(
            "USBIP-CLEAN-002: residual device {instance_id} could not be removed (config error {})",
            uninstalled.0
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_orphaned_driver_service(name: &str, expected_binary: &str) -> Result<(), String> {
    use std::process::Command;
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = format!(r"SYSTEM\CurrentControlSet\Services\{name}");
    let Ok(key) = hklm.open_subkey_with_flags(path, KEY_READ) else {
        return Ok(());
    };
    let image_path = key.get_value::<String, _>("ImagePath").unwrap_or_default();
    if !image_path
        .to_ascii_lowercase()
        .ends_with(&expected_binary.to_ascii_lowercase())
    {
        return Err(format!(
            "USBIP-CLEAN-004: refusing to remove unexpected service {name}"
        ));
    }
    drop(key);
    let output = Command::new("sc.exe")
        .args(["delete", name])
        .output()
        .map_err(|error| format!("USBIP-CLEAN-004: unable to remove service {name}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "USBIP-CLEAN-004: unable to remove orphaned service {name}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_missing_registry_values_are_treated_as_absent() {
        assert_eq!(registry_optional(Ok(7)).unwrap(), Some(7));
        assert_eq!(
            registry_optional::<()>(Err(std::io::ErrorKind::NotFound.into())).unwrap(),
            None
        );
        for kind in [
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::Other,
            std::io::ErrorKind::InvalidData,
        ] {
            assert!(registry_optional::<()>(Err(kind.into())).is_err());
        }
    }

    #[test]
    fn phantom_nodes_are_detected_alongside_a_live_interface() {
        let interfaces =
            vec![r"\\?\ROOT#USB#0001#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}".to_string()];
        assert!(!transport_not_ready(
            &interfaces,
            &[node(r"root\usb\0001", false)]
        ));
        assert!(transport_not_ready(
            &interfaces,
            &[node(r"ROOT\USB\0001", false), node(r"ROOT\USB\0002", false)]
        ));
        assert!(transport_not_ready(&[], &[node(r"ROOT\USB\0002", false)]));
        assert!(!transport_not_ready(&[], &[]));
        assert!(transport_not_ready(
            &[interfaces[0].clone(), interfaces[0].clone()],
            &[]
        ));
    }

    fn node(instance_id: &str, disabled: bool) -> TransportNode {
        TransportNode {
            instance_id: instance_id.to_string(),
            disabled,
        }
    }

    #[test]
    fn a_disabled_previous_root_does_not_block_either_live_name() {
        let alk =
            vec![r"\\?\ROOT#ALKAIDUSB#0000#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}".to_string()];
        assert!(!transport_not_ready(
            &alk,
            &[
                node(r"ROOT\USB\0000", true),
                node(r"ROOT\ALKAIDUSB\0000", false),
            ]
        ));
        let usb = vec![r"\\?\ROOT#USB#0000#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}".to_string()];
        assert!(!transport_not_ready(
            &usb,
            &[
                node(r"ROOT\USB\0000", false),
                node(r"ROOT\ALKAIDUSB\0000", true),
            ]
        ));
    }

    #[test]
    fn a_broken_controller_still_blocks() {
        assert!(transport_not_ready(&[], &[node(r"ROOT\USB\0000", true)]));
        assert!(transport_not_ready(
            &[],
            &[node(r"ROOT\ALKAIDUSB\0000", true)]
        ));
        let live = vec![r"\\?\ROOT#USB#0001#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}".to_string()];
        assert!(transport_not_ready(
            &live,
            &[node(r"ROOT\USB\0001", false), node(r"ROOT\USB\0002", false)]
        ));
        assert!(transport_not_ready(
            &[live[0].clone(), live[0].clone()],
            &[]
        ));
        assert!(!transport_not_ready(&[], &[]));
    }

    #[test]
    fn derives_instance_ids_from_interface_paths() {
        assert_eq!(
            instance_id_from_interface_path(
                r"\\?\ROOT#USB#0001#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}"
            ),
            Some("ROOT\\USB\\0001".to_string())
        );
        assert_eq!(
            instance_id_from_interface_path(
                r"\\?\ROOT#ALKAIDUSB#0000#{b4030c06-dc5f-4fcc-87eb-e5515a0935c0}"
            ),
            Some("ROOT\\ALKAIDUSB\\0000".to_string())
        );
        assert_eq!(instance_id_from_interface_path(r"\\?\ROOT#USB#0001"), None);
        assert_eq!(instance_id_from_interface_path(r"\\?\#{guid}"), None);
        assert_eq!(instance_id_from_interface_path("C:\\plain\\path"), None);
    }

    #[test]
    fn recognizes_only_the_usbip_win2_vhci_hardware_id() {
        assert!(is_vhci_hardware_id(r"ROOT\USBIP_WIN2\UDE"));
        assert!(is_vhci_hardware_id(r"root\usbip_win2\ude"));
        assert!(!is_vhci_hardware_id(r"ROOT\USB\0000"));
        assert!(!is_vhci_hardware_id(r"ROOT\HIDMAESTRO_UDE"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn splits_quoted_and_bare_uninstaller_commands() {
        assert_eq!(
            split_executable_command(r#""C:\Program Files\USBip\unins000.exe""#),
            Some((
                "C:\\Program Files\\USBip\\unins000.exe".to_string(),
                Vec::new()
            ))
        );
        assert_eq!(
            split_executable_command(r#""C:\Program Files\USBip\unins000.exe" /log=x"#),
            Some((
                "C:\\Program Files\\USBip\\unins000.exe".to_string(),
                vec!["/log=x".to_string()]
            ))
        );
        assert_eq!(
            split_executable_command(r"C:\USBip\unins000.exe /silent"),
            Some((
                "C:\\USBip\\unins000.exe".to_string(),
                vec!["/silent".to_string()]
            ))
        );
        assert_eq!(
            split_executable_command(r"C:\USBip\unins000.exe"),
            Some(("C:\\USBip\\unins000.exe".to_string(), Vec::new()))
        );
        assert_eq!(split_executable_command("  "), None);
        // An unclosed quote quotes to the end, as CommandLineToArgvW does.
        assert_eq!(
            split_executable_command(r#""C:\USBip\unins000.exe"#),
            Some(("C:\\USBip\\unins000.exe".to_string(), Vec::new()))
        );
        // Spaces inside a quoted argument are preserved.
        assert_eq!(
            split_executable_command(r#""C:\USBip\unins000.exe" /log="C:\logs dir\x"#),
            Some((
                "C:\\USBip\\unins000.exe".to_string(),
                vec!["/log=C:\\logs dir\\x".to_string()]
            ))
        );
        // An escaped quote stays inside the argument.
        assert_eq!(
            split_executable_command(r#""C:\USBip\unins000.exe" /D="a\"b""#),
            Some((
                "C:\\USBip\\unins000.exe".to_string(),
                vec!["/D=a\"b".to_string()]
            ))
        );
        // Backslashes before a non-quote are literal.
        assert_eq!(
            split_executable_command(r#""C:\a\\b\unins000.exe""#),
            Some(("C:\\a\\\\b\\unins000.exe".to_string(), Vec::new()))
        );
    }
}
