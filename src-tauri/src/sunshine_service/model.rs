//! Platform-free part of the Sunshine service control: the start types, the
//! allowlisted operations of the elevated helper and their argument parsing,
//! the helper exit codes, and the location rule for the elevated executable.
//! Everything here is pure so it is unit-tested on every platform.

use serde::{Deserialize, Serialize};

/// Name of the service the installer registers (`install-service.bat`).
pub(crate) const SERVICE_NAME: &str = "SunshineService";

/// The one verb that turns this executable into the elevated service helper.
pub(crate) const HELPER_VERB: &str = "--elevated-sunshine-service";

/// SCM start types the panel offers. Boot and system start (0 and 1) are
/// driver-only values and never valid for this service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StartType {
    /// SERVICE_AUTO_START without the delayed flag.
    Auto,
    /// SERVICE_AUTO_START with SERVICE_DELAYED_AUTO_START_INFO.fDelayedAutostart.
    DelayedAuto,
    /// SERVICE_DEMAND_START.
    Manual,
    /// SERVICE_DISABLED.
    Disabled,
}

pub(crate) const SERVICE_AUTO_START: u32 = 2;
pub(crate) const SERVICE_DEMAND_START: u32 = 3;
pub(crate) const SERVICE_DISABLED: u32 = 4;

impl StartType {
    pub(crate) const ALL: [StartType; 4] = [
        StartType::Auto,
        StartType::DelayedAuto,
        StartType::Manual,
        StartType::Disabled,
    ];

    /// Command-line token of the helper; the only spellings it accepts.
    pub(crate) fn token(self) -> &'static str {
        match self {
            StartType::Auto => "auto",
            StartType::DelayedAuto => "delayed-auto",
            StartType::Manual => "manual",
            StartType::Disabled => "disabled",
        }
    }

    /// Exact, case-sensitive match against [`StartType::token`].
    pub(crate) fn from_token(token: &str) -> Option<StartType> {
        StartType::ALL.into_iter().find(|value| value.token() == token)
    }

    /// QUERY_SERVICE_CONFIGW.dwStartType plus the delayed flag of
    /// QueryServiceConfig2W(SERVICE_CONFIG_DELAYED_AUTO_START_INFO). The flag
    /// only counts for an auto-start service, as the SCM documents.
    pub(crate) fn from_scm(start_type: u32, delayed: bool) -> Result<StartType, String> {
        match start_type {
            SERVICE_AUTO_START if delayed => Ok(StartType::DelayedAuto),
            SERVICE_AUTO_START => Ok(StartType::Auto),
            SERVICE_DEMAND_START => Ok(StartType::Manual),
            SERVICE_DISABLED => Ok(StartType::Disabled),
            value => Err(format!(
                "SVC-SCM-003: unexpected start type {value} for {SERVICE_NAME}"
            )),
        }
    }

    /// dwStartType for ChangeServiceConfigW.
    pub(crate) fn scm_start_type(self) -> u32 {
        match self {
            StartType::Auto | StartType::DelayedAuto => SERVICE_AUTO_START,
            StartType::Manual => SERVICE_DEMAND_START,
            StartType::Disabled => SERVICE_DISABLED,
        }
    }

    /// fDelayedAutostart for ChangeServiceConfig2W.
    pub(crate) fn delayed(self) -> bool {
        matches!(self, StartType::DelayedAuto)
    }

    /// Whether Windows starts the service by itself after a reboot.
    pub(crate) fn starts_at_boot(self) -> bool {
        matches!(self, StartType::Auto | StartType::DelayedAuto)
    }
}

/// The complete set of things the elevated helper may do. Anything else on
/// its command line is rejected before an SCM handle is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    SetStartType(StartType),
    Start,
    Stop,
    /// Stop the service, end any sunshine.exe of this install that is left
    /// (a user-mode one or an orphan), start the service: also the way from
    /// user mode back to the service.
    Restart,
    /// Stop the service, end any sunshine.exe of this install that is left,
    /// start sunshine.exe of this install with administrator rights.
    UserMode,
}

impl Operation {
    /// Arguments after [`HELPER_VERB`]; the inverse of [`parse_helper_arguments`].
    pub(crate) fn arguments(self) -> Vec<&'static str> {
        match self {
            Operation::SetStartType(start_type) => vec!["set-start-type", start_type.token()],
            Operation::Start => vec!["start"],
            Operation::Stop => vec!["stop"],
            Operation::Restart => vec!["restart"],
            Operation::UserMode => vec!["user-mode"],
        }
    }

}

