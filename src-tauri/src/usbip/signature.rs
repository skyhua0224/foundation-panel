//! Authenticode state of the registered usbip-win2 kernel driver.
//!
//! The Device Hub and the host's native DualSense path both rely on the signed
//! `usbip2_ude.sys`. The state is read from the system through
//! `Get-AuthenticodeSignature`, which also understands catalog-signed drivers,
//! and is never changed by the Panel.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
static SIGNATURE: super::memo::Memo = super::memo::Memo::new("usbip.signature");

/// Drops the kept signature (the driver is about to change).
#[cfg(target_os = "windows")]
pub(super) fn forget() {
    SIGNATURE.forget();
}

#[cfg(not(target_os = "windows"))]
pub(super) fn forget() {}

/// `signed` and `unsigned` are verdicts; `invalid` means a signature exists but
/// does not verify; `unknown` means it could not be read.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DriverSignature {
    pub state: String,
    pub signer: String,
}

#[derive(Deserialize)]
struct SignatureReport {
    #[serde(default)]
    status: String,
    #[serde(default)]
    signer: String,
}

fn unknown() -> DriverSignature {
    DriverSignature {
        state: "unknown".to_string(),
        signer: String::new(),
    }
}

/// Extracts the common name from an X.500 subject such as `CN="Name, Inc.", O=…`.
fn common_name(subject: &str) -> String {
    let Some(start) = subject.find("CN=") else {
        return String::new();
    };
    let rest = &subject[start + 3..];
    if let Some(quoted) = rest.strip_prefix('"') {
        return quoted.split('"').next().unwrap_or_default().to_string();
    }
    rest.split(',').next().unwrap_or_default().trim().to_string()
}

/// Maps the JSON written by the probe script to a signature verdict.
pub(super) fn parse_signature_report(output: &str) -> DriverSignature {
    let Ok(report) = serde_json::from_str::<SignatureReport>(output.trim()) else {
        return unknown();
    };
    let state = match report.status.as_str() {
        "Valid" => "signed",
        "NotSigned" => "unsigned",
        "UnknownError" | "" => return unknown(),
        // HashMismatch, NotTrusted, NotSupportedFileFormat, Incompatible.
        _ => "invalid",
    };
    DriverSignature {
        state: state.to_string(),
        signer: common_name(&report.signer),
    }
}

#[cfg(target_os = "windows")]
const PROBE_SCRIPT: &str = "$s = Get-AuthenticodeSignature -LiteralPath $env:ALK_SIGNATURE_TARGET; \
[pscustomobject]@{ status = [string]$s.Status; signer = if ($s.SignerCertificate) { $s.SignerCertificate.Subject } else { '' } } | ConvertTo-Json -Compress";

/// Reads the Authenticode state of one file through PowerShell.
#[cfg(target_os = "windows")]
async fn signature_of(path: &std::path::Path) -> DriverSignature {
    let mut command = tokio::process::Command::new(crate::elevation::system_binary(
        r"WindowsPowerShell\v1.0\powershell.exe",
    ));
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", PROBE_SCRIPT])
        .env("ALK_SIGNATURE_TARGET", path)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(super::memo::BACKGROUND_PROBE_FLAGS);
    match tokio::time::timeout(std::time::Duration::from_secs(20), command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            parse_signature_report(&crate::text_encoding::decode_console_bytes(&output.stdout))
        }
        _ => unknown(),
    }
}

/// Reads the signature of the driver the system has registered for the transport. The verdict belongs to the driver
/// file (size and time) and is kept (memo.rs), so PowerShell starts once per driver, not once per status read.
#[cfg(target_os = "windows")]
pub(crate) async fn driver_signature() -> DriverSignature {
    let Ok(path) = super::manager::registered_driver_path() else {
        return unknown();
    };
    let fingerprint = super::memo::file_identity(&path);
    SIGNATURE
        .get(&fingerprint, || async {
            // An unreadable answer is not a verdict: it is not kept.
            let signature = signature_of(&path).await;
            (signature.state != "unknown").then(|| format!("{}\u{1}{}", signature.state, signature.signer))
        })
        .await
        .and_then(|text| {
            let (state, signer) = text.split_once('\u{1}')?;
            Some(DriverSignature { state: state.to_string(), signer: signer.to_string() })
        })
        .unwrap_or_else(unknown)
}

#[cfg(not(target_os = "windows"))]
pub(crate) async fn driver_signature() -> DriverSignature {
    unknown()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_status_is_signed_and_names_the_signer() {
        let signature = parse_signature_report(
            r#"{"status":"Valid","signer":"CN=\"Vadim Grn, Example\", O=Example, C=UA"}"#,
        );
        assert_eq!(signature.state, "signed");
        assert_eq!(signature.signer, "Vadim Grn, Example");
        let plain = parse_signature_report(r#"{"status":"Valid","signer":"CN=Example Ltd, C=UA"}"#);
        assert_eq!(plain.signer, "Example Ltd");
    }

    #[test]
    fn not_signed_and_broken_signatures_are_distinct() {
        assert_eq!(
            parse_signature_report(r#"{"status":"NotSigned","signer":""}"#).state,
            "unsigned"
        );
        for status in ["HashMismatch", "NotTrusted", "Incompatible"] {
            let report = format!(r#"{{"status":"{status}","signer":""}}"#);
            assert_eq!(parse_signature_report(&report).state, "invalid", "{status}");
        }
    }

    #[test]
    fn unreadable_output_is_unknown_not_unsigned() {
        for output in ["", "not json", r#"{"status":"UnknownError"}"#, "{}"] {
            assert_eq!(parse_signature_report(output).state, "unknown", "{output:?}");
        }
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn a_signed_system_file_reads_as_signed() {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let file = std::path::Path::new(&root).join("System32").join("kernel32.dll");
        let signature = signature_of(&file).await;
        assert_eq!(signature.state, "signed", "{signature:?}");
        assert!(!signature.signer.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn an_unsigned_script_and_a_missing_file_are_not_called_signed() {
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("plain.ps1");
        std::fs::write(&script, "Write-Output 1\r\n").unwrap();
        assert_eq!(signature_of(&script).await.state, "unsigned");
        let missing = signature_of(&directory.path().join("absent.sys")).await;
        assert!(matches!(missing.state.as_str(), "unknown" | "invalid"), "{missing:?}");
    }
}
