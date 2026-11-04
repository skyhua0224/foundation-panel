//! Direct Windows queries for the driver and process probes of the panel. They replace PowerShell, a batch script, `sunshine.exe --version` and WMI,
//! which took 0.05 to 1.9 s each on the Skyindows VM: the registry keys of the device nodes, the Configuration Manager for their state and children,
//! the process snapshot for the Sunshine processes, and the version resource of the executable.
//!
//! - Device nodes of a root-enumerated device: HKLM\SYSTEM\CurrentControlSet\Enum\ROOT\<enumerator>\<instance>
//!   https://learn.microsoft.com/windows-hardware/drivers/install/hklm-system-currentcontrolset-enum-registry-key
//! - Driver version of the node: HKLM\SYSTEM\CurrentControlSet\Control\Class\{class}\<index>\DriverVersion
//!   https://learn.microsoft.com/windows-hardware/drivers/install/hklm-system-currentcontrolset-control-class-registry-key
//! - CM_Locate_DevNodeW, CM_Get_DevNode_Status, CM_Get_Child, CM_Get_Sibling, CM_Get_Device_IDW
//!   https://learn.microsoft.com/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_devnode_status
//! - NtQuerySystemInformation(SystemProcessInformation): the creation time, working set and commit of every process in one call, readable for the
//!   processes of other accounts too (OpenProcess is refused for the Sunshine service, which runs as SYSTEM, even with PROCESS_QUERY_LIMITED_INFORMATION):
//!   https://learn.microsoft.com/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation
//! - GetFileVersionInfoW / VerQueryValueW: https://learn.microsoft.com/windows/win32/api/winver/nf-winver-getfileversioninfow

/// What the Configuration Manager says about a device node, as the driver script of the host reads it: started without a problem is `Ok`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeState {
    Missing,
    Ok,
    Error(u32),
}

/// One device node under HKLM\SYSTEM\CurrentControlSet\Enum\ROOT\<enumerator>.
#[derive(Clone, Debug, Default)]
pub struct EnumDevice {
    pub instance_id: String,
    pub hardware_ids: Vec<String>,
    pub friendly_name: String,
    pub device_desc: String,
    pub driver_key: String,
}

/// A child device node of a device (its service and its state).
#[derive(Clone, Debug)]
pub struct ChildNode {
    pub service: String,
    pub state: NodeState,
}

/// A running process, as `NtQuerySystemInformation(SystemProcessInformation)` lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemProcess {
    pub pid: u32,
    pub name: String,
    /// creation time, FILETIME ticks (100 ns since 1601)
    pub created: u64,
    pub peak_working_set: u64,
    pub working_set: u64,
    /// commit charge (PagefileUsage)
    pub pagefile: u64,
    /// private commit (PrivatePageCount)
    pub private_bytes: u64,
}

