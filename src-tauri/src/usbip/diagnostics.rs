//! One-click export of what is needed to diagnose a USB/IP transport that will not install, clean or become ready: the
//! status the panel computed, the host-controller device tree as Windows keeps it, the driver services, the driver
//! packages, the registered installations, the last cleanup's error and the Windows version. Everything is read without
//! administrator rights; nothing is changed.

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

#[cfg(target_os = "windows")]
use std::fmt::Write as _;

/// The text the file starts with: what it is for and what it does not contain.
pub(super) const HEADER: &str = "USB/IP diagnostics\nThis file lists the USB/IP host-controller devices, driver services, driver packages and the Windows version of this PC. It contains no stream content and no account data.\n";

/// Lines of `pnputil /enum-drivers` that belong to a usbip driver package: its output is blocks separated by empty lines.
pub(super) fn usbip_driver_blocks(output: &str) -> String {
    let text = output.replace("\r\n", "\n");
    text.split("\n\n")
        .filter(|block| block.to_ascii_lowercase().contains("usbip"))
        .map(|block| block.trim())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The lines of the device-installation log (`setupapi.dev.log`) that mention the USB/IP drivers or their root nodes
/// plus error/exit status lines within those matching sections: the last `max`.
/// `DiUninstallDevice` documents that log as where the detail of a removal is.
pub(super) fn setupapi_usbip_lines(text: &str, max: usize) -> Vec<String> {
    let mut in_usbip_section = false;
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        let trimmed = raw.trim_end();
        let lower = trimmed.to_ascii_lowercase();
        let mentions_usbip = lower.contains("usbip")
            || lower.contains(r"root\usb\")
            || lower.contains(r"root\alkaidusb\");
        if trimmed.trim_start().starts_with(">>>") {
            in_usbip_section = mentions_usbip;
        }
        let is_diag_outcome = in_usbip_section
            && (trimmed.trim_start().starts_with("!!!")
                || trimmed.trim_start().starts_with("<<<")
                || lower.contains("error ")
                || lower.contains("exit status")
                || lower.contains("0x")
                || lower.contains("cm_prob"));
        if mentions_usbip || is_diag_outcome {
            lines.push(trimmed.to_string());
        }
        if trimmed.trim_start().starts_with("<<<") {
            in_usbip_section = false;
        }
    }
    if lines.len() > max {
        lines.drain(..lines.len() - max);
    }
    lines
}

/// The file name the export gets: sortable and free of characters Windows refuses.
pub(super) fn file_name(timestamp: &str) -> String {
    let safe: String = timestamp
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect();
    format!("usbip-diagnostics-{safe}.txt")
}

/// Runs a system tool and returns what it printed, or why it did not. A tool that does not finish in twenty seconds is stopped
/// and waited for, so repeated exports cannot pile up processes or threads.
#[cfg(target_os = "windows")]
fn run_capture(program: &str, arguments: &[&str]) -> Result<String, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let root = std::env::var_os("SystemRoot").ok_or("Windows directory unavailable")?;
    let path = std::path::PathBuf::from(root).join("System32").join(program);
    let mut child = Command::new(path)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
        .spawn()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    let mut stdout = child.stdout.take().ok_or("no output pipe")?;
    // The pipe is read on its own thread so a full pipe cannot stall the tool while this side waits for it to exit.
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Err(error) => return Err(format!("{program} failed: {error}")),
        }
    };
    let bytes = reader.join().unwrap_or_default();
    match status {
        None => Err(format!("{program} did not finish in 20 seconds and was stopped")),
        Some(status) if !status.success() => Err(format!("{program} exited with {status}")),
        Some(_) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
    }
}

