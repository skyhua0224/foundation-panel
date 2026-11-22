//! Residual VHCI device detection and cleanup.
//!
//! An older usbip-win2 (0.9.7.x) uninstaller, an interrupted uninstall or an upgrade can leave host-controller devnodes
//! behind. This module enumerates the live interfaces the way `usbip.exe` does, and removes the leftovers. The node model
//! and the readiness rule are in `nodes.rs`; reading and removing a devnode is `devnode.rs`.

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

#[cfg_attr(not(target_os = "windows"), allow(unused_imports))]
pub(super) use super::nodes::{Residual, TransportNode, residual_kind, transport_not_ready};

/// Host-controller nodes under either enumerator, each with whether it is in the device tree. The readiness decision
/// needs both enumerators: the live controller can be at ROOT\ALKAIDUSB.
#[cfg(target_os = "windows")]
pub(super) fn enumerate_transport_nodes() -> Result<Vec<TransportNode>, String> {
    let mut nodes = super::devnode::read_transport_nodes(r"ROOT\USB")?;
    nodes.extend(super::devnode::read_transport_nodes(r"ROOT\ALKAIDUSB")?);
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

/// How a cleanup ended: done, or accepted by Windows and finished at the next restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Finish {
    Done,
    RestartNeeded,
}

/// Runs inside the elevated helper. Order matters: the vendor uninstaller is
/// the only supported way to retire the driver packages, service registration,
/// and scheduled task; the devnode sweep afterwards catches the nodes the vendor
/// uninstaller leaves behind (0.9.7.x does, and the host's helper reports success
/// without touching nodes when no installation is registered any more). An
/// uninstaller failure never skips the sweep on the paths that run the uninstaller
/// here — leftover devnodes are the hard blocker for the next install — but it is
/// still reported so the person can retry.
#[cfg(target_os = "windows")]
pub(super) async fn cleanup_broken_transport() -> Result<Finish, String> {
    if super::manager::externally_managed() {
        return Err("USBIP-CLEAN-005: externally managed USB/IP is preserved; use Windows Installed apps if you choose to replace it".into());
    }
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
        // It answers success without touching devnodes when nothing is registered any more, which is exactly the state
        // an old uninstaller leaves; the helper has finished, so the sweep no longer competes with it.
        return sweep_devnodes(false).await;
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
        return match uninstall_error {
            Some(error) => Err(error),
            None => sweep_devnodes(false).await,
        };
    }
    let finish = sweep_devnodes(false).await?;
    remove_orphaned_driver_service("usbip2_ude", "usbip2_ude.sys")?;
    remove_orphaned_driver_service("usbip2_filter", "usbip2_filter.sys")?;
    uninstall_error.map_or(Ok(finish), Err)
}

/// Removes the leftover controllers and keeps the live one: the repair for a working transport that an older version
/// left controllers behind. Runs inside the elevated helper.
#[cfg(target_os = "windows")]
pub(super) async fn remove_stale_nodes() -> Result<Finish, String> {
    // No stream check: one live controller stays, so nothing a stream uses is touched. Only a transport with a live controller
    // is repaired this way; with none, nothing says which node to keep, and the sweep says so.
    sweep_devnodes(true).await
}

/// After an install or a replacement: the controllers an older version left beside the new live one are removed, so the
/// person never meets "installed but not ready". Best effort and never an error: a leftover that stays is reported by the
/// status with its own repair, and the install itself has succeeded.
#[cfg(target_os = "windows")]
pub(super) async fn sweep_stale_after_install() -> Finish {
    match enumerate_vhci_interfaces() {
        Ok(interfaces) if !interfaces.is_empty() => {
            sweep_devnodes(true).await.unwrap_or(Finish::Done)
        }
        _ => Finish::Done,
    }
}

/// Removes devnodes until none is left that should go. PnP teardown is asynchronous, so a node that is still listed is
/// asked again for the whole polling budget; a node Windows accepted but keeps until the restart is set aside.
/// `keep_live` keeps one live controller and removes the others; which one is chosen once, here, and every round leaves it alone.
#[cfg(target_os = "windows")]
pub(super) async fn sweep_devnodes(keep_live: bool) -> Result<Finish, String> {
    use super::nodes::{
        CleanupReason, NodeFailure, NodeRemoval, PENDING_DETAIL, classify_failures, cleanup_error,
        describe_failure, pick_live, removal_targets,
    };

    let kept: Option<String> = if keep_live {
        match pick_live(&enumerate_vhci_interfaces()?) {
            Some(id) => Some(id),
            None => {
                return Err("USBIP-CLEAN-002: there is no live USB/IP controller to keep; clean the shared transport instead".to_string());
            }
        }
    } else {
        None
    };
    let mut pending: Vec<String> = Vec::new();
    let mut failed: Vec<(String, Vec<NodeFailure>)> = Vec::new();
    for _ in 0..CLEANUP_POLL_ATTEMPTS {
        let interfaces = enumerate_vhci_interfaces()?;
        let nodes = enumerate_transport_nodes()?;
        let mut targets = removal_targets(&interfaces, &nodes, kept.as_deref());
        targets.retain(|id| !pending.iter().any(|set_aside| set_aside.eq_ignore_ascii_case(id)));
        failed.clear();
        if targets.is_empty() {
            break;
        }
        for instance_id in targets {
            // Removal can be transiently vetoed while PnP is busy; keep
            // sweeping and let the closing check decide the outcome instead
            // of abandoning the remaining nodes and retries.
            let owned = instance_id.clone();
            let outcome = tokio::task::spawn_blocking(move || super::devnode::remove_devnode(&owned))
                .await
                .map_err(|error| format!("USBIP-CLEAN-002: the removal task failed: {error}"))?;
            match outcome {
                NodeRemoval::Removed => {}
                NodeRemoval::RebootRequired => pending.push(instance_id),
                NodeRemoval::Failed(steps) => failed.push((instance_id, steps)),
            }
        }
        if !failed.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
    let interfaces = enumerate_vhci_interfaces()?;
    let nodes = enumerate_transport_nodes()?;
    let mut remaining = removal_targets(&interfaces, &nodes, kept.as_deref());
    remaining.retain(|id| !pending.iter().any(|set_aside| set_aside.eq_ignore_ascii_case(id)));
    // A node that failed in the last round and is gone by now is not a failure.
    failed.retain(|(id, _)| remaining.iter().any(|left| left.eq_ignore_ascii_case(id)));
    if remaining.is_empty() {
        return Ok(if pending.is_empty() {
            Finish::Done
        } else {
            Finish::RestartNeeded
        });
    }
    // Something is still there. What Windows answered says what the person does next; the nodes that were accepted for the
    // restart are named in the same error, so the caller keeps the restart marker for them as well.
    let steps: Vec<NodeFailure> = failed
        .iter()
        .flat_map(|(_, steps)| steps.iter().cloned())
        .collect();
    let mut details: Vec<String> = failed
        .iter()
        .flat_map(|(id, steps)| steps.iter().map(|step| describe_failure(id, step)))
        .collect();
    details.extend(pending.iter().map(|id| format!("node={id} {PENDING_DETAIL}")));
    details.extend(remaining.iter().map(|id| format!("node={id} still listed")));
    let reason = if steps.is_empty() {
        CleanupReason::Unknown
    } else {
        classify_failures(&steps)
    };
    Err(cleanup_error(reason, &details))
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
    #[cfg(target_os = "windows")]
    use super::*;

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
