//! Reading and removing usbip-win2 host-controller devnodes (Windows).
//!
//! Removal follows Microsoft's guidance for desktop applications: uninstall a device with `DiUninstallDevice`
//! (`DIF_REMOVE` through the class installers; it stops a running device and reports `NeedReboot` when Windows can only
//! finish at the next restart). The Configuration Manager sequence the `CM_Uninstall_DevNode` page prescribes (is it
//! present? `CM_Query_And_Remove_SubTree`, then `CM_Uninstall_DevNode`) is the fallback; it reports why a removal was
//! vetoed. `CM_Uninstall_DevNode` alone, on a device that is still running, removes its persistent state without stopping
//! it, which is not a removal.

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_LOCATE_DEVNODE_FLAGS, CM_LOCATE_DEVNODE_NORMAL, CM_LOCATE_DEVNODE_PHANTOM, CM_Locate_DevNodeW,
    CM_Query_And_Remove_SubTreeW, CM_REMOVE_NO_RESTART, CM_REMOVE_UI_NOT_OK, CM_Uninstall_DevNode,
    CR_REMOVE_VETOED, CR_SUCCESS, DiUninstallDevice, PNP_VETO_TYPE, SP_DEVINFO_DATA,
    SetupDiCreateDeviceInfoList, SetupDiDestroyDeviceInfoList, SetupDiOpenDeviceInfoW,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::core::{BOOL, PCWSTR, w};

use super::nodes::{
    CR_NO_SUCH_DEVNODE, CONFIGFLAG_DISABLED, NodeFailure, NodeRemoval, TransportNode,
    is_vhci_hardware_id, registry_optional,
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// The device instance handle, or the `CR_*` code that refused.
fn locate(instance_id: &str, flags: CM_LOCATE_DEVNODE_FLAGS) -> Result<u32, u32> {
    let encoded = wide(instance_id);
    let mut devinst = 0u32;
    let result = unsafe { CM_Locate_DevNodeW(&mut devinst, PCWSTR(encoded.as_ptr()), flags) };
    if result == CR_SUCCESS {
        Ok(devinst)
    } else {
        Err(result.0)
    }
}

/// In the device tree now. A registry key under Enum that the tree does not hold is a ghost: only the answer "no such devnode" says
/// so. Any other failure (out of resources, a refused call) does not prove the node is absent, so it counts as present: the
/// removal then runs the documented sequence and reports what Windows answers, instead of skipping the query-and-remove step.
pub(super) fn in_device_tree(instance_id: &str) -> bool {
    locate(instance_id, CM_LOCATE_DEVNODE_NORMAL) != Err(CR_NO_SUCH_DEVNODE)
}

/// Neither in the tree nor as a ghost: nothing of the device is left.
fn is_gone(instance_id: &str) -> bool {
    locate(instance_id, CM_LOCATE_DEVNODE_PHANTOM) == Err(CR_NO_SUCH_DEVNODE)
}

/// Host-controller nodes under one root enumerator, with whether each is in the device tree.
pub(super) fn read_transport_nodes(enumerator: &str) -> Result<Vec<TransportNode>, String> {
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
            let instance_id = format!(r"{enumerator}\{name}");
            nodes.push(TransportNode {
                present: in_device_tree(&instance_id),
                instance_id,
                disabled: (flags & CONFIGFLAG_DISABLED) != 0,
            });
        }
    }
    Ok(nodes)
}

/// The Win32 error inside an `HRESULT_FROM_WIN32` value (SetupAPI and `newdev` report errors that way).
fn win32_code(error: &windows::core::Error) -> u32 {
    let code = error.code().0 as u32;
    if code >> 16 == 0x8007 { code & 0xFFFF } else { code }
}

