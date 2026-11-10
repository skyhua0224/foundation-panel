//! The components installed on this computer, each with its version and the short commit it was built from where the build embeds one (开发者 › 构建与组件).
//! Read at the time of the question from what is installed (the host's VERSIONINFO, the driver and package probes, the files next to the panel) plus the manifest the panel was built
//! with (`PANEL_COMPONENTS_JSON`, written by the build script: the commits of the panel, the Web UI bundle, foundation-ui, the plugins and the haptics library). A component that is not
//! installed is listed as such, never left out.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct ComponentRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub commit: String,
    pub installed: bool,
}

/// What the build recorded: `{ "panel": { "version", "commit" }, "web": {...}, "m3e": {...}, "stylus": {...}, "haptics": {...}, "installer": {...} }`. An empty or damaged value is an empty manifest.
pub(crate) fn manifest() -> Value {
    serde_json::from_str(option_env!("PANEL_COMPONENTS_JSON").unwrap_or("{}")).unwrap_or(Value::Null)
}

fn field(manifest: &Value, id: &str, key: &str) -> String {
    manifest.get(id).and_then(|c| c.get(key)).and_then(Value::as_str).unwrap_or("").to_string()
}

fn row(id: &str, name: &str, version: String, commit: String, installed: bool) -> ComponentRow {
    ComponentRow { id: id.into(), name: name.into(), version, commit, installed }
}

/// The rows from the facts the probes gave; pure, so the rules (what is listed, what is 未安装) are tested.
pub(crate) fn rows_from(
    manifest: &Value,
    host: Option<(String, String)>,
    vdd: Option<(bool, String)>,
    vigem: Option<(bool, String)>,
    usbip: Option<(bool, String)>,
    dlssnr: Option<(bool, String)>,
    stylus_present: bool,
) -> Vec<ComponentRow> {
    let host_version = host.as_ref().map(|(v, _)| v.clone()).unwrap_or_default();
    let panel_version = {
        let v = field(manifest, "panel", "version");
        if v.is_empty() { env!("CARGO_PKG_VERSION").to_string() } else { v }
    };
    let mut rows = vec![
        match host {
            Some((version, commit)) => row("host", "Sunshine.exe", version, commit, true),
            None => row("host", "Sunshine.exe", String::new(), String::new(), false),
        },
        row("panel", "Panel", panel_version, field(manifest, "panel", "commit"), true),
        // the Web UI bundle is served by the host: its version is the host's, the commit is the one it was built from
        row("web", "Web UI", host_version.clone(), field(manifest, "web", "commit"), !host_version.is_empty()),
        row("m3e", "foundation-ui", field(manifest, "m3e", "version"), field(manifest, "m3e", "commit"), true),
    ];
    let (vdd_on, vdd_version) = vdd.unwrap_or((false, String::new()));
    rows.push(row("vdd", "ZakoVDD", vdd_version, String::new(), vdd_on));
    let (vigem_on, vigem_version) = vigem.unwrap_or((false, String::new()));
    rows.push(row("vigem", "ViGEmBus", vigem_version, String::new(), vigem_on));
    let (usbip_on, usbip_version) = usbip.unwrap_or((false, String::new()));
    rows.push(row("usbip", "USB/IP", usbip_version, String::new(), usbip_on));
    let (dlss_on, dlss_hash) = dlssnr.unwrap_or((false, String::new()));
    rows.push(row("dlssnr", "DLSS NR plugin", String::new(), dlss_hash.chars().take(12).collect(), dlss_on));
    rows.push(row("stylus", "Stylus plugin", field(manifest, "stylus", "version"), field(manifest, "stylus", "commit"), stylus_present));
    let haptics_commit = field(manifest, "haptics", "commit");
    rows.push(row("haptics", "moonlight-audio-haptics", String::new(), haptics_commit.clone(), !haptics_commit.is_empty()));
    // the installer and its engine carry the version of the host they installed
    rows.push(row("installer", "Installer", host_version.clone(), field(manifest, "installer", "commit"), !host_version.is_empty()));
    rows
}

#[tauri::command]
pub async fn get_component_inventory() -> Result<Vec<ComponentRow>, String> {
    let host = crate::sunshine::get_sunshine_build_info().await.ok().flatten().map(|identity| (identity.version, identity.commit_short));
    let vdd = crate::vdd::get_vdd_status().await.ok().map(|s| (s.installed, s.installed_version));
    let vigem = crate::vigem::get_vigem_status().await.ok().map(|s| (s.installed, s.version));
    let usbip = crate::usbip::usbip_get_status().await.ok().map(|s| (s.installed, s.version));
    let dlssnr = crate::native_components::providers::nvidia_dlssnr::component_get_status().await.ok().map(|s| (s.installed, s.runtime_sha256));
    let stylus = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("alkaidlab-plugin-stylus.dll").exists())).unwrap_or(false);
    Ok(rows_from(&manifest(), host, vdd, vigem, usbip, dlssnr, stylus))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest() -> Value {
        json!({ "panel": { "version": "2026.1110.165906", "commit": "a6f75d9" }, "web": { "commit": "be1ce3d" }, "m3e": { "version": "0.1.0", "commit": "77d2f7a" },
                "stylus": { "version": "2026.1110.165906", "commit": "a6f75d9" }, "haptics": { "commit": "c4f1b8c" }, "installer": { "commit": "be1ce3d" } })
    }

    #[test]
    fn the_table_lists_every_component_and_marks_the_missing_ones() {
        let rows = rows_from(&manifest(), Some(("2026.1110.165906".into(), "abc1234".into())), Some((true, "1.0.4.0".into())), Some((false, String::new())), None, None, true);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["host", "panel", "web", "m3e", "vdd", "vigem", "usbip", "dlssnr", "stylus", "haptics", "installer"]);
        assert!(rows.iter().find(|r| r.id == "host").unwrap().installed && rows.iter().find(|r| r.id == "host").unwrap().commit == "abc1234");
        assert!(!rows.iter().find(|r| r.id == "vigem").unwrap().installed, "a driver that is not there is listed as not installed");
        assert!(!rows.iter().find(|r| r.id == "dlssnr").unwrap().installed);
        assert_eq!(rows.iter().find(|r| r.id == "panel").unwrap().version, "2026.1110.165906");
        assert_eq!(rows.iter().find(|r| r.id == "web").unwrap().version, "2026.1110.165906", "the Web UI bundle has the version of the host that serves it");
        assert_eq!(rows.iter().find(|r| r.id == "haptics").unwrap().commit, "c4f1b8c");
    }

    #[test]
    fn a_dev_build_has_no_manifest_and_says_so_and_a_missing_host_is_not_installed() {
        let rows = rows_from(&Value::Null, None, None, None, None, None, false);
        assert!(!rows.iter().find(|r| r.id == "host").unwrap().installed);
        assert!(!rows.iter().find(|r| r.id == "web").unwrap().installed);
        assert_eq!(rows.iter().find(|r| r.id == "panel").unwrap().version, env!("CARGO_PKG_VERSION"));
        assert_eq!(rows.iter().find(|r| r.id == "panel").unwrap().commit, "");
    }
}
