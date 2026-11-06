//! Build identity of the installed host (sunshine.exe): full commit, channel,
//! UTC build time, variant and package. The host records them at build time
//! as custom String keys of its VERSIONINFO (cmake/prep/build_info.cmake of
//! the host), so the panel reads them without starting the program; a host
//! without the keys answers `sunshine.exe --version --json` (same fields), and
//! an older host has neither. Pure: the Windows reads live in `sunshine.rs`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The VERSIONINFO String keys the host writes.
pub(crate) const VERSIONINFO_KEYS: [&str; 10] = [
    "FileVersion",
    "Commit",
    "Branch",
    "Dirty",
    "Channel",
    "BuildTime",
    "Variant",
    "Package",
    "Builder",
    "CiRun",
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    pub version: String,
    pub commit: String,
    pub commit_short: String,
    pub branch: String,
    pub dirty: bool,
    /// stable | beta | dev | local; empty when the host does not say
    pub channel: String,
    /// ISO 8601 UTC with Z
    pub build_time_utc: String,
    pub variant: String,
    /// installer | portable
    pub package: String,
    /// build (fixed when built) | layout (decided from the install folder)
    pub package_source: String,
    /// ci | local
    pub builder: String,
    pub ci_run: String,
    /// versioninfo | version-json
    pub source: String,
}

const CHANNELS: [&str; 4] = ["stable", "beta", "dev", "local"];

fn short(commit: &str) -> String {
    commit.chars().take(12).collect()
}

