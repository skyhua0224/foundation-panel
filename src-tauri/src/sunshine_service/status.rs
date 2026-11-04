//! The three layers of "is Sunshine running", each reported on its own:
//! the SCM record of SunshineService, the sunshine.exe process, and whether
//! the host API answers. Pure mapping, unit-tested on every platform.

use super::model::StartType;
use serde::Serialize;

/// SERVICE_STATUS_PROCESS.dwCurrentState.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScmState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

impl ScmState {
    pub(crate) fn from_scm(state: u32) -> ScmState {
        match state {
            1 => ScmState::Stopped,
            2 => ScmState::StartPending,
            3 => ScmState::StopPending,
            4 => ScmState::Running,
            5 => ScmState::ContinuePending,
            6 => ScmState::PausePending,
            7 => ScmState::Paused,
            _ => ScmState::Unknown,
        }
    }
}

/// SC_ACTION.Type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryKind {
    None,
    Restart,
    Reboot,
    RunCommand,
    Unknown,
}

impl RecoveryKind {
    pub(crate) fn from_scm(kind: i32) -> RecoveryKind {
        match kind {
            0 => RecoveryKind::None,
            1 => RecoveryKind::Restart,
            2 => RecoveryKind::Reboot,
            3 => RecoveryKind::RunCommand,
            _ => RecoveryKind::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAction {
    pub kind: RecoveryKind,
    pub delay_ms: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// Failures 1, 2 and 3+ (the last entry repeats), as SERVICE_FAILURE_ACTIONSW lists them.
    pub actions: Vec<RecoveryAction>,
    /// dwResetPeriod: seconds without failure after which the count resets.
    pub reset_period_s: u32,
    /// The SCM restarts the service after its first failure.
    pub restarts_on_failure: bool,
}

impl Recovery {
    pub(crate) fn new(actions: Vec<RecoveryAction>, reset_period_s: u32) -> Recovery {
        let restarts_on_failure = actions.first().is_some_and(|action| action.kind == RecoveryKind::Restart);
        Recovery { actions, reset_period_s, restarts_on_failure }
    }
}

const ERROR_SERVICE_SPECIFIC_ERROR: u32 = 1066;

/// The exit code that explains the last stop: dwServiceSpecificExitCode when
/// dwWin32ExitCode is ERROR_SERVICE_SPECIFIC_ERROR, otherwise dwWin32ExitCode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitCode {
    pub code: u32,
    pub service_specific: bool,
}

pub(crate) fn last_exit(win32_exit_code: u32, service_specific_exit_code: u32) -> Option<ExitCode> {
    match win32_exit_code {
        0 => None,
        ERROR_SERVICE_SPECIFIC_ERROR => Some(ExitCode { code: service_specific_exit_code, service_specific: true }),
        code => Some(ExitCode { code, service_specific: false }),
    }
}

/// SCM layer. `installed = false`: there is no SunshineService.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScmLayer {
    pub installed: bool,
    /// The registered binary is this install's tools\sunshinesvc.exe.
    pub belongs_to_install: bool,
    pub state: Option<ScmState>,
    pub start_type: Option<StartType>,
    pub binary_path: String,
    pub account: String,
    pub last_exit: Option<ExitCode>,
    /// PID of the service process (sunshinesvc.exe, the wrapper).
    pub service_pid: Option<u32>,
    pub recovery: Option<Recovery>,
    pub error: Option<String>,
}

/// Process layer: sunshine.exe of this install.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessLayer {
    pub running: bool,
    pub pid: Option<u32>,
    /// Creation time (GetProcessTimes), Unix milliseconds.
    pub started_at_ms: Option<u64>,
    /// Its parent is the service process (the service launched it).
    pub child_of_service: bool,
    pub error: Option<String>,
}

/// API layer: an HTTP answer from the host web server (any status below
/// 500 counts, 401 included: the server is up).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLayer {
    pub answers: bool,
    pub http_status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub(crate) fn api_answers(http_status: u16) -> bool {
    http_status < 500
}

