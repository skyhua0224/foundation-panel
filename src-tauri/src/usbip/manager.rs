//! Shared usbip-win2 installation policy and discovery.
//!
//! Device Hub passthrough and the host's native DualSense path use the same
//! system-wide VHCI driver. Keep the version, registry discovery, executable
//! selection, and replacement policy here so one feature cannot silently
//! replace the transport underneath the other.

// This tested combination is shared with the host build. The host may use
// this driver, but must never install or replace it while creating devices.
pub(crate) const PINNED_VERSION: &str = env!("SUNSHINE_USBIP_VERSION");
pub(super) const REQUIRED_VERSION: &str = PINNED_VERSION;
pub(crate) const INSTALLER_URL: &str = env!("SUNSHINE_USBIP_URL");
pub(crate) const INSTALLER_SHA256: &str = env!("SUNSHINE_USBIP_SHA256");

/// Rejects missing or replaced transport binaries using the contract's SHA-256.
#[cfg(target_os = "windows")]
fn verify_transport_file(path: &std::path::Path, expected: &str) -> Result<(), String> {
    let actual = crate::native_components::install::sha256_file(path, u64::MAX)
        .map_err(|error| format!("USBIP-EXEC-006: cannot verify the shared transport: {error}"))?;
    if actual != expected {
        return Err(
            "USBIP-EXEC-006: the shared transport does not match the tested component combination"
                .to_string(),
        );
    }
    Ok(())
}

/// Resolves the registered kernel driver inside the Windows driver store.
#[cfg(target_os = "windows")]
pub(super) fn registered_driver_path() -> Result<std::path::PathBuf, String> {
    use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};
    let service = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Services\usbip2_ude")
        .map_err(|error| {
            format!("USBIP-EXEC-006: shared driver registration is unavailable: {error}")
        })?;
    let image: String = service
        .get_value("ImagePath")
        .map_err(|error| format!("USBIP-EXEC-006: shared driver path is unavailable: {error}"))?;
    let windows = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").ok_or("USBIP-EXEC-006: Windows directory is unavailable")?,
    );
    let path = if image.to_ascii_lowercase().starts_with(r"\systemroot\") {
        windows.join(&image[12..])
    } else if let Some(path) = image.strip_prefix(r"\??\") {
        std::path::PathBuf::from(path)
    } else if image.to_ascii_lowercase().starts_with(r"system32\") {
        windows.join(image)
    } else {
        std::path::PathBuf::from(image)
    };
    let path = path
        .canonicalize()
        .map_err(|error| format!("USBIP-EXEC-006: shared driver is unavailable: {error}"))?;
    let windows = windows
        .canonicalize()
        .map_err(|error| format!("USBIP-EXEC-006: cannot resolve Windows directory: {error}"))?;
    if !path.starts_with(windows)
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !name.eq_ignore_ascii_case("usbip2_ude.sys"))
    {
        return Err(
            "USBIP-EXEC-006: shared driver path is outside the Windows driver store".to_string(),
        );
    }
    Ok(path)
}