/// Parse the helper command line (`argv[1..]`). `None`: not a helper
/// invocation. `Some(Err)`: the helper verb with arguments outside the
/// allowlist — the process must exit without touching the SCM.
pub(crate) fn parse_helper_arguments<S: AsRef<str>>(
    arguments: &[S],
) -> Option<Result<Operation, String>> {
    let (verb, rest) = arguments.split_first()?;
    if verb.as_ref() != HELPER_VERB {
        return None;
    }
    let rest: Vec<&str> = rest.iter().map(AsRef::as_ref).collect();
    let operation = match rest.as_slice() {
        ["set-start-type", value] => StartType::from_token(value)
            .map(Operation::SetStartType)
            .ok_or_else(|| format!("SVC-ARG-002: start type '{value}' is not allowed")),
        ["start"] => Ok(Operation::Start),
        ["stop"] => Ok(Operation::Stop),
        ["restart"] => Ok(Operation::Restart),
        ["user-mode"] => Ok(Operation::UserMode),
        _ => Err("SVC-ARG-001: unsupported service helper arguments".to_string()),
    };
    Some(operation)
}

/// Manual and Disabled leave the host unreachable after the next reboot; the
/// caller must have shown that and received consent.
pub(crate) fn require_reboot_acknowledgement(
    target: StartType,
    acknowledged: bool,
) -> Result<(), String> {
    if target.starts_at_boot() || acknowledged {
        Ok(())
    } else {
        Err(
            "SVC-CONFIRM-001: Manual or Disabled needs confirmation that the host will not be reachable after a reboot"
                .to_string(),
        )
    }
}

/// Exit codes of the elevated helper. Win32 errors from the SCM are carried as
/// `WIN32_BASE | code` so they never collide with the helper's own codes.
pub(crate) mod exit {
    pub(crate) const OK: i32 = 0;
    pub(crate) const BAD_ARGUMENTS: i32 = 2;
    pub(crate) const NOT_ELEVATED: i32 = 3;
    pub(crate) const FOREIGN_SERVICE: i32 = 4;
    pub(crate) const UNTRUSTED_LOCATION: i32 = 5;
    pub(crate) const TIMEOUT: i32 = 6;
    pub(crate) const RUNTIME: i32 = 7;
    pub(crate) const PROCESS_START: i32 = 8;
    pub(crate) const WIN32_BASE: i32 = 0x2000_0000;

    pub(crate) fn from_win32(code: u32) -> i32 {
        WIN32_BASE | (code & 0xFFFF) as i32
    }
}

/// Turn a helper exit code into the result the UI shows.
pub(crate) fn describe_helper_exit(code: i32) -> Result<(), String> {
    match code {
        exit::OK => Ok(()),
        exit::BAD_ARGUMENTS => Err("SVC-ARG-001: the service helper rejected its arguments".into()),
        exit::NOT_ELEVATED => Err("SVC-UAC-001: administrator authorization was not granted".into()),
        exit::FOREIGN_SERVICE => Err(format!(
            "SVC-SCM-004: {SERVICE_NAME} belongs to another Sunshine installation"
        )),
        exit::UNTRUSTED_LOCATION => Err(
            "SVC-UAC-003: the panel is not running from its protected install folder".into(),
        ),
        exit::TIMEOUT => Err("SVC-SCM-005: the service did not reach the requested state in time".into()),
        exit::RUNTIME => Err("SVC-UAC-002: the service helper could not start".into()),
        exit::PROCESS_START => Err("SVC-PROC-003: sunshine.exe did not start in user mode".into()),
        code if code & exit::WIN32_BASE != 0 => {
            let win32 = (code & 0xFFFF) as u32;
            Err(format!("SVC-SCM-002: {}", win32_service_error(win32)))
        }
        // ShellExecuteEx returns 1223 (ERROR_CANCELLED) as an error, never as
        // an exit code; anything else is a crash or an unknown helper.
        code => Err(format!("SVC-UAC-002: the service helper exited with code {code}")),
    }
}

/// Wording of the SCM errors the service operations can return.
pub(crate) fn win32_service_error(code: u32) -> String {
    let text = match code {
        5 => "access denied",
        1051 => "a dependent service is running",
        1052 => "the service does not accept this control",
        1053 => "the service did not respond in time",
        1056 => "the service is already running",
        1058 => "the service is disabled",
        1060 => "the service is not installed",
        1061 => "the service cannot accept control messages now",
        1062 => "the service is not running",
        1069 => "the service did not start because of a logon failure",
        1072 => "the service is marked for deletion",
        _ => "",
    };
    if text.is_empty() {
        format!("Windows error {code}")
    } else {
        format!("{text} (Windows error {code})")
    }
}