/// One word for the whole picture; the page still shows each layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// All three layers are green.
    Running,
    /// The SCM is starting or stopping the service.
    Starting,
    Stopping,
    /// The service is stopped and no sunshine.exe runs.
    Stopped,
    /// The service runs but sunshine.exe is gone (the wrapper reports RUNNING on its own).
    ProcessMissing,
    /// sunshine.exe runs but the API does not answer (starting up, hung or a port conflict).
    ApiSilent,
    /// No service (or it is stopped) and sunshine.exe runs on its own.
    UserMode,
    /// No SunshineService is registered.
    NotInstalled,
    /// SunshineService is registered by another install.
    ForeignService,
}

/// Which layer is the first to explain a non-running verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layer {
    Scm,
    Process,
    Api,
}

pub(crate) fn summarize(scm: &ScmLayer, process: &ProcessLayer, api: &ApiLayer) -> (Verdict, Option<Layer>) {
    let service_running = scm.installed && scm.belongs_to_install && scm.state == Some(ScmState::Running);
    if !scm.installed || !scm.belongs_to_install {
        let fallback = if scm.installed { Verdict::ForeignService } else { Verdict::NotInstalled };
        return match (process.running, api.answers) {
            (true, true) => (Verdict::UserMode, None),
            (true, false) => (Verdict::ApiSilent, Some(Layer::Api)),
            (false, _) => (fallback, Some(Layer::Scm)),
        };
    }
    match scm.state {
        Some(ScmState::StartPending | ScmState::ContinuePending) => return (Verdict::Starting, Some(Layer::Scm)),
        Some(ScmState::StopPending | ScmState::PausePending) => return (Verdict::Stopping, Some(Layer::Scm)),
        _ => {}
    }
    match (service_running, process.running, api.answers) {
        (true, true, true) => (Verdict::Running, None),
        (true, false, _) => (Verdict::ProcessMissing, Some(Layer::Process)),
        (true, true, false) => (Verdict::ApiSilent, Some(Layer::Api)),
        (false, true, true) => (Verdict::UserMode, None),
        (false, true, false) => (Verdict::ApiSilent, Some(Layer::Api)),
        (false, false, _) => (Verdict::Stopped, Some(Layer::Scm)),
    }
}

/// Everything the service page shows.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub verdict: Verdict,
    pub failing_layer: Option<Layer>,
    pub scm: ScmLayer,
    pub process: ProcessLayer,
    pub api: ApiLayer,
    /// Live streams the host reports (`None`: could not ask).
    pub live_streams: Option<usize>,
}