fn valid_commit(commit: &str) -> bool {
    !commit.is_empty() && commit.len() <= 40 && commit.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The package of a build that left it to the install layout ("auto").
pub(crate) fn resolve_package(recorded: &str, has_inno_uninstaller: bool) -> (String, String) {
    match recorded {
        "installer" | "portable" => (recorded.to_string(), "build".to_string()),
        _ => (
            if has_inno_uninstaller { "installer" } else { "portable" }.to_string(),
            "layout".to_string(),
        ),
    }
}

/// An Inno Setup uninstaller name (unins000.exe, unins001.exe, ...).
pub(crate) fn is_inno_uninstaller(file_name: &str) -> bool {
    let name = file_name.to_ascii_lowercase();
    name.len() == 12
        && name.starts_with("unins")
        && name.ends_with(".exe")
        && name[5..8].bytes().all(|b| b.is_ascii_digit())
}

/// From the VERSIONINFO strings. `None` when the host predates the keys
/// (no valid Commit): the caller then asks `--version --json`.
pub(crate) fn from_versioninfo(strings: &HashMap<String, String>, has_inno_uninstaller: bool) -> Option<BuildIdentity> {
    let get = |key: &str| strings.get(key).map(|value| value.trim().to_string()).unwrap_or_default();
    let commit = get("Commit").to_ascii_lowercase();
    if !valid_commit(&commit) {
        return None;
    }
    let channel = get("Channel");
    let (package, package_source) = resolve_package(&get("Package"), has_inno_uninstaller);
    Some(BuildIdentity {
        version: get("FileVersion"),
        commit_short: short(&commit),
        commit,
        branch: get("Branch"),
        dirty: get("Dirty") == "1",
        channel: if CHANNELS.contains(&channel.as_str()) { channel } else { String::new() },
        build_time_utc: get("BuildTime"),
        variant: get("Variant"),
        package,
        package_source,
        builder: get("Builder"),
        ci_run: get("CiRun"),
        source: "versioninfo".to_string(),
    })
}

/// From `sunshine.exe --version --json` (the host already resolved the package).
pub(crate) fn from_version_json(output: &str) -> Option<BuildIdentity> {
    let line = output.lines().map(str::trim).find(|line| line.starts_with('{'))?;
    let json: serde_json::Value = serde_json::from_str(line).ok()?;
    let text = |key: &str| json.get(key).and_then(|v| v.as_str()).unwrap_or_default().trim().to_string();
    let commit = text("commit").to_ascii_lowercase();
    if !valid_commit(&commit) {
        return None;
    }
    let channel = text("channel");
    Some(BuildIdentity {
        version: text("version"),
        commit_short: short(&commit),
        commit,
        branch: text("branch"),
        dirty: json.get("dirty").and_then(|v| v.as_bool()).unwrap_or(false),
        channel: if CHANNELS.contains(&channel.as_str()) { channel } else { String::new() },
        build_time_utc: text("build_time_utc"),
        variant: text("variant"),
        package: text("package"),
        package_source: text("package_source"),
        builder: text("builder"),
        ci_run: text("ci_run"),
        source: "version-json".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    const COMMIT: &str = "96a1e2b68829134784b718c9471b8f58e61e8b89";

    #[test]
    fn versioninfo_keys_become_the_identity() {
        let info = from_versioninfo(
            &strings(&[
                ("FileVersion", "2026.1104.121813"),
                ("Commit", COMMIT),
                ("Branch", "master"),
                ("Dirty", "0"),
                ("Channel", "beta"),
                ("BuildTime", "2026-11-04T04:18:13Z"),
                ("Variant", "foundation-sunshine"),
                ("Package", "auto"),
                ("Builder", "ci"),
                ("CiRun", "https://github.com/x/y/actions/runs/1"),
            ]),
            true,
        )
        .unwrap();
        assert_eq!(info.commit, COMMIT);
        assert_eq!(info.commit_short, "96a1e2b68829");
        assert_eq!(info.channel, "beta");
        assert!(!info.dirty);
        assert_eq!(info.package, "installer");
        assert_eq!(info.package_source, "layout");
        assert_eq!(info.source, "versioninfo");
        assert_eq!(info.version, "2026.1104.121813");
    }

    #[test]
    fn a_host_without_the_keys_has_no_identity() {
        assert_eq!(from_versioninfo(&strings(&[("FileVersion", "2026.1104.121813")]), false), None);
        assert_eq!(from_versioninfo(&strings(&[("Commit", "not-hex")]), false), None);
    }

    #[test]
    fn unknown_channels_are_not_shown() {
        let info = from_versioninfo(&strings(&[("Commit", COMMIT), ("Channel", "nightly")]), false).unwrap();
        assert_eq!(info.channel, "");
        assert_eq!(info.package, "portable");
    }

    #[test]
    fn a_fixed_package_is_kept() {
        assert_eq!(resolve_package("portable", true), ("portable".into(), "build".into()));
        assert_eq!(resolve_package("auto", false), ("portable".into(), "layout".into()));
        assert_eq!(resolve_package("", true), ("installer".into(), "layout".into()));
    }

    #[test]
    fn inno_uninstaller_names() {
        assert!(is_inno_uninstaller("unins000.exe"));
        assert!(is_inno_uninstaller("UNINS001.EXE"));
        assert!(!is_inno_uninstaller("unins000.dat"));
        assert!(!is_inno_uninstaller("uninstall.exe"));
        assert!(!is_inno_uninstaller("unins0000.exe"));
    }

    #[test]
    fn version_json_is_read_from_its_line() {
        let output = format!(
            "{{\"json_version\":1,\"version\":\"2026.1104.121813\",\"commit\":\"{COMMIT}\",\"branch\":\"master\",\"dirty\":true,\"channel\":\"dev\",\"build_time_utc\":\"2026-11-04T04:18:13Z\",\"variant\":\"foundation-sunshine\",\"package\":\"portable\",\"package_source\":\"layout\",\"builder\":\"local\",\"ci_run\":\"\"}}\n"
        );
        let info = from_version_json(&output).unwrap();
        assert_eq!(info.channel, "dev");
        assert!(info.dirty);
        assert_eq!(info.package, "portable");
        assert_eq!(info.source, "version-json");
        // an older host prints its plain version line
        assert_eq!(from_version_json("Sunshine version: 2026.1104.121813\n"), None);
    }
}