/// The entries of a SystemProcessInformation buffer (the 64-bit layout of SYSTEM_PROCESS_INFORMATION). Stops at the first entry that does not fit.
pub fn parse_system_processes(buffer: &[u8]) -> Vec<SystemProcess> {
    const MIN_ENTRY: usize = 208;
    let u32_at = |at: usize| u32::from_le_bytes(buffer[at..at + 4].try_into().unwrap());
    let u64_at = |at: usize| u64::from_le_bytes(buffer[at..at + 8].try_into().unwrap());
    let mut entries = Vec::new();
    let mut offset = 0usize;
    while offset + MIN_ENTRY <= buffer.len() {
        let name_len = u16::from_le_bytes([buffer[offset + 56], buffer[offset + 57]]) as usize;
        let name_ptr = u64_at(offset + 64) as usize;
        // the name lives in the same buffer, after the entries; read it by its address relative to the buffer start
        let base = buffer.as_ptr() as usize;
        let name = if name_ptr >= base && name_ptr + name_len <= base + buffer.len() && name_len % 2 == 0 {
            let at = name_ptr - base;
            let units: Vec<u16> = buffer[at..at + name_len].chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
            String::from_utf16_lossy(&units)
        } else {
            String::new()
        };
        entries.push(SystemProcess {
            pid: u64_at(offset + 80) as u32,
            name,
            created: u64_at(offset + 32),
            peak_working_set: u64_at(offset + 136),
            working_set: u64_at(offset + 144),
            pagefile: u64_at(offset + 184),
            private_bytes: u64_at(offset + 200),
        });
        let next = u32_at(offset) as usize;
        if next == 0 {
            break;
        }
        offset += next;
    }
    entries
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_DEVNODE_STATUS_FLAGS, CM_Get_Child, CM_Get_DevNode_Status, CM_Get_Device_IDW,
        CM_Get_Sibling, CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CM_PROB, CR_INVALID_DEVNODE,
        CR_NO_SUCH_DEVNODE, CR_SUCCESS, DN_STARTED,
    };
    use windows::core::PCWSTR;
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};

    fn hklm() -> RegKey {
        RegKey::predef(HKEY_LOCAL_MACHINE)
    }

    fn open(path: &str) -> Option<RegKey> {
        hklm()
            .open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
            .ok()
    }

    fn multi_or_single(key: &RegKey, name: &str) -> Vec<String> {
        key.get_value::<Vec<String>, _>(name)
            .or_else(|_| key.get_value::<String, _>(name).map(|one| vec![one]))
            .unwrap_or_default()
    }

    /// Every device node of `HKLM\SYSTEM\CurrentControlSet\Enum\ROOT\<enumerator>`, present or not (like the registry walk of the host's driver script).
    pub fn enum_root_devices(enumerator: &str) -> Result<Vec<EnumDevice>, String> {
        let base = format!(r"SYSTEM\CurrentControlSet\Enum\ROOT\{enumerator}");
        let Some(enum_key) = open(&base) else {
            return Ok(Vec::new());
        };
        let mut devices = Vec::new();
        for name in enum_key.enum_keys().filter_map(Result::ok) {
            let Ok(node) = enum_key.open_subkey_with_flags(&name, KEY_READ | KEY_WOW64_64KEY)
            else {
                continue;
            };
            devices.push(EnumDevice {
                instance_id: format!(r"ROOT\{enumerator}\{name}"),
                hardware_ids: multi_or_single(&node, "HardwareID"),
                friendly_name: node.get_value::<String, _>("FriendlyName").unwrap_or_default(),
                device_desc: node.get_value::<String, _>("DeviceDesc").unwrap_or_default(),
                driver_key: node.get_value::<String, _>("Driver").unwrap_or_default(),
            });
        }
        Ok(devices)
    }

    /// (DriverVersion, InfPath) of a node's driver key.
    pub fn driver_values(driver_key: &str) -> (String, String) {
        if driver_key.is_empty() {
            return (String::new(), String::new());
        }
        match open(&format!(r"SYSTEM\CurrentControlSet\Control\Class\{driver_key}")) {
            Some(key) => (
                key.get_value::<String, _>("DriverVersion").unwrap_or_default(),
                key.get_value::<String, _>("InfPath").unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        }
    }

    pub fn windows_build_number() -> Option<u32> {
        let key = open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")?;
        key.get_value::<String, _>("CurrentBuildNumber")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| key.get_value::<String, _>("CurrentBuild").ok())
            .and_then(|value| value.trim().parse().ok())
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn locate(instance_id: &str) -> Result<u32, windows::Win32::Devices::DeviceAndDriverInstallation::CONFIGRET> {
        let id = wide(instance_id);
        let mut devinst = 0u32;
        let result = unsafe {
            CM_Locate_DevNodeW(&mut devinst, PCWSTR(id.as_ptr()), CM_LOCATE_DEVNODE_NORMAL)
        };
        if result == CR_SUCCESS { Ok(devinst) } else { Err(result) }
    }

    fn state_of(devinst: u32) -> NodeState {
        let mut status = CM_DEVNODE_STATUS_FLAGS(0);
        let mut problem = CM_PROB(0);
        let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, devinst, 0) };
        if result == CR_INVALID_DEVNODE || result == CR_NO_SUCH_DEVNODE {
            NodeState::Missing
        } else if result == CR_SUCCESS && problem.0 == 0 && status.0 & DN_STARTED.0 != 0 {
            NodeState::Ok
        } else {
            NodeState::Error(problem.0)
        }
    }

    /// The state of a device node, as `Get-VddDeviceStatus` of the host's script reads it.
    pub fn node_state(instance_id: &str) -> NodeState {
        match locate(instance_id) {
            Ok(devinst) => state_of(devinst),
            Err(result) if result == CR_INVALID_DEVNODE || result == CR_NO_SUCH_DEVNODE => NodeState::Missing,
            Err(_) => NodeState::Error(0),
        }
    }

    fn device_id(devinst: u32) -> Option<String> {
        let mut buffer = [0u16; 512];
        let result = unsafe { CM_Get_Device_IDW(devinst, &mut buffer, 0) };
        (result == CR_SUCCESS).then(|| {
            let len = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
            String::from_utf16_lossy(&buffer[..len])
        })
    }

    /// The child device nodes of a present device node.
    pub fn child_nodes(instance_id: &str) -> Result<Vec<ChildNode>, String> {
        let parent = locate(instance_id).map_err(|result| format!("CM_Locate_DevNodeW {}", result.0))?;
        let mut children = Vec::new();
        let mut child = 0u32;
        if unsafe { CM_Get_Child(&mut child, parent, 0) } != CR_SUCCESS {
            return Ok(children);
        }
        loop {
            if let Some(id) = device_id(child) {
                let service = open(&format!(r"SYSTEM\CurrentControlSet\Enum\{id}"))
                    .and_then(|key| key.get_value::<String, _>("Service").ok())
                    .unwrap_or_default();
                children.push(ChildNode { service, state: state_of(child) });
            }
            let mut next = 0u32;
            if unsafe { CM_Get_Sibling(&mut next, child, 0) } != CR_SUCCESS {
                break;
            }
            child = next;
        }
        Ok(children)
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQuerySystemInformation(class: u32, info: *mut core::ffi::c_void, length: u32, returned: *mut u32) -> i32;
    }

    /// Every process of the system with its creation time and memory counters (one call, 2 to 5 ms). 64-bit layout only.
    #[cfg(target_pointer_width = "64")]
    pub fn system_processes() -> Result<Vec<SystemProcess>, String> {
        const SYSTEM_PROCESS_INFORMATION: u32 = 5;
        const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004u32 as i32;
        let mut buffer = vec![0u8; 1 << 20];
        for _ in 0..4 {
            let mut returned = 0u32;
            let status = unsafe {
                NtQuerySystemInformation(SYSTEM_PROCESS_INFORMATION, buffer.as_mut_ptr().cast(), buffer.len() as u32, &mut returned)
            };
            if status == STATUS_INFO_LENGTH_MISMATCH {
                buffer = vec![0u8; (returned as usize).max(buffer.len()) + (64 << 10)];
                continue;
            }
            if status < 0 {
                return Err(format!("NtQuerySystemInformation: {status:#x}"));
            }
            buffer.truncate(returned as usize);
            return Ok(parse_system_processes(&buffer));
        }
        Err("NtQuerySystemInformation: the process list kept growing".to_string())
    }

    #[cfg(not(target_pointer_width = "64"))]
    pub fn system_processes() -> Result<Vec<SystemProcess>, String> {
        Err("the 64-bit process list layout is needed".to_string())
    }

    /// The processes whose image name contains `needle` (case-insensitive).
    pub fn processes_named(needle: &str) -> Result<Vec<SystemProcess>, String> {
        let needle = needle.to_lowercase();
        Ok(system_processes()?.into_iter().filter(|p| p.name.to_lowercase().contains(&needle)).collect())
    }

    /// The FileVersion string of an executable (VS_VERSION_INFO StringFileInfo), if it has one.
    pub fn file_version_string(path: &std::path::Path) -> Option<String> {
        file_version_strings(path, &["FileVersion"])?.remove("FileVersion")
    }

    /// Several String values of the first language/code page of the VERSIONINFO, read with one
    /// GetFileVersionInfoW (the predefined names and custom keys such as the host's `Commit`).
    /// Keys the resource lacks are absent from the map; `None` when the file has no version resource.
    pub fn file_version_strings(path: &std::path::Path, keys: &[&str]) -> Option<std::collections::HashMap<String, String>> {
        use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
        let file = wide(&path.to_string_lossy());
        unsafe {
            let size = GetFileVersionInfoSizeW(PCWSTR(file.as_ptr()), None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(PCWSTR(file.as_ptr()), Some(0), size, data.as_mut_ptr().cast()).ok()?;
            // the first language/code page pair of the resource
            let mut translation: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut len = 0u32;
            let key = wide(r"\VarFileInfo\Translation");
            if !VerQueryValueW(data.as_ptr().cast(), PCWSTR(key.as_ptr()), &mut translation, &mut len).as_bool() || len < 4 {
                return None;
            }
            let pair = std::slice::from_raw_parts(translation as *const u16, 2);
            let mut found = std::collections::HashMap::new();
            for name in keys {
                let sub_block = wide(&format!(r"\StringFileInfo\{:04x}{:04x}\{name}", pair[0], pair[1]));
                let mut value: *mut core::ffi::c_void = std::ptr::null_mut();
                let mut value_len = 0u32;
                if !VerQueryValueW(data.as_ptr().cast(), PCWSTR(sub_block.as_ptr()), &mut value, &mut value_len).as_bool() {
                    continue;
                }
                let units = std::slice::from_raw_parts(value as *const u16, value_len as usize);
                let end = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
                found.insert(name.to_string(), String::from_utf16_lossy(&units[..end]).trim().to_string());
            }
            Some(found)
        }
    }

    /// (total physical bytes, available physical bytes).
    pub fn physical_memory() -> Result<(u64, u64), String> {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut status).map_err(|e| e.to_string())?;
            Ok((status.ullTotalPhys, status.ullAvailPhys))
        }
    }
}