struct DeviceInfoSet(windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

/// `DiUninstallDevice` on the node. `Ok(true)` when Windows needs a restart to finish.
fn setupapi_uninstall(instance_id: &str) -> Result<bool, NodeFailure> {
    let set = unsafe { SetupDiCreateDeviceInfoList(None, None) }.map_err(|error| NodeFailure {
        step: "SetupDiCreateDeviceInfoList",
        win32: Some(win32_code(&error)),
        ..Default::default()
    })?;
    let set = DeviceInfoSet(set);
    let encoded = wide(instance_id);
    let mut data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    unsafe { SetupDiOpenDeviceInfoW(set.0, PCWSTR(encoded.as_ptr()), None, 0, Some(&mut data)) }
        .map_err(|error| NodeFailure {
            step: "SetupDiOpenDeviceInfo",
            win32: Some(win32_code(&error)),
            ..Default::default()
        })?;
    let mut need_reboot = BOOL(0);
    unsafe { DiUninstallDevice(HWND::default(), set.0, &data, 0, Some(&mut need_reboot)) }.map_err(
        |error| NodeFailure {
            step: "DiUninstallDevice",
            win32: Some(win32_code(&error)),
            ..Default::default()
        },
    )?;
    Ok(need_reboot.as_bool())
}

/// `CM_Query_And_Remove_SubTree` requires `SeLoadDriverPrivilege`. An elevated administrator token holds it switched off,
/// so it is switched on before the Configuration Manager calls. Best effort: the call that follows reports a refusal.
fn enable_load_driver_privilege() {
    use windows::Win32::Foundation::LUID;
    use windows::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .is_err()
        {
            return;
        }
        let mut luid = LUID::default();
        if LookupPrivilegeValueW(PCWSTR::null(), w!("SeLoadDriverPrivilege"), &mut luid).is_ok() {
            let privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            let _ = AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None);
        }
        let _ = CloseHandle(token);
    }
}

/// The sequence the `CM_Uninstall_DevNode` documentation gives: present? query-and-remove, then uninstall.
fn configuration_manager_remove(instance_id: &str) -> Result<(), NodeFailure> {
    enable_load_driver_privilege();
    let devinst = match locate(instance_id, CM_LOCATE_DEVNODE_PHANTOM) {
        Ok(devinst) => devinst,
        Err(CR_NO_SUCH_DEVNODE) => return Ok(()),
        Err(code) => {
            return Err(NodeFailure {
                step: "CM_Locate_DevNode",
                cr: Some(code),
                ..Default::default()
            });
        }
    };
    if in_device_tree(instance_id) {
        let mut veto = PNP_VETO_TYPE::default();
        let mut name = [0u16; 260];
        let result = unsafe {
            CM_Query_And_Remove_SubTreeW(
                devinst,
                Some(&mut veto),
                Some(&mut name),
                CM_REMOVE_UI_NOT_OK | CM_REMOVE_NO_RESTART,
            )
        };
        if result != CR_SUCCESS {
            let length = name.iter().position(|unit| *unit == 0).unwrap_or(name.len());
            return Err(NodeFailure {
                step: "CM_Query_And_Remove_SubTree",
                cr: Some(result.0),
                veto: (result == CR_REMOVE_VETOED)
                    .then(|| (veto.0, String::from_utf16_lossy(&name[..length]))),
                ..Default::default()
            });
        }
    }
    let result = unsafe { CM_Uninstall_DevNode(devinst, 0) };
    if result == CR_SUCCESS {
        Ok(())
    } else {
        Err(NodeFailure {
            step: "CM_Uninstall_DevNode",
            cr: Some(result.0),
            ..Default::default()
        })
    }
}

/// Removes one devnode: `DiUninstallDevice` first, the Configuration Manager sequence when it fails, and a check that
/// nothing of the node is left (a node Windows keeps until the restart is `RebootRequired`).
pub(super) fn remove_devnode(instance_id: &str) -> NodeRemoval {
    if is_gone(instance_id) {
        return NodeRemoval::Removed;
    }
    let mut failures = Vec::new();
    let mut need_reboot = false;
    match setupapi_uninstall(instance_id) {
        Ok(reboot) => need_reboot = reboot,
        Err(failure) => {
            failures.push(failure);
            match configuration_manager_remove(instance_id) {
                Ok(()) => {}
                Err(second) => {
                    failures.push(second);
                    // a refusal of both steps can still be a node that is gone by now (asynchronous teardown)
                    return if is_gone(instance_id) {
                        NodeRemoval::Removed
                    } else {
                        NodeRemoval::Failed(failures)
                    };
                }
            }
        }
    }
    // Teardown is asynchronous: a node Windows did not flag for a restart gets a few seconds to disappear.
    let polls = if need_reboot { 1 } else { 10 };
    for _ in 0..polls {
        if is_gone(instance_id) {
            return NodeRemoval::Removed;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    if is_gone(instance_id) {
        NodeRemoval::Removed
    } else {
        NodeRemoval::RebootRequired
    }
}