/// Split a Windows path into components without a Windows `Path`, so the
/// location rule is testable on every platform.
fn windows_components(path: &str) -> Vec<String> {
    path.trim()
        .trim_matches('"')
        .trim_start_matches(r"\\?\")
        .split(['\\', '/'])
        .filter(|part| !part.is_empty() && *part != ".")
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

/// Install root of a panel executable at `<root>\assets\gui\<exe>`.
pub(crate) fn install_root_of_panel(panel_executable: &str) -> Option<Vec<String>> {
    let parts = windows_components(panel_executable);
    if parts.len() < 4 || parts.iter().any(|part| part == "..") {
        return None;
    }
    let (root, tail) = parts.split_at(parts.len() - 3);
    (tail[0] == "assets" && tail[1] == "gui" && tail[2].ends_with(".exe")).then(|| root.to_vec())
}

/// Install root of the service binary `<root>\tools\sunshinesvc.exe` from the
/// SCM binary path (which may be quoted and carry arguments).
pub(crate) fn install_root_of_service(binary_path: &str) -> Option<Vec<String>> {
    let text = binary_path.trim();
    let executable = if let Some(rest) = text.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        let lower = text.to_ascii_lowercase();
        let end = lower.find(".exe").map(|index| index + 4)?;
        &text[..end]
    };
    let parts = windows_components(executable);
    if parts.len() < 3 || parts.iter().any(|part| part == "..") {
        return None;
    }
    let (root, tail) = parts.split_at(parts.len() - 2);
    (tail[0] == "tools" && tail[1] == "sunshinesvc.exe").then(|| root.to_vec())
}

/// The elevated helper is this panel executable, and only when it sits in the
/// same install folder as the registered service binary. A panel elsewhere
/// (a portable copy, a download folder, a dev build) never elevates itself to
/// change a service another install registered.
pub(crate) fn panel_belongs_to_service(panel_executable: &str, service_binary_path: &str) -> bool {
    match (
        install_root_of_panel(panel_executable),
        install_root_of_service(service_binary_path),
    ) {
        (Some(panel_root), Some(service_root)) => panel_root == service_root,
        _ => false,
    }
}

/// Whether the registered service binary is `<install_dir>\tools\sunshinesvc.exe`.
pub(crate) fn service_binary_matches_install(binary_path: &str, install_dir: &str) -> bool {
    match (
        install_root_of_service(binary_path),
        Some(windows_components(install_dir)).filter(|root| !root.is_empty() && !root.iter().any(|part| part == "..")),
    ) {
        (Some(service_root), Some(install_root)) => service_root == install_root,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn helper_arguments_round_trip_for_every_operation() {
        let mut operations: Vec<Operation> =
            StartType::ALL.into_iter().map(Operation::SetStartType).collect();
        operations.extend([Operation::Start, Operation::Stop, Operation::Restart, Operation::UserMode]);
        for operation in operations {
            let mut line = vec![HELPER_VERB];
            line.extend(operation.arguments());
            assert_eq!(parse_helper_arguments(&line), Some(Ok(operation)));
        }
    }

    #[test]
    fn other_command_lines_are_not_helper_invocations() {
        assert_eq!(parse_helper_arguments::<&str>(&[]), None);
        assert_eq!(parse_helper_arguments(&["--hidden"]), None);
        assert_eq!(parse_helper_arguments(&["--elevated-usbip", "start"]), None);
        // The verb must be the first argument.
        assert_eq!(parse_helper_arguments(&["--hidden", HELPER_VERB, "start"]), None);
    }

    #[test]
    fn helper_rejects_everything_outside_the_allowlist() {
        let rejected: &[&[&str]] = &[
            &[HELPER_VERB],
            &[HELPER_VERB, "set-start-type"],
            &[HELPER_VERB, "set-start-type", "Auto"],
            &[HELPER_VERB, "set-start-type", "boot"],
            &[HELPER_VERB, "set-start-type", "system"],
            &[HELPER_VERB, "set-start-type", "demand"],
            &[HELPER_VERB, "set-start-type", "auto "],
            &[HELPER_VERB, "set-start-type", "auto", "extra"],
            &[HELPER_VERB, "set-start-type", "auto&calc"],
            &[HELPER_VERB, "start", "now"],
            &[HELPER_VERB, "pause"],
            &[HELPER_VERB, "delete"],
            &[HELPER_VERB, "config", "binPath=", r"C:\evil.exe"],
            &[HELPER_VERB, "START"],
            &[HELPER_VERB, "user-mode", "--port", "1"],
            &[HELPER_VERB, "user-mode", r"C:\evil.exe"],
            &[HELPER_VERB, "usermode"],
            &[HELPER_VERB, ""],
        ];
        for line in rejected {
            assert!(
                matches!(parse_helper_arguments(&args(line)), Some(Err(_))),
                "accepted {line:?}"
            );
        }
    }

    #[test]
    fn start_types_map_to_and_from_the_scm() {
        for start_type in StartType::ALL {
            assert_eq!(
                StartType::from_scm(start_type.scm_start_type(), start_type.delayed()),
                Ok(start_type)
            );
        }
        // The delayed flag is ignored unless the service is auto-start.
        assert_eq!(StartType::from_scm(3, true), Ok(StartType::Manual));
        assert_eq!(StartType::from_scm(4, true), Ok(StartType::Disabled));
        assert!(StartType::from_scm(0, false).is_err());
        assert!(StartType::from_scm(1, false).is_err());
        assert!(StartType::from_scm(5, false).is_err());
    }

    #[test]
    fn start_types_serialize_as_their_tokens() {
        for start_type in StartType::ALL {
            assert_eq!(
                serde_json::to_string(&start_type).unwrap(),
                format!("\"{}\"", start_type.token())
            );
        }
        assert!(serde_json::from_str::<StartType>("\"boot\"").is_err());
    }

    #[test]
    fn manual_and_disabled_need_the_reboot_acknowledgement() {
        assert!(require_reboot_acknowledgement(StartType::Auto, false).is_ok());
        assert!(require_reboot_acknowledgement(StartType::DelayedAuto, false).is_ok());
        assert!(require_reboot_acknowledgement(StartType::Manual, false).is_err());
        assert!(require_reboot_acknowledgement(StartType::Disabled, false).is_err());
        assert!(require_reboot_acknowledgement(StartType::Manual, true).is_ok());
        assert!(require_reboot_acknowledgement(StartType::Disabled, true).is_ok());
    }

    #[test]
    fn helper_exit_codes_describe_their_cause() {
        assert!(describe_helper_exit(exit::OK).is_ok());
        let access = describe_helper_exit(exit::from_win32(5)).unwrap_err();
        assert!(access.starts_with("SVC-SCM-002") && access.contains("access denied"), "{access}");
        let missing = describe_helper_exit(exit::from_win32(1060)).unwrap_err();
        assert!(missing.contains("not installed"), "{missing}");
        assert!(describe_helper_exit(exit::BAD_ARGUMENTS).unwrap_err().starts_with("SVC-ARG"));
        assert!(describe_helper_exit(exit::UNTRUSTED_LOCATION).unwrap_err().starts_with("SVC-UAC-003"));
        assert!(describe_helper_exit(1).unwrap_err().starts_with("SVC-UAC-002"));
        assert!(describe_helper_exit(exit::PROCESS_START).unwrap_err().starts_with("SVC-PROC-003"));
    }

    #[test]
    fn the_panel_must_sit_in_the_install_of_the_service() {
        let service = r#""C:\Program Files\Sunshine\tools\sunshinesvc.exe""#;
        assert!(panel_belongs_to_service(
            r"C:\Program Files\Sunshine\assets\gui\sunshine-gui.exe",
            service
        ));
        assert!(panel_belongs_to_service(
            r"\\?\C:\PROGRAM FILES\sunshine\assets\GUI\Sunshine-GUI.exe",
            service
        ));
        assert!(panel_belongs_to_service(
            r"D:\Apps\Sunshine\assets\gui\sunshine-gui.exe",
            r"D:\Apps\Sunshine\tools\sunshinesvc.exe --flag"
        ));
        // another install, a download folder, a dev build, traversal
        assert!(!panel_belongs_to_service(
            r"C:\Users\me\Downloads\Sunshine\assets\gui\sunshine-gui.exe",
            service
        ));
        assert!(!panel_belongs_to_service(r"C:\Users\me\AppData\Local\Temp\sunshine-gui.exe", service));
        assert!(!panel_belongs_to_service(
            r"C:\src\panel\src-tauri\target\release\sunshine-gui.exe",
            service
        ));
        assert!(!panel_belongs_to_service(
            r"C:\Program Files\Sunshine\assets\gui\..\..\..\Temp\assets\gui\x.exe",
            service
        ));
        assert!(!panel_belongs_to_service(
            r"C:\Program Files\Sunshine\assets\gui\sunshine-gui.exe",
            r"C:\Program Files\Sunshine\sunshine.exe"
        ));
        assert!(!panel_belongs_to_service(r"C:\Program Files\Sunshine\assets\gui\sunshine-gui.exe", ""));
    }

    #[test]
    fn install_dir_matching_uses_the_service_binary() {
        assert!(service_binary_matches_install(
            r#""C:\Portable Sunshine\tools\sunshinesvc.exe""#,
            r"C:\Portable Sunshine"
        ));
        assert!(service_binary_matches_install(
            r#""C:\Portable Sunshine\tools\sunshinesvc.exe""#,
            r"C:\Portable Sunshine\"
        ));
        assert!(!service_binary_matches_install(
            r#""C:\Program Files\Sunshine\tools\sunshinesvc.exe""#,
            r"C:\Portable Sunshine"
        ));
        assert!(!service_binary_matches_install(r#""C:\S\tools\sunshinesvc.exe""#, ""));
    }
}
