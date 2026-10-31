//! Authenticode state of the registered usbip-win2 kernel driver.
//!
//! The Device Hub and the host's native DualSense path both rely on the signed
//! `usbip2_ude.sys`. The state is read from the system through
//! `Get-AuthenticodeSignature`, which also understands catalog-signed drivers,
//! and is never changed by the Panel.

use serde::{Deserialize, Serialize};

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

/// Reads the signature of the driver the system has registered for the transport.
#[cfg(target_os = "windows")]
pub(crate) async fn driver_signature() -> DriverSignature {
    let Ok(path) = super::manager::registered_driver_path() else {
        return unknown();
    };
    let mut command = tokio::process::Command::new(crate::elevation::system_binary(
        r"WindowsPowerShell\v1.0\powershell.exe",
    ));
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", PROBE_SCRIPT])
        .env("ALK_SIGNATURE_TARGET", path)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    match tokio::time::timeout(std::time::Duration::from_secs(20), command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            parse_signature_report(&String::from_utf8_lossy(&output.stdout))
        }
        _ => unknown(),
    }
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
}