/// Checks the registered driver path and bytes without changing driver state.
#[cfg(target_os = "windows")]
pub(super) fn verify_driver_compatibility() -> Result<(), String> {
    let path = registered_driver_path()?;
    verify_transport_file(&path, env!("SUNSHINE_USBIP_DRIVER_SHA256"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallDisposition {
    Ready,
    Install,
}

/// Only the explicitly tested combination is reused; newer is not an ABI guarantee.
/// Replacing an unsupported installation requires explicit shared cleanup.
pub(crate) fn install_disposition(
    installed_version: Option<&str>,
) -> Result<InstallDisposition, String> {
    match installed_version {
        Some(version) if supported_version_installed(Some(version)) => {
            Ok(InstallDisposition::Ready)
        }
        None => Ok(InstallDisposition::Install),
        Some(version) => Err(format!(
            "USBIP-SETUP-008: USB/IP {version} is installed; it is externally managed; choose whether to replace it using Windows Installed apps before installing {PINNED_VERSION}"
        )),
    }
}

/// Accepts only the tested four-part version, including neither older nor newer ABIs.
pub(crate) fn supported_version_installed(installed_version: Option<&str>) -> bool {
    installed_version
        .and_then(parse_version)
        .is_some_and(|version| {
            version == parse_version(PINNED_VERSION).expect("valid pinned USB/IP version")
        })
}

/// Requires four unsigned numeric components; suffixes, signs and overflow are invalid.
fn parse_version(value: &str) -> Option<[u32; 4]> {
    let components: Vec<_> = value.split('.').collect();
    if components.len() != 4 {
        return None;
    }
    let mut version = [0; 4];
    for (index, component) in components.iter().enumerate() {
        if component.is_empty() || !component.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        version[index] = component.parse().ok()?;
    }
    Some(version)
}

// Inno Setup's English display name used to include "version". Newer
// usbip-win2 installers register the same product as "USBip 0.9.8.1".
// Keep discovery and the missing-DisplayVersion fallback consistent.
/// Recognizes both usbip-win2 installer names without matching other USB/IP products.
fn version_from_display_name(display_name: &str) -> Option<&str> {
    display_name.strip_prefix("USBip version ").or_else(|| {
        display_name
            .strip_prefix("USBip ")
            .filter(|version| parse_version(version).is_some())
    })
}

/// Rejects malformed or conflicting registrations instead of selecting whichever entry appears first.
fn consistent_version(
    versions: impl IntoIterator<Item = String>,
) -> Result<Option<String>, String> {
    let mut found: Option<String> = None;
    for value in versions {
        let parsed = parse_version(&value).ok_or_else(|| {
            "USBIP-SETUP-008: invalid USB/IP registration; clean the shared transport first"
                .to_string()
        })?;
        if found
            .as_deref()
            .and_then(parse_version)
            .is_some_and(|previous| previous != parsed)
        {
            return Err(
                "USBIP-SETUP-008: conflicting USB/IP versions; clean the shared transport first"
                    .to_string(),
            );
        }
        found.get_or_insert(value);
    }
    Ok(found)
}

/// An explicit receipt identifies the slim layout independently of localized ARP text.
/// usbip-slim-ms-reg-uninstall-20261117: InstallLocation / DisplayVersion.
// panel-usbip-winreg-20261117: typed registry values, no shell parsing.
fn slim_registration(key: &str, layout: &str) -> Result<bool, String> {
    if !key.eq_ignore_ascii_case("AlkaidLabUSBip") {
        return Ok(false);
    }
    if layout != "usbip-slim-v1" {
        return Err("USBIP-SETUP-008: unsupported USB/IP installation layout".into());
    }
    Ok(true)
}

#[cfg(target_os = "windows")]
pub(super) fn verify_slim_payload(root: &std::path::Path) -> Result<(), String> {
    let receipt = root.join("payload.json");
    verify_transport_file(&receipt, env!("SUNSHINE_USBIP_PAYLOAD_SHA256"))?;
    let bytes = std::fs::read(&receipt)
        .map_err(|error| format!("USBIP-EXEC-006: cannot read USB/IP receipt: {error}"))?;
    // panel-usbip-serde-json-20261117, from_slice: parse the hash-pinned UTF-8 receipt.
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("USBIP-EXEC-006: invalid USB/IP receipt: {error}"))?;
    let files = manifest["files"]
        .as_object()
        .ok_or("USBIP-EXEC-006: missing USB/IP file inventory")?;
    for (name, record) in files {
        // The receipt is hash-pinned above; retain this path boundary check as
        // a separate invariant before using a receipt member as a file path.
        if name
            .split('/')
            .any(|part| part.is_empty() || part == ".." || part.contains([':', '\\']))
        {
            return Err("USBIP-EXEC-006: invalid USB/IP receipt member".into());
        }
        let digest = record["sha256"]
            .as_str()
            .ok_or("USBIP-EXEC-006: missing USB/IP member hash")?;
        verify_transport_file(&root.join(name), digest)?;
    }
    Ok(())
}

/// usbip-policy-reg-20261117: ARP version is not proof of origin.
/// GUI/SDK, a different version or uncertainty always preserve external ownership.
fn external_ownership(present: bool, compatible: bool, extras: bool, owned: bool) -> bool {
    present && (extras || !compatible || !owned)
}

#[cfg(target_os = "windows")]
pub(crate) fn externally_managed() -> bool {
    let inspect = || -> Result<bool, String> {
        let default_root = std::path::PathBuf::from(
            std::env::var_os("ProgramW6432")
                .or_else(|| std::env::var_os("ProgramFiles"))
                .ok_or("Windows program directory unavailable")?,
        )
        .join("USBip");
        let mut residual = false;
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        for name in ["usbip2_ude", "usbip2_filter", "usbip2_vhci"] {
            match hklm.open_subkey(format!(r"SYSTEM\CurrentControlSet\Services\{name}")) {
                Ok(_) => residual = true,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        inspect_ownership(
            &uninstall_entries()?,
            &default_root,
            &crate::sunshine::install_dir(),
            residual,
        )
    };
    inspect().unwrap_or(true)
}

// usbip-policy-winreg-20261117, get_value; usbip-policy-files-20261117,
// presence and read failures. usbip-policy-rust-path-20261117, try_exists:
// propagate permission errors instead of classifying an unreadable layout as absent.
#[cfg(target_os = "windows")]
fn inspect_ownership(
    entries: &[UninstallEntry],
    default_root: &std::path::Path,
    host_root: &std::path::Path,
    residual: bool,
) -> Result<bool, String> {
    let mut roots = vec![default_root.to_path_buf()];
    for entry in entries {
        roots.push(std::path::PathBuf::from(
            entry
                .key
                .get_value::<String, _>("InstallLocation")
                .map_err(|_| "USB/IP install location unavailable")?,
        ));
    }
    let mut extras = false;
    let mut present = !entries.is_empty() || residual;
    for root in &roots {
        for name in ["wusbip.exe", "include", "lib"] {
            extras |= root.join(name).try_exists().map_err(|e| e.to_string())?;
        }
        for name in ["usbip.exe", "payload.json", "devnode.exe"] {
            present |= root.join(name).try_exists().map_err(|e| e.to_string())?;
        }
    }
    let version = consistent_version(entries.iter().map(entry_version))?;
    let compatible = supported_version_installed(version.as_deref());
    let same_root = roots.iter().all(|root| root == default_root);
    let receipt = entries.len() == 1
        && entries[0].slim
        && same_root
        && entries[0]
            .key
            .get_value::<String, _>("SourceSHA256")
            .is_ok_and(|hash| hash == INSTALLER_SHA256);
    let bundled = host_root
        .join("scripts")
        .join("usbip")
        .join(format!("USBip-{PINNED_VERSION}-x64.exe"));
    let legacy = !entries.is_empty()
        && same_root
        && entries.iter().all(|entry| !entry.slim)
        && crate::native_components::install::sha256_file(&bundled, u64::MAX)
            .is_ok_and(|hash| hash == INSTALLER_SHA256);
    Ok(external_ownership(
        present,
        compatible,
        extras,
        receipt || legacy,
    ))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn externally_managed() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(crate) struct UninstallEntry {
    pub(crate) key: winreg::RegKey,
    pub(crate) display_name: String,
    pub(crate) slim: bool,
}

#[cfg(target_os = "windows")]
pub(crate) fn uninstall_entries() -> Result<Vec<UninstallEntry>, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    read_uninstall_entries(
        &RegKey::predef(HKEY_LOCAL_MACHINE),
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    )
}

#[cfg(target_os = "windows")]
fn read_uninstall_entries(
    hklm: &winreg::RegKey,
    uninstall: &str,
) -> Result<Vec<UninstallEntry>, String> {
    use winreg::enums::{KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    let mut entries = Vec::new();
    for view in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
        let root = match hklm.open_subkey_with_flags(uninstall, view) {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "USBIP-SETUP-002: cannot inspect uninstall registry: {error}"
                ));
            }
        };
        for name in root.enum_keys() {
            let name = name.map_err(|error| {
                format!("USBIP-SETUP-002: cannot enumerate uninstall registry: {error}")
            })?;
            let key = match root.open_subkey_with_flags(&name, view) {
                Ok(key) => key,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "USBIP-SETUP-002: cannot inspect uninstall entry: {error}"
                    ));
                }
            };
            let display_name = key
                .get_value::<String, _>("DisplayName")
                .unwrap_or_default();
            let layout = key
                .get_value::<String, _>("InstallLayout")
                .unwrap_or_default();
            let slim = slim_registration(&name, &layout)?;
            if slim
                || name.eq_ignore_ascii_case("{199505b0-b93d-4521-a8c7-897818e0205a}_is1")
                || version_from_display_name(&display_name).is_some()
            {
                entries.push(UninstallEntry {
                    key,
                    display_name,
                    slim,
                });
            }
        }
    }
    Ok(entries)
}