/// Stop and restart end every live stream; they need the user's consent
/// when the host reports any.
pub(crate) fn require_live_stream_acknowledgement(
    operation: super::model::Operation,
    live_streams: Option<usize>,
    acknowledged: bool,
) -> Result<(), String> {
    use super::model::Operation;
    let interrupts = matches!(operation, Operation::Stop | Operation::Restart);
    match live_streams {
        Some(count) if interrupts && count > 0 && !acknowledged => Err(format!(
            "SVC-CONFIRM-002: {count} live stream(s) would end; confirm to continue"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Operation;
    use super::*;

    fn scm(state: ScmState) -> ScmLayer {
        ScmLayer { installed: true, belongs_to_install: true, state: Some(state), ..Default::default() }
    }
    fn process(running: bool) -> ProcessLayer {
        ProcessLayer { running, pid: running.then_some(42), ..Default::default() }
    }
    fn api(answers: bool) -> ApiLayer {
        ApiLayer { answers, ..Default::default() }
    }

    #[test]
    fn scm_states_map_from_dw_current_state() {
        let expected = [
            ScmState::Stopped,
            ScmState::StartPending,
            ScmState::StopPending,
            ScmState::Running,
            ScmState::ContinuePending,
            ScmState::PausePending,
            ScmState::Paused,
        ];
        for (index, state) in expected.into_iter().enumerate() {
            assert_eq!(ScmState::from_scm(index as u32 + 1), state);
        }
        assert_eq!(ScmState::from_scm(0), ScmState::Unknown);
        assert_eq!(ScmState::from_scm(8), ScmState::Unknown);
    }

    #[test]
    fn all_green_is_running() {
        assert_eq!(summarize(&scm(ScmState::Running), &process(true), &api(true)), (Verdict::Running, None));
    }

    #[test]
    fn each_red_layer_is_named() {
        assert_eq!(
            summarize(&scm(ScmState::Running), &process(false), &api(false)),
            (Verdict::ProcessMissing, Some(Layer::Process))
        );
        assert_eq!(
            summarize(&scm(ScmState::Running), &process(true), &api(false)),
            (Verdict::ApiSilent, Some(Layer::Api))
        );
        assert_eq!(
            summarize(&scm(ScmState::Stopped), &process(false), &api(false)),
            (Verdict::Stopped, Some(Layer::Scm))
        );
        assert_eq!(
            summarize(&scm(ScmState::StartPending), &process(false), &api(false)),
            (Verdict::Starting, Some(Layer::Scm))
        );
        assert_eq!(
            summarize(&scm(ScmState::StopPending), &process(true), &api(true)),
            (Verdict::Stopping, Some(Layer::Scm))
        );
    }

    #[test]
    fn a_process_without_the_service_is_user_mode() {
        assert_eq!(summarize(&scm(ScmState::Stopped), &process(true), &api(true)), (Verdict::UserMode, None));
        let none = ScmLayer::default();
        assert_eq!(summarize(&none, &process(true), &api(true)), (Verdict::UserMode, None));
        assert_eq!(summarize(&none, &process(false), &api(false)), (Verdict::NotInstalled, Some(Layer::Scm)));
        let foreign = ScmLayer { installed: true, belongs_to_install: false, state: Some(ScmState::Running), ..Default::default() };
        assert_eq!(summarize(&foreign, &process(false), &api(false)), (Verdict::ForeignService, Some(Layer::Scm)));
    }

    #[test]
    fn exit_codes_prefer_the_service_specific_code() {
        assert_eq!(last_exit(0, 7), None);
        assert_eq!(last_exit(1066, 7), Some(ExitCode { code: 7, service_specific: true }));
        assert_eq!(last_exit(1067, 0), Some(ExitCode { code: 1067, service_specific: false }));
    }

    #[test]
    fn recovery_actions_map_from_sc_action() {
        assert_eq!(RecoveryKind::from_scm(0), RecoveryKind::None);
        assert_eq!(RecoveryKind::from_scm(1), RecoveryKind::Restart);
        assert_eq!(RecoveryKind::from_scm(2), RecoveryKind::Reboot);
        assert_eq!(RecoveryKind::from_scm(3), RecoveryKind::RunCommand);
        assert_eq!(RecoveryKind::from_scm(4), RecoveryKind::Unknown);
        let restart = Recovery::new(vec![RecoveryAction { kind: RecoveryKind::Restart, delay_ms: 5000 }], 86400);
        assert!(restart.restarts_on_failure);
        assert!(!Recovery::new(Vec::new(), 0).restarts_on_failure);
        let reboot_first = Recovery::new(vec![RecoveryAction { kind: RecoveryKind::Reboot, delay_ms: 0 }], 0);
        assert!(!reboot_first.restarts_on_failure);
    }

    #[test]
    fn the_api_answers_below_500() {
        assert!(api_answers(200));
        assert!(api_answers(401));
        assert!(!api_answers(502));
        assert!(!api_answers(503));
    }

    #[test]
    fn stop_and_restart_need_consent_while_streaming() {
        assert!(require_live_stream_acknowledgement(Operation::Stop, Some(1), false).is_err());
        assert!(require_live_stream_acknowledgement(Operation::Restart, Some(2), false).is_err());
        assert!(require_live_stream_acknowledgement(Operation::Stop, Some(1), true).is_ok());
        assert!(require_live_stream_acknowledgement(Operation::Stop, Some(0), false).is_ok());
        assert!(require_live_stream_acknowledgement(Operation::Stop, None, false).is_ok());
        assert!(require_live_stream_acknowledgement(Operation::Start, Some(3), false).is_ok());
    }

    #[test]
    fn status_serializes_with_camel_case_keys() {
        let status = ServiceStatus {
            verdict: Verdict::Running,
            failing_layer: None,
            scm: ScmLayer { start_type: Some(StartType::DelayedAuto), state: Some(ScmState::Running), ..Default::default() },
            process: process(true),
            api: api(true),
            live_streams: Some(0),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["verdict"], "running");
        assert_eq!(json["scm"]["startType"], "delayed-auto");
        assert_eq!(json["scm"]["state"], "running");
        assert_eq!(json["process"]["pid"], 42);
        assert_eq!(json["liveStreams"], 0);
    }
}
