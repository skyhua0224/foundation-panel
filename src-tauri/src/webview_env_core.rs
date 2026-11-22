//! The pure part of webview_env.rs (no system, no logging): the switch string every WebView2 of the panel is started with, its idempotent composition, and the retry rules for a window that fails with the
//! invalid-state code. Kept apart so that it is tested with plain `rustc --test` as well as in the crate.

use std::time::Duration;

/// The switches the panel starts every WebView2 with (main.rs): loopback never goes through the user's proxy, and the interface sounds may play without a gesture.
pub const PANEL_SWITCHES: [&str; 2] = [
    // WebView2 uses Chromium's bypass syntax. Keep external requests on the user's configured proxy while forcing every loopback form to direct.
    "--proxy-bypass-list=localhost;*.localhost;127.0.0.0/8;[::1]",
    // UI sounds: gamepad input is not a "user gesture" and would not unlock an AudioContext; the app's own navigation sounds are not web autoplay media
    "--autoplay-policy=no-user-gesture-required",
];

/// Splits a switch string at the spaces that are not inside double quotes.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in text.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                current.push(ch);
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn switch_name(token: &str) -> &str {
    token.split('=').next().unwrap_or(token)
}

/// The value of `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` for a process that has the panel's switches: what the caller put in front (for example `--remote-debugging-port` to trace an instance) stays in
/// front, the panel's switches follow, and a switch is never there twice. Applying it to its own result changes nothing, so a process that inherits the value of the panel that started it ends up with
/// exactly the value that panel had.
pub fn merge_browser_arguments(inherited: Option<&str>, ours: &[&str]) -> String {
    let our_names: Vec<&str> = ours.iter().map(|token| switch_name(token)).collect();
    let mut kept: Vec<String> = Vec::new();
    for token in tokens(inherited.unwrap_or("")) {
        if our_names.contains(&switch_name(&token)) || kept.contains(&token) {
            continue;
        }
        kept.push(token);
    }
    kept.extend(ours.iter().map(|token| token.to_string()));
    kept.join(" ")
}

/// Whether the text of a window creation error is the "options differ from the running browser process" failure.
pub fn is_invalid_state(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("8007139f") || lower.contains("error_invalid_state") || lower.contains("not in the correct state")
}

/// Pauses between the attempts to make a window that failed with the invalid-state code (the browser process of a panel that just went away is still shutting down for a moment).
pub const RETRY_DELAYS: [Duration; 4] = [Duration::from_millis(250), Duration::from_millis(750), Duration::from_millis(2_000), Duration::from_millis(5_000)];

/// The pause before attempt `failed + 1`, or None when the attempts are used up.
pub fn retry_delay(failed: usize, error: &str) -> Option<Duration> {
    if !is_invalid_state(error) {
        return None;
    }
    RETRY_DELAYS.get(failed).copied()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_caller_value_stays_in_front_and_our_switches_follow() {
        let merged = merge_browser_arguments(Some("--remote-debugging-port=9222"), &PANEL_SWITCHES);
        assert_eq!(merged, format!("--remote-debugging-port=9222 {} {}", PANEL_SWITCHES[0], PANEL_SWITCHES[1]));
        assert_eq!(merge_browser_arguments(None, &PANEL_SWITCHES), format!("{} {}", PANEL_SWITCHES[0], PANEL_SWITCHES[1]));
        assert_eq!(merge_browser_arguments(Some("   "), &PANEL_SWITCHES), merge_browser_arguments(None, &PANEL_SWITCHES));
    }

    #[test]
    fn a_panel_started_by_a_panel_gets_the_value_its_parent_had() {
        // the parent's value is what the child inherits: applying the merge again must not change it
        let parent = merge_browser_arguments(Some("--remote-debugging-port=9222"), &PANEL_SWITCHES);
        let child = merge_browser_arguments(Some(&parent), &PANEL_SWITCHES);
        assert_eq!(child, parent, "the arguments of the child differed from the parent's: the options mismatch the running browser process");
        let grandchild = merge_browser_arguments(Some(&child), &PANEL_SWITCHES);
        assert_eq!(grandchild, parent);
        let bare_parent = merge_browser_arguments(None, &PANEL_SWITCHES);
        assert_eq!(merge_browser_arguments(Some(&bare_parent), &PANEL_SWITCHES), bare_parent);
    }

    #[test]
    fn the_old_doubled_value_is_cleaned_to_the_single_one() {
        let doubled = format!("{0} {1} {0} {1}", PANEL_SWITCHES[0], PANEL_SWITCHES[1]);
        assert_eq!(merge_browser_arguments(Some(&doubled), &PANEL_SWITCHES), merge_browser_arguments(None, &PANEL_SWITCHES));
    }

    #[test]
    fn our_switch_replaces_the_same_switch_with_another_value() {
        let merged = merge_browser_arguments(Some("--autoplay-policy=user-gesture-required --foo"), &PANEL_SWITCHES);
        assert_eq!(merged, format!("--foo {} {}", PANEL_SWITCHES[0], PANEL_SWITCHES[1]));
    }

    #[test]
    fn quoted_values_with_spaces_stay_whole_and_are_not_repeated() {
        let merged = merge_browser_arguments(Some("--a=\"x y\" --a=\"x y\""), &PANEL_SWITCHES);
        assert!(merged.starts_with("--a=\"x y\" --proxy-bypass-list"));
        assert_eq!(merged.matches("--a=").count(), 1);
    }

    #[test]
    fn the_invalid_state_code_is_told_from_other_failures() {
        assert!(is_invalid_state("failed to create webview: WebView2 error: WindowsError(Error { code: HRESULT(0x8007139F), message: \"The group or resource is not in the correct state to perform the requested operation.\" })"));
        assert!(is_invalid_state("0x8007139f"));
        assert!(is_invalid_state("HRESULT_FROM_WIN32(ERROR_INVALID_STATE)"));
        assert!(!is_invalid_state("failed to create webview: the window was destroyed (E_ABORT 0x80004004)"));
        assert!(!is_invalid_state(""));
    }

    #[test]
    fn only_the_invalid_state_code_is_retried_and_only_a_few_times() {
        let error = "HRESULT(0x8007139F)";
        assert_eq!(retry_delay(0, error), Some(Duration::from_millis(250)));
        assert_eq!(retry_delay(3, error), Some(Duration::from_millis(5_000)));
        assert_eq!(retry_delay(4, error), None, "the attempts are used up");
        assert_eq!(retry_delay(0, "E_ABORT"), None, "another error is not waited out");
    }
}