#[cfg(target_os = "windows")]
fn registry_string(root: &winreg::RegKey, path: &str, name: &str) -> String {
    root.open_subkey(path)
        .ok()
        .and_then(|key| key.get_value::<String, _>(name).ok())
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn registry_u32(root: &winreg::RegKey, path: &str, name: &str) -> Option<u32> {
    root.open_subkey(path)
        .ok()
        .and_then(|key| key.get_value::<u32, _>(name).ok())
}

#[cfg(target_os = "windows")]
fn windows_section(out: &mut String) {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let version = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let _ = writeln!(out, "== Windows");
    for name in [
        "ProductName",
        "DisplayVersion",
        "ReleaseId",
        "CurrentBuild",
        "BuildLabEx",
        "EditionID",
    ] {
        let _ = writeln!(out, "{name}: {}", registry_string(&hklm, version, name));
    }
    let _ = writeln!(
        out,
        "UBR: {}",
        registry_u32(&hklm, version, "UBR").map_or(String::new(), |value| value.to_string())
    );
    let _ = writeln!(
        out,
        "FastStartup (HiberbootEnabled): {}",
        registry_u32(
            &hklm,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Power",
            "HiberbootEnabled"
        )
        .map_or("unknown".to_string(), |value| value.to_string())
    );
    let uptime = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() } / 1000;
    let _ = writeln!(out, "Uptime: {uptime} s");
    let _ = writeln!(out, "Panel elevated: {}", crate::elevation::is_elevated());
    let _ = writeln!(out, "Panel version: {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(out, "Pinned USB/IP: {}", super::USBIP_VERSION);
}

#[cfg(target_os = "windows")]
fn devices_section(out: &mut String) {
    use super::nodes::instance_id_from_interface_path;

    let _ = writeln!(out, "\n== Host-controller devices (hardware ID ROOT\\USBIP_WIN2\\UDE)");
    match super::device::enumerate_vhci_interfaces() {
        Ok(interfaces) => {
            let _ = writeln!(out, "Present VHCI interfaces: {}", interfaces.len());
            for path in &interfaces {
                let _ = writeln!(
                    out,
                    "  {path} -> {}",
                    instance_id_from_interface_path(path).unwrap_or_default()
                );
            }
        }
        Err(error) => {
            let _ = writeln!(out, "Present VHCI interfaces: unreadable ({error})");
        }
    }
    match super::device::enumerate_transport_nodes() {
        Ok(nodes) => {
            let _ = writeln!(out, "Nodes under Enum: {}", nodes.len());
            for node in &nodes {
                let _ = writeln!(
                    out,
                    "  {} in_device_tree={} disabled={}",
                    node.instance_id, node.present, node.disabled
                );
            }
        }
        Err(error) => {
            let _ = writeln!(out, "Nodes under Enum: unreadable ({error})");
        }
    }
}

#[cfg(target_os = "windows")]
fn services_section(out: &mut String) {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let _ = writeln!(out, "\n== Driver services");
    for name in ["usbip2_ude", "usbip2_filter", "usbip2_vhci"] {
        let path = format!(r"SYSTEM\CurrentControlSet\Services\{name}");
        if hklm.open_subkey(&path).is_err() {
            let _ = writeln!(out, "{name}: not registered");
            continue;
        }
        let _ = writeln!(
            out,
            "{name}: Start={} Type={} ImagePath={}",
            registry_u32(&hklm, &path, "Start").map_or("?".to_string(), |value| value.to_string()),
            registry_u32(&hklm, &path, "Type").map_or("?".to_string(), |value| value.to_string()),
            registry_string(&hklm, &path, "ImagePath"),
        );
        let _ = writeln!(
            out,
            "{}",
            run_capture("sc.exe", &["query", name]).map_or_else(|error| format!("({error})"), |text| text.trim().to_string())
        );
    }
}

#[cfg(target_os = "windows")]
fn installations_section(out: &mut String) {
    let _ = writeln!(out, "\n== Registered installations");
    match super::usbip_uninstall_entries() {
        Ok(entries) if entries.is_empty() => {
            let _ = writeln!(out, "none");
        }
        Ok(entries) => {
            for entry in entries {
                let value = |name: &str| entry.key.get_value::<String, _>(name).unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{} | DisplayVersion={} | InstallLocation={} | slim={} | UninstallString={}",
                    entry.display_name,
                    value("DisplayVersion"),
                    value("InstallLocation"),
                    entry.slim,
                    value("UninstallString"),
                );
            }
        }
        Err(error) => {
            let _ = writeln!(out, "unreadable ({error})");
        }
    }
    let _ = writeln!(out, "\n== Driver packages (pnputil /enum-drivers, usbip only)");
    // A tool that failed is not "no driver packages": the failure is what the file says.
    match run_capture("pnputil.exe", &["/enum-drivers"]) {
        Ok(text) => {
            let blocks = usbip_driver_blocks(&text);
            let _ = writeln!(out, "{}", if blocks.is_empty() { "none" } else { &blocks });
        }
        Err(error) => {
            let _ = writeln!(out, "unavailable ({error})");
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn read_setupapi_usbip_tail(max: usize) -> Result<Vec<String>, String> {
    use std::io::{Read, Seek, SeekFrom};

    let root = std::env::var_os("SystemRoot").ok_or_else(|| "unavailable".to_string())?;
    let path = std::path::PathBuf::from(root).join("INF").join("setupapi.dev.log");
    // The log grows to megabytes: only its tail is read.
    const TAIL_BYTES: u64 = 6 * 1024 * 1024;
    let text = std::fs::File::open(&path)
        .and_then(|mut file| {
            let length = file.metadata()?.len();
            file.seek(SeekFrom::Start(length.saturating_sub(TAIL_BYTES)))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        })
        .map_err(|error| format!("unreadable ({error})"))?;
    Ok(setupapi_usbip_lines(&text, max))
}

#[cfg(target_os = "windows")]
fn setupapi_section(out: &mut String) {
    let _ = writeln!(out, "\n== Device installation log (setupapi.dev.log, usbip lines, last 150)");
    match read_setupapi_usbip_tail(150) {
        Ok(lines) => {
            let _ = writeln!(out, "{}", if lines.is_empty() { "no usbip lines".to_string() } else { lines.join("\n") });
        }
        Err(error) => {
            let _ = writeln!(out, "{error}");
        }
    }
}

/// Writes the diagnostics next to the panel's log and shows the file in Explorer. Returns the path.
#[cfg(target_os = "windows")]
pub(super) async fn export() -> Result<String, String> {
    let mut text = String::from(HEADER);
    let _ = writeln!(text, "Created: {}", chrono::Local::now().to_rfc3339());
    windows_section(&mut text);
    let _ = writeln!(text, "\n== Panel status");
    match super::status::usbip_get_status().await {
        Ok(status) => {
            let json = serde_json::to_string_pretty(&status)
                .unwrap_or_else(|error| format!("(status could not be encoded: {error})"));
            let _ = writeln!(text, "{json}");
        }
        Err(error) => {
            let _ = writeln!(text, "unavailable: {error}");
        }
    }
    let kept = super::journal::read();
    let _ = writeln!(text, "\n== Last cleanup");
    let _ = writeln!(text, "restart marker: {:?}", kept.marker);
    let _ = writeln!(text, "last error: {}", kept.last_error);
    let _ = writeln!(text, "\n== Last install");
    let _ = writeln!(text, "last install error: {}", kept.last_install_error);
    // The slow reads (a process each) run off the async worker.
    let rest = tokio::task::spawn_blocking(|| {
        let mut part = String::new();
        devices_section(&mut part);
        services_section(&mut part);
        installations_section(&mut part);
        setupapi_section(&mut part);
        part
    })
    .await
    .map_err(|error| format!("USBIP-DIAG-001: the diagnostics could not be collected: {error}"))?;
    text.push_str(&rest);

    let directory = crate::logger::panel_log_path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or("USBIP-DIAG-001: the log folder is unavailable")?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("USBIP-DIAG-001: cannot create the log folder: {error}"))?;
    let path = directory.join(file_name(
        &chrono::Local::now().format("%Y%m%d-%H%M%S").to_string(),
    ));
    std::fs::write(&path, text)
        .map_err(|error| format!("USBIP-DIAG-001: cannot write the diagnostics: {error}"))?;
    reveal(&path);
    Ok(path.display().to_string())
}

#[cfg(not(target_os = "windows"))]
pub(super) async fn export() -> Result<String, String> {
    Err("USBIP-SETUP-004: USB/IP passthrough is only supported on Windows".to_string())
}

/// Shows the file selected in Explorer. Explorer answers exit code 1 even when it worked, so the result is not read.
#[cfg(target_os = "windows")]
fn reveal(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;

    let Some(root) = std::env::var_os("SystemRoot") else {
        return;
    };
    let _ = std::process::Command::new(std::path::PathBuf::from(root).join("explorer.exe"))
        .arg(format!("/select,{}", path.display()))
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_driver_package_blocks_that_name_usbip() {
        let output = "Microsoft PnP Utility\r\n\r\nPublished Name:     oem12.inf\r\nOriginal Name:      usbip2_ude.inf\r\nProvider Name:      USBip\r\n\r\nPublished Name:     oem3.inf\r\nOriginal Name:      netwtw.inf\r\n\r\nPublished Name:     oem13.inf\r\nOriginal Name:      usbip2_filter.inf\r\n";
        let blocks = usbip_driver_blocks(output);
        assert!(blocks.contains("oem12.inf"));
        assert!(blocks.contains("oem13.inf"));
        assert!(!blocks.contains("netwtw"));
        assert_eq!(usbip_driver_blocks("nothing here"), "");
    }

    #[test]
    fn keeps_the_setupapi_lines_about_usbip_and_its_root_nodes() {
        let log = ">>>  [Device Install (UpdateDriverForPlugAndPlayDevices) - ROOT\\USBIP_WIN2\\UDE]\nunrelated line\n     dvi: Class installer: usbip2_ude.inf\n!!!  dvi: Error 0xe0000247: A problem was encountered\n<<<  [Exit status: FAILURE(0xe0000247)]\n>>>  [Device Install - ROOT\\USB\\0000]\nanother unrelated\n";
        let lines = setupapi_usbip_lines(log, 10);
        assert_eq!(lines.len(), 5, "{lines:?}");
        assert!(lines.iter().all(|line| !line.contains("unrelated")));
        assert!(lines.iter().any(|line| line.contains("0xe0000247")));
        // only the last ones are kept
        assert_eq!(setupapi_usbip_lines(log, 2), lines[lines.len() - 2..].to_vec());
        assert!(setupapi_usbip_lines("nothing\nhere", 5).is_empty());
    }

    #[test]
    fn the_file_name_is_safe_and_sortable() {
        assert_eq!(file_name("20261120-093015"), "usbip-diagnostics-20261120-093015.txt");
        assert_eq!(file_name("2026/11:20 ..\\x"), "usbip-diagnostics-20261120x.txt");
    }
}
