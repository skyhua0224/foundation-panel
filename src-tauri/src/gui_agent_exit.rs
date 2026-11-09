/// Exit code the Windows host and the service treat as a deliberate quit.
///
/// Must match sunshinesvc::GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE in the host
/// (0x5351). The tray menu Exit Sunshine command and the updater handoff use it.
/// Task Manager and the taskbar End task end the process with TerminateProcess
/// and never write this value, so the host starts the agent again.
/// GetExitCodeProcess returns the caller's exit code. 0 is also what a
/// single-instance duplicate and the elevated-restart handoff use, and those
/// keep the tray by attaching to the process that is still running.
pub const GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE: i32 = 0x5351;

#[cfg(test)]
mod tests {
    use super::GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE;

    #[test]
    fn deliberate_quit_is_not_a_zero_or_forced_end() {
        assert_eq!(GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE, 0x5351);
        assert_ne!(GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE, 0);
        assert_ne!(GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE, 1);
        assert_ne!(GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE, 259);
    }
}