#[cfg(target_os = "windows")]
pub use imp::*;

/// FILETIME ticks (100 ns since 1601) as Unix milliseconds, truncated to the whole second.
pub fn unix_ms_from_filetime(ticks: u64) -> Option<u64> {
    const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;
    let since_epoch = ticks.checked_sub(UNIX_EPOCH_TICKS)?;
    Some(since_epoch / 10_000_000 * 1000)
}

/// The FileVersion of the host program when it is the plain `year.mmdd.build` form its `--version` prints (any other form is read by running it).
pub fn plain_host_version(file_version: &str) -> Option<String> {
    let parts: Vec<&str> = file_version.trim().split('.').collect();
    (parts.len() == 3 && parts.iter().all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
        .then(|| parts.join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_becomes_whole_unix_seconds() {
        // 2026-11-03 04:11:37 UTC and a fraction
        let seconds_since_1601 = 11_644_473_600u64 + 1_793_693_497;
        let ticks = seconds_since_1601 * 10_000_000 + 1_234_567;
        assert_eq!(unix_ms_from_filetime(ticks), Some(1_793_693_497_000));
        assert_eq!(unix_ms_from_filetime(5), None);
    }

    fn entry(next: u32, pid: u64, name_ptr: u64, name_len: u16, ws: u64) -> Vec<u8> {
        let mut e = vec![0u8; 208];
        e[0..4].copy_from_slice(&next.to_le_bytes());
        e[32..40].copy_from_slice(&1234u64.to_le_bytes());
        e[56..58].copy_from_slice(&name_len.to_le_bytes());
        e[64..72].copy_from_slice(&name_ptr.to_le_bytes());
        e[80..88].copy_from_slice(&pid.to_le_bytes());
        e[136..144].copy_from_slice(&(ws + 1).to_le_bytes());
        e[144..152].copy_from_slice(&ws.to_le_bytes());
        e[184..192].copy_from_slice(&7u64.to_le_bytes());
        e[200..208].copy_from_slice(&9u64.to_le_bytes());
        e
    }

    #[test]
    fn the_process_list_is_read_entry_by_entry_with_names_from_the_same_buffer() {
        let name: Vec<u8> = "Sunshine.exe".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let mut buffer = vec![0u8; 208 * 2 + name.len()];
        let base = buffer.as_ptr() as usize as u64;
        let name_at = base + 208 * 2;
        buffer[..208].copy_from_slice(&entry(208, 4, 0, 0, 100));
        buffer[208..416].copy_from_slice(&entry(0, 9952, name_at, name.len() as u16, 7_426_048));
        buffer[416..].copy_from_slice(&name);
        let list = parse_system_processes(&buffer);
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].pid, list[0].name.as_str()), (4, ""));
        assert_eq!((list[1].pid, list[1].name.as_str(), list[1].working_set, list[1].peak_working_set), (9952, "Sunshine.exe", 7_426_048, 7_426_049));
        assert_eq!((list[1].created, list[1].pagefile, list[1].private_bytes), (1234, 7, 9));
    }

    #[test]
    fn a_truncated_buffer_gives_what_it_holds() {
        assert!(parse_system_processes(&[0u8; 100]).is_empty());
    }

    #[test]
    fn only_the_plain_three_part_version_is_taken_from_the_resource() {
        assert_eq!(plain_host_version("2026.1103.155614"), Some("2026.1103.155614".to_string()));
        assert_eq!(plain_host_version(" 2026.1103.155614 "), Some("2026.1103.155614".to_string()));
        assert_eq!(plain_host_version("2026.1103.155614.0"), None);
        assert_eq!(plain_host_version("2026.1103.1.杂鱼"), None);
        assert_eq!(plain_host_version(""), None);
    }
}