#[cfg(target_os = "windows")]
pub(crate) fn installed_version() -> Result<Option<String>, String> {
    consistent_version(uninstall_entries()?.iter().map(entry_version))
}

#[cfg(target_os = "windows")]
fn entry_version(entry: &UninstallEntry) -> String {
    entry
        .key
        .get_value::<String, _>("DisplayVersion")
        .unwrap_or_else(|_| {
            version_from_display_name(&entry.display_name)
                .unwrap_or_default()
                .to_string()
        })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn installed_version() -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub(super) struct Installation {
    pub(super) version: String,
    pub(super) executable: std::path::PathBuf,
}

/// Resolves the registered CLI and verifies its pinned bytes before execution.
#[cfg(target_os = "windows")]
pub(super) fn find_installation() -> Result<Installation, String> {
    let entries = uninstall_entries()?;
    let version = consistent_version(entries.iter().map(entry_version))?
        .ok_or_else(|| "USBIP-SETUP-001: USB/IP transport is not installed".to_string())?;
    let mut installation = None;
    for entry in entries {
        let candidate = (|| -> Result<Installation, String> {
            let install_location = entry
                .key
                .get_value::<String, _>("InstallLocation")
                .map_err(|_| "USBIP-SETUP-002: USB/IP install location is missing".to_string())?;
            let root = std::path::PathBuf::from(install_location)
                .canonicalize()
                .map_err(|error| {
                    format!("USBIP-SETUP-002: USB/IP install location is invalid: {error}")
                })?;
            let executable = root.join("usbip.exe").canonicalize().map_err(|error| {
                format!(
                    "USBIP-SETUP-002: usbip.exe is missing from the installed transport: {error}"
                )
            })?;
            if !executable.starts_with(&root)
                || !executable
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("usbip.exe"))
            {
                return Err(
                    "USBIP-SETUP-002: the installed USB/IP executable path is unsafe".to_string(),
                );
            }
            if entry.slim {
                verify_slim_payload(&root)?;
            }
            if supported_version_installed(Some(&version)) {
                verify_transport_file(&executable, env!("SUNSHINE_USBIP_TOOL_SHA256"))?;
            }
            Ok(Installation {
                version: version.clone(),
                executable,
            })
        })();
        // A valid registration must not hide another broken registration.
        let candidate = candidate?;
        installation.get_or_insert(candidate);
    }
    installation.ok_or_else(|| "USBIP-SETUP-001: USB/IP transport is not installed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn shared_binary_verification_rejects_replaced_or_missing_files() {
        let file = std::env::temp_dir().join(format!("usbip-verify-{}", uuid::Uuid::new_v4()));
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        std::fs::write(&file, b"hello").unwrap();
        assert!(verify_transport_file(&file, expected).is_ok());
        std::fs::write(&file, b"replaced").unwrap();
        assert!(
            verify_transport_file(&file, expected)
                .unwrap_err()
                .starts_with("USBIP-EXEC-006:")
        );
        std::fs::remove_file(&file).unwrap();
        assert!(verify_transport_file(&file, expected).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn registry_and_file_layout_fixtures_preserve_external_installs() {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let id = uuid::Uuid::new_v4().to_string();
        let path = format!(r"Software\UsbipOwnershipTests\{id}");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let work = std::env::temp_dir().join(format!("usbip-ownership-{id}"));
        std::fs::create_dir_all(&work).unwrap();
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/usbip-ownership-fixtures.json"))
                .unwrap();
        let payload = std::env::var_os("USBIP_LEGACY_FIXTURE_INSTALLER");
        for f in fixtures.as_array().unwrap() {
            let name = f["name"].as_str().unwrap();
            let key_path = format!(r"{path}\{name}");
            let root = work.join(name).join("USBip");
            let host = work.join(name).join("host");
            std::fs::create_dir_all(&root).unwrap();
            let present = f["present"].as_bool().unwrap();
            let slim = f["receipt"].as_bool().unwrap();
            if present {
                std::fs::write(root.join("usbip.exe"), b"fixture").unwrap();
                let key_name = if slim {
                    "AlkaidLabUSBip"
                } else {
                    "{199505b0-b93d-4521-a8c7-897818e0205a}_is1"
                };
                let (key, _) = hkcu
                    .create_subkey(format!(r"{key_path}\{key_name}"))
                    .unwrap();
                key.set_value("DisplayVersion", &f["version"].as_str().unwrap())
                    .unwrap();
                key.set_value("InstallLocation", &root.to_str().unwrap())
                    .unwrap();
                if slim {
                    key.set_value("InstallLayout", &"usbip-slim-v1").unwrap();
                    key.set_value("SourceSHA256", &INSTALLER_SHA256).unwrap();
                }
            }
            if f["extras"].as_bool().unwrap() {
                std::fs::write(root.join("wusbip.exe"), b"fixture").unwrap();
                std::fs::create_dir_all(root.join("include")).unwrap();
                std::fs::create_dir_all(root.join("lib")).unwrap();
                std::fs::write(root.join("lib").join("libusbip.lib"), b"fixture").unwrap();
            }
            if f["legacy"].as_bool().unwrap() {
                let bundled = host
                    .join("scripts")
                    .join("usbip")
                    .join(format!("USBip-{PINNED_VERSION}-x64.exe"));
                std::fs::create_dir_all(bundled.parent().unwrap()).unwrap();
                if let Some(ref payload) = payload {
                    std::fs::copy(payload, &bundled).unwrap();
                } else {
                    std::fs::write(&bundled, b"unverified installer").unwrap();
                }
            }
            let mut entries = read_uninstall_entries(&hkcu, &key_path).unwrap();
            // HKCU Software is shared across registry views; deduplicate the
            // intentionally identical fixture observations for the receipt case.
            if slim {
                entries.truncate(1);
            }
            let external = inspect_ownership(&entries, &root, &host, false).unwrap();
            let expected =
                f["ownership"] == "external" || (name == "legacy-owned" && payload.is_none());
            assert_eq!(external, expected, "{name}");
            println!("PASS registry-layout/{name}: external={external}");
            hkcu.delete_subkey_all(&key_path).ok();
        }
        hkcu.delete_subkey_all(&path).ok();
        std::fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn ownership_scenarios_preserve_user_installations() {
        for (name, present, compatible, extras, owned, external) in [
            ("external-full", true, true, true, false, true),
            ("legacy-owned", true, true, false, true, false),
            ("incompatible", true, false, false, true, true),
            ("slim", true, true, false, true, false),
            ("absent", false, false, false, false, false),
            ("observed-full-double-install", true, true, true, true, true),
            ("unknown-origin", true, true, false, false, true),
            ("inspection-incomplete", true, false, false, false, true),
        ] {
            assert_eq!(
                external_ownership(present, compatible, extras, owned),
                external,
                "{name}"
            );
        }
    }

    #[test]
    fn slim_receipt_is_recognized_without_a_vendor_display_name() {
        assert!(slim_registration("AlkaidLabUSBip", "usbip-slim-v1").unwrap());
        assert!(slim_registration("alkaidlabusbip", "usbip-slim-v1").unwrap());
        assert!(!slim_registration("other", "usbip-slim-v1").unwrap());
        assert!(slim_registration("AlkaidLabUSBip", "").is_err());
        assert!(slim_registration("AlkaidLabUSBip", "future").is_err());
    }

    #[test]
    fn discovers_legacy_and_current_installer_display_names() {
        for (name, version) in [
            ("USBip version 0.9.7.7", "0.9.7.7"),
            ("USBip version 0.9.8.0", "0.9.8.0"),
            ("USBip 0.9.8.0", "0.9.8.0"),
            ("USBip version 0.9.8.1", "0.9.8.1"),
            ("USBip 0.9.8.1", "0.9.8.1"),
        ] {
            let discovered = version_from_display_name(name);
            assert_eq!(discovered, Some(version));
        }
        // Keep malformed legacy registrations visible to version validation,
        // without treating other USB/IP products as usbip-win2.
        assert_eq!(version_from_display_name("USBip version bad"), Some("bad"));
        for name in [
            "USBip Tools",
            "USBip Server",
            "Other USBip 0.9.8.1",
            "USBip",
        ] {
            assert_eq!(version_from_display_name(name), None, "{name}");
        }
    }

    #[test]
    fn mixed_installer_display_names_still_detect_conflicting_versions() {
        for names in [
            ["USBip version 0.9.7.7", "USBip 0.9.8.1"],
            ["USBip 0.9.8.1", "USBip version 0.9.7.7"],
            ["USBip 0.9.8.0", "USBip 0.9.8.1"],
        ] {
            assert!(
                consistent_version(
                    names.map(|name| version_from_display_name(name).unwrap().to_string())
                )
                .is_err()
            );
        }
        assert_eq!(
            consistent_version(
                ["USBip version 0.9.8.1", "USBip 0.9.8.1"].map(|name| version_from_display_name(
                    name
                )
                .unwrap()
                .to_string())
            )
            .unwrap(),
            Some("0.9.8.1".to_string())
        );
    }

    #[test]
    fn shared_transport_never_silently_replaces_another_version() {
        assert_eq!(
            install_disposition(Some(PINNED_VERSION)).unwrap(),
            InstallDisposition::Ready
        );
        assert_eq!(
            install_disposition(None).unwrap(),
            InstallDisposition::Install
        );
        assert!(install_disposition(Some("0.9.8.2")).is_err());
        assert!(install_disposition(Some("0.9.7.6")).is_err());
        assert!(install_disposition(Some("0.9.7.7")).is_err());
    }

    #[test]
    fn only_the_tested_transport_version_is_accepted() {
        assert!(supported_version_installed(Some(PINNED_VERSION)));
        for version in [
            "0.9.8.2",
            "0.9.8.10",
            "0.10.0.0",
            "1.0.0.0",
            "0.9.7.6",
            "0.9.7.7",
            "0.9.7.10",
            "0.9.8.0",
            "0.8.99.99",
            "",
            "0.9.7",
            "0.9.7.8-beta",
            "0.9.7.+8",
            "0.9.7.4294967296",
        ] {
            assert!(!supported_version_installed(Some(version)), "{version}");
        }
        assert!(!supported_version_installed(None));
    }

    #[test]
    fn all_registrations_must_agree_regardless_of_order() {
        for versions in [
            vec!["0.9.7.7", "0.9.7.8"],
            vec!["0.9.7.8", "0.9.7.7"],
            vec!["0.9.7.7", ""],
            vec!["bad", "0.9.7.8"],
        ] {
            assert!(consistent_version(versions.into_iter().map(str::to_string)).is_err());
        }
        assert_eq!(
            consistent_version(["0.9.7.8".into(), "0.9.7.8".into()]).unwrap(),
            Some("0.9.7.8".into())
        );
        assert_eq!(consistent_version(Vec::new()).unwrap(), None);
    }
}
