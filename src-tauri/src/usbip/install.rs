//! Explicit, user-confirmed installation of the pinned usbip-win2 transport.
//!
//! This runs only inside the elevated helper, after the Device Hub asked the
//! person to confirm the installation. The installer is staged in a directory
//! that only administrators can write, hashed against the pinned digest and
//! only then executed.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) enum InstallResult {
    Ready,
    RebootRecommended,
}

/// Inno Setup exits with the code requested by `/RESTARTEXITCODE`, and `usbip-package.ps1`
/// reports guard/ownership outcomes via 102..106.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_installer_exit_code(
    exit_code: Option<i32>,
) -> Result<InstallResult, String> {
    classify_installer_outcome(exit_code, "", "")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn compact_diagnostic_excerpt(output: &str, setupapi_tail: &str) -> String {
    let mut picked: Vec<String> = Vec::new();
    for line in output.lines().chain(setupapi_tail.lines()) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        let interesting = trimmed.starts_with("!!!")
            || trimmed.starts_with("<<<")
            || lower.contains("error")
            || lower.contains("fail")
            || lower.contains("0x")
            || lower.contains("cm_prob")
            || lower.contains("problem")
            || lower.contains("pnputil")
            || lower.contains("devnode")
            || lower.contains("blocked")
            || lower.contains("reboot")
            || lower.contains("restart")
            || lower.contains("exit")
            || lower.contains("warning")
            || lower.contains("=");
        if interesting && !picked.iter().any(|existing| existing == trimmed) {
            picked.push(trimmed.to_string());
        }
    }
    if picked.is_empty() {
        picked = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
    }
    if picked.len() > 6 {
        picked.drain(..picked.len() - 6);
    }
    let joined = picked.join(" | ");
    const MAX_LEN: usize = 420;
    if joined.chars().count() > MAX_LEN {
        joined.chars().take(MAX_LEN).collect::<String>() + "..."
    } else {
        joined
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_installer_outcome(
    exit_code: Option<i32>,
    output: &str,
    setupapi_tail: &str,
) -> Result<InstallResult, String> {
    match exit_code {
        Some(0) => return Ok(InstallResult::Ready),
        Some(3010) => return Ok(InstallResult::RebootRecommended),
        _ => {}
    }
    let code_num = exit_code.unwrap_or(-1);
    let excerpt = compact_diagnostic_excerpt(output, setupapi_tail);
    let suffix = if excerpt.is_empty() {
        String::new()
    } else {
        format!(" [{excerpt}]")
    };
    let lower = format!("{output}\n{setupapi_tail}").to_ascii_lowercase();

    match exit_code {
        Some(102) => Err(format!(
            "USBIP-SETUP-011: USB root hub check failed and was rolled back (exit code 102){suffix}"
        )),
        Some(103) => Err(format!(
            "USBIP-SETUP-012: USB root hub check failed and rollback was incomplete (exit code 103){suffix}"
        )),
        Some(104) => Err(format!(
            "USBIP-SETUP-013: Windows appears to boot from a USB disk (exit code 104){suffix}"
        )),
        Some(105) => Err(format!(
            "USBIP-SETUP-014: USB root hub guard could not run (exit code 105){suffix}"
        )),
        Some(106) => Err(format!(
            "USBIP-SETUP-008: externally managed incompatible USB/IP is installed (exit code 106){suffix}"
        )),
        _ if contains_any(
            &lower,
            &[
                "smart app control",
                "application control",
                "0x11c7",
                "0x800711c7",
                "os error 4551",
                "error 4551",
            ],
        ) =>
        {
            Err(format!(
                "USBIP-SETUP-017: Windows Smart App Control or Application Control blocked the installer (exit code {code_num}){suffix}"
            ))
        }
        _ if contains_any(
            &lower,
            &[
                "driver is blocked",
                "vulnerable driver",
                "memory integrity",
                "hvci",
                "code integrity",
                "policy blocked",
                "not digitally signed",
                "cm_prob_driver_blocked",
                "cm_prob_unsigned_driver",
                "0xe0000247",
                "0xe0000248",
                "0xe000022f",
                "0xc0000428",
                "0x80096010",
                "0x4fb",
                "error 1275",
                "error 577",
                "0x241",
                "code 48",
                "code 52",
                "code 39",
                "problemcode=48",
                "problemcode=52",
                "problemcode=39",
                "problem=48",
                "problem=52",
                "problem=39",
            ],
        ) =>
        {
            Err(format!(
                "USBIP-SETUP-016: Windows driver policy or signature verification blocked the USB/IP driver (exit code {code_num}){suffix}"
            ))
        }
        _ if contains_any(
            &lower,
            &[
                "error_success_reboot_required",
                "reboot is required",
                "restart is required",
                "restart required",
                "reboot required",
                "pending reboot",
                "pending restart",
                "marked for deletion",
                "cm_prob_need_restart",
                "problemcode=14",
                "problem=14",
                "code 14",
                "0xbc2",
                "0x430",
                "error 1072",
                "error 3010",
            ],
        ) =>
        {
            Err(format!(
                "USBIP-SETUP-015: a Windows restart is pending before USB/IP can finish installing (exit code {code_num}){suffix}"
            ))
        }
        _ => Err(format!(
            "USBIP-SETUP-010: the USB/IP installer failed with exit code {code_num}{suffix}"
        )),
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_spawn_error(os_error: Option<i32>, error_text: &str) -> String {
    let lower = error_text.to_ascii_lowercase();
    if matches!(os_error, Some(4551 | 5))
        || lower.contains("smart app control")
        || lower.contains("application control")
    {
        format!("USBIP-SETUP-017: Windows security policy blocked USB/IP setup: {error_text}")
    } else {
        format!("USBIP-SETUP-010: USB/IP setup did not start: {error_text}")
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn install_error_has_pending_restart(error: &str) -> bool {
    error.starts_with("USBIP-SETUP-015:")
}

/// What `usbip-package.ps1 -Action replace` answers. 107 is the host refusing before it touched anything; 102 and 103
/// come from the root-hub guard's rollback. Everything else is a failure whose real state the panel reads afterwards.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn classify_replace_exit_code(exit_code: Option<i32>) -> Result<InstallResult, String> {
    match exit_code {
        Some(0) => Ok(InstallResult::Ready),
        Some(3010) => Ok(InstallResult::RebootRecommended),
        Some(107) => Err("USBIP-REPLACE-001: this USB/IP installation cannot be replaced from here; nothing was changed".to_string()),
        Some(102) => Err("USBIP-REPLACE-002: the replacement was rolled back and the USB root hubs are healthy".to_string()),
        Some(103) => Err("USBIP-REPLACE-003: the USB root hubs were not fully recovered after the replacement".to_string()),
        code => Err(format!(
            "USBIP-REPLACE-004: the replacement failed with exit code {}",
            code.unwrap_or(-1)
        )),
    }
}

/// What the host's `status` answer says: `Some(true)` for an external installation the person may ask to replace, `Some(false)`
/// for a well-formed answer that says otherwise, `None` for output that is not an answer (empty, not JSON).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn parse_host_answer(output: &str) -> Option<bool> {
    let value =
        serde_json::from_str::<serde_json::Value>(output.trim().trim_start_matches('\u{feff}')).ok()?;
    value["ownership"].as_str()?;
    Some(value["ownership"].as_str() == Some("external") && value["replaceable"].as_bool() == Some(true))
}

/// True only on a positive, well-formed answer: the button appears on nothing else.
#[cfg(test)]
pub(crate) fn parse_host_replaceable(output: &str) -> bool {
    parse_host_answer(output).unwrap_or(false)
}

/// Directory next to the Sunshine installation where the installer is staged.
#[cfg(target_os = "windows")]
pub(crate) fn staging_root() -> std::path::PathBuf {
    crate::sunshine::install_dir()
        .join("tools")
        .join("usbip-staging")
}

/// Refuses to touch the transport while a stream may be using it.
#[cfg(target_os = "windows")]
pub(crate) async fn ensure_no_active_session() -> Result<(), String> {
    let sessions = crate::sunshine::get_active_sessions()
        .await
        .map_err(|error| format!("USBIP-SETUP-006: unable to verify stream ownership: {error}"))?;
    if sessions.is_empty() {
        Ok(())
    } else {
        Err(
            "USBIP-SETUP-006: finish the active Sunshine stream before changing the transport"
                .to_string(),
        )
    }
}

/// Reuses a healthy pinned transport or installs the pinned release.
/// An existing incompatible installation needs the shared cleanup first.
#[cfg(target_os = "windows")]
pub(crate) async fn ensure_pinned_usbip(
    _staging_root: &std::path::Path,
) -> Result<InstallResult, String> {
    use super::InstallDisposition;

    match super::install_disposition(super::installed_usbip_version()?.as_deref())? {
        InstallDisposition::Ready => {
            let status = super::usbip_get_status().await?;
            if !status.ready {
                return Err(format!(
                    "USBIP-SETUP-008: existing USB/IP transport needs repair before reuse: {}",
                    status.detail
                ));
            }
            return Ok(InstallResult::Ready);
        }
        InstallDisposition::Install => {}
    }

    run_host_package("install").await
}

#[cfg(target_os = "windows")]
static REPLACEABLE: super::memo::Memo = super::memo::Memo::new("usbip.replaceable");

/// Drops the kept answer (the installation is about to change).
pub(super) fn forget_replaceable() {
    #[cfg(target_os = "windows")]
    REPLACEABLE.forget();
}

/// Whether the host's lifecycle helper reports the installed USB/IP as one the person may replace. The ownership
/// rules live only in that helper, so the panel asks it instead of keeping a third copy. Asking starts PowerShell
/// (seconds), so the answer is kept for as long as the registered installations and the helper itself are unchanged
/// (memo.rs); the process runs below normal priority.
#[cfg(target_os = "windows")]
pub(super) async fn host_replaceable() -> bool {
    let script = crate::sunshine::install_dir()
        .join("scripts")
        .join("usbip-package.ps1");
    if !script.is_file() {
        return false;
    }
    let fingerprint = format!(
        "{}|{}",
        super::manager::ownership_fingerprint(),
        super::memo::file_identity(&script)
    );
    REPLACEABLE
        .get(&fingerprint, || async {
            probe_host_replaceable(&script).await.map(|yes| if yes { "1" } else { "0" }.to_string())
        })
        .await
        .is_some_and(|answer| answer == "1")
}

/// The helper's answer, asked now and never from a kept answer: the elevated copy revalidates a replacement with
/// it, and what the unelevated side stored must not be able to widen what the elevated side does.
#[cfg(target_os = "windows")]
pub(super) async fn host_replaceable_fresh() -> bool {
    let script = crate::sunshine::install_dir()
        .join("scripts")
        .join("usbip-package.ps1");
    script.is_file() && probe_host_replaceable(&script).await.unwrap_or(false)
}

/// `None` when the helper could not be asked (nothing is kept then).
#[cfg(target_os = "windows")]
async fn probe_host_replaceable(script: &std::path::Path) -> Option<bool> {
    let root = std::env::var_os("SystemRoot")?;
    let powershell =
        std::path::PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut command = tokio::process::Command::new(powershell);
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .args(["-Action", "status"])
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(super::memo::BACKGROUND_PROBE_FLAGS);
    let child = command.spawn().ok()?;
    match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
        Ok(Ok(output)) if output.status.success() => {
            parse_host_answer(&String::from_utf8_lossy(&output.stdout))
        }
        _ => None,
    }
}

/// Host-supplied lifecycle helper owns the global mutex through hub verification.
#[cfg(target_os = "windows")]
pub(super) async fn run_host_package(action: &str) -> Result<InstallResult, String> {
    // Use the host's immutable signed payload and its root-hub transaction.
    // No runtime installer or vendor setup executable is downloaded by the panel.
    let script = crate::sunshine::install_dir()
        .join("scripts")
        .join("usbip-package.ps1");
    if !script.is_file() {
        return Err(
            "USBIP-SETUP-002: update the host to install the bundled USB/IP component".into(),
        );
    }
    // `replace` takes over an installation the person confirmed; it needs the same verified payload as an install.
    if action == "install" || action == "replace" {
        super::manager::verify_slim_payload(
            &crate::sunshine::install_dir().join("scripts").join("usbip"),
        )?;
    }
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot")
            .ok_or("USBIP-SETUP-002: Windows directory is unavailable")?,
    )
    .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut command = tokio::process::Command::new(powershell);
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .args(["-Action", action])
        // panel-usbip-tokio-20261117, kill_on_drop: closing the UI must not
        // interrupt the child guard between driver mutation and hub recovery.
        .kill_on_drop(false)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
    let mut child = command
        .spawn()
        .map_err(|error| classify_spawn_error(error.raw_os_error(), &error.to_string()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let collect = async {
        let out_fut = async {
            match stdout {
                Some(pipe) => {
                    super::exec::read_bounded_output(pipe, super::MAX_OUTPUT_BYTES / 2)
                        .await
                        .unwrap_or_default()
                }
                None => Vec::new(),
            }
        };
        let err_fut = async {
            match stderr {
                Some(pipe) => {
                    super::exec::read_bounded_output(pipe, super::MAX_OUTPUT_BYTES / 2)
                        .await
                        .unwrap_or_default()
                }
                None => Vec::new(),
            }
        };
        let (out_bytes, err_bytes, status) = tokio::join!(out_fut, err_fut, child.wait());
        (out_bytes, err_bytes, status)
    };
    let (out_bytes, err_bytes, status_result) =
        tokio::time::timeout(std::time::Duration::from_secs(600), collect)
            .await
            .map_err(|_| "USBIP-SETUP-010: USB/IP setup timed out".to_string())?;
    let status =
        status_result.map_err(|error| format!("USBIP-SETUP-010: USB/IP setup failed: {error}"))?;
    let result = if action == "replace" {
        classify_replace_exit_code(status.code())?
    } else if matches!(status.code(), Some(0 | 3010)) {
        classify_installer_outcome(status.code(), "", "")?
    } else {
        let combined_output = [
            crate::text_encoding::decode_console_bytes(&err_bytes),
            crate::text_encoding::decode_console_bytes(&out_bytes),
            read_steps_summary(),
        ]
        .into_iter()
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
        let setupapi_tail = super::diagnostics::read_setupapi_usbip_tail(30)
            .map(|lines| lines.join("\n"))
            .unwrap_or_default();
        classify_installer_outcome(status.code(), &combined_output, &setupapi_tail)?
    };
    if action == "install" || action == "replace" {
        super::manager::find_installation()?;
        super::manager::verify_driver_compatibility()?;
        if action == "replace"
            && super::usbip_uninstall_entries()?
                .iter()
                .any(|entry| !entry.slim)
        {
            return Err(
                "USBIP-REPLACE-004: the old USB/IP registration remains after the replacement"
                    .into(),
            );
        }
    } else if !super::usbip_uninstall_entries()?.is_empty() {
        return Err("USBIP-CLEAN-001: USB/IP registration remains after guarded cleanup".into());
    }
    Ok(result)
}

#[cfg(target_os = "windows")]
fn read_steps_summary() -> String {
    let mut parts = Vec::new();
    let candidates = [
        crate::sunshine::install_dir()
            .join("scripts")
            .join("result-usbip-steps.txt"),
        std::env::var_os("ProgramData")
            .map(|dir| std::path::PathBuf::from(dir).join("AlkaidLab").join("USBip").join("result-usbip-steps.txt"))
            .unwrap_or_default(),
    ];
    for path in candidates {
        if !path.as_os_str().is_empty()
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.replace('\n', "; "));
            }
        }
    }
    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_exit_codes_map_to_ready_or_reboot() {
        assert_eq!(
            classify_installer_exit_code(Some(0)),
            Ok(InstallResult::Ready)
        );
        assert_eq!(
            classify_installer_exit_code(Some(3010)),
            Ok(InstallResult::RebootRecommended)
        );
    }

    #[test]
    fn installer_failures_classify_guard_policy_sac_and_reboot_causes() {
        assert!(
            classify_installer_outcome(Some(102), "", "")
                .unwrap_err()
                .starts_with("USBIP-SETUP-011:")
        );
        assert!(
            classify_installer_outcome(Some(103), "", "")
                .unwrap_err()
                .starts_with("USBIP-SETUP-012:")
        );
        assert!(
            classify_installer_outcome(Some(104), "", "")
                .unwrap_err()
                .starts_with("USBIP-SETUP-013:")
        );
        assert!(
            classify_installer_outcome(Some(105), "", "")
                .unwrap_err()
                .starts_with("USBIP-SETUP-014:")
        );
        assert!(
            classify_installer_outcome(Some(106), "", "")
                .unwrap_err()
                .starts_with("USBIP-SETUP-008:")
        );
        let pending = classify_installer_outcome(
            Some(1),
            "pnputil exited 3010",
            "!!!  dvi: CM_PROB_NEED_RESTART (ProblemCode=14)",
        )
        .unwrap_err();
        assert!(pending.starts_with("USBIP-SETUP-015:"), "{pending}");
        assert!(install_error_has_pending_restart(&pending));

        let blocked = classify_installer_outcome(
            Some(1),
            "pnputil /add-driver failed",
            "!!!  dvi: Error 0xe0000247: A problem was encountered (Code 48)",
        )
        .unwrap_err();
        assert!(blocked.starts_with("USBIP-SETUP-016:"), "{blocked}");
        assert!(blocked.contains("0xe0000247"), "{blocked}");

        let sac = classify_installer_outcome(
            Some(1),
            "Smart App Control blocked devnode.exe (0x800711c7)",
            "",
        )
        .unwrap_err();
        assert!(sac.starts_with("USBIP-SETUP-017:"), "{sac}");

        let sac_spawn = classify_spawn_error(Some(4551), "Application Control blocked");
        assert!(sac_spawn.starts_with("USBIP-SETUP-017:"), "{sac_spawn}");

        let generic = classify_installer_outcome(
            Some(1),
            "pnputil failed with 259",
            "<<<  [Exit status: FAILURE(0xe000020b)]",
        )
        .unwrap_err();
        assert!(generic.starts_with("USBIP-SETUP-010:"), "{generic}");
        assert!(generic.contains("0xe000020b"), "{generic}");
        assert!(!install_error_has_pending_restart(&generic));
    }

    #[test]
    fn replace_exit_codes_map_to_results_the_person_can_act_on() {
        assert_eq!(
            classify_replace_exit_code(Some(0)),
            Ok(InstallResult::Ready)
        );
        assert_eq!(
            classify_replace_exit_code(Some(3010)),
            Ok(InstallResult::RebootRecommended)
        );
        for (code, expected) in [
            (Some(107), "USBIP-REPLACE-001:"),
            (Some(102), "USBIP-REPLACE-002:"),
            (Some(103), "USBIP-REPLACE-003:"),
            (Some(1), "USBIP-REPLACE-004:"),
            (Some(106), "USBIP-REPLACE-004:"),
            (None, "USBIP-REPLACE-004:"),
        ] {
            let error = classify_replace_exit_code(code).unwrap_err();
            assert!(error.starts_with(expected), "{code:?}: {error}");
        }
    }

    #[test]
    fn host_status_offers_the_replacement_only_on_a_positive_well_formed_answer() {
        let external = r#"{"ownership":"external","compatible":true,"reason":"gui-or-sdk","ready":true,"replaceable":true,"version":"0.9.8.1"}"#;
        assert!(parse_host_replaceable(external));
        // Windows PowerShell may prefix a byte order mark and end with a newline.
        assert!(parse_host_replaceable(&format!("\u{feff}{external}\r\n")));
        for refused in [
            r#"{"ownership":"external","replaceable":false}"#,
            r#"{"ownership":"managed","replaceable":true}"#,
            r#"{"ownership":"legacy-owned","replaceable":true}"#,
            r#"{"ownership":"external"}"#,
            r#"{"ownership":"external","replaceable":"true"}"#,
            "not json",
            "",
        ] {
            assert!(!parse_host_replaceable(refused), "{refused}");
        }
        // an answer that is not one (nothing, not JSON, no ownership) is kept by nobody: it is `None`, not a "no"
        for unreadable in ["", "not json", "{}", r#"{"replaceable":true}"#] {
            assert_eq!(parse_host_answer(unreadable), None, "{unreadable}");
        }
        assert_eq!(parse_host_answer(r#"{"ownership":"managed"}"#), Some(false));
        assert_eq!(parse_host_answer(external), Some(true));
    }

    #[test]
    fn installer_failure_carries_a_usbip_code() {
        for code in [Some(1), Some(-1), None] {
            let error = classify_installer_exit_code(code).unwrap_err();
            assert!(error.starts_with("USBIP-SETUP-010:"), "{error}");
        }
    }
}
