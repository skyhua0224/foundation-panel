//! The decisions of the memory-adaptive release (mem_adaptive.rs), without the system: which mode is asked for, how much memory pressure a sample of the machine means, and what each
//! mode does about the windows that are idle at that pressure. Pure, so that the thresholds are tested with numbers.
//!
//! Adaptive is the default on every machine (`Setting::Auto` picks the tier from the installed memory); `Off` and a fixed tier are settings the person (or a measurement) chooses.
//!
//! Physical memory and commit signals use Microsoft Learn `MEMORYSTATUSEX` and `PERFORMANCE_INFORMATION` (Members; `panel-memory-win32-physical` and `panel-memory-win32-commit`).
//! The system notification can independently indicate low memory (`CreateMemoryResourceNotification` and `QueryMemoryResourceNotification`; `panel-memory-win32-notification` and
//! `panel-memory-win32-notification-query`). The Panel's numerical thresholds and hysteresis are application policy. Inactive WebViews share the environment and lower their usage target
//! (`panel-memory-webview2-performance`, Memory usage and process management; `windows/lifecycle.rs`).

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// today's behaviour, nothing released
    Off,
    /// the windows stay warm while memory is plentiful; idle ones go when the machine is short of memory
    Adaptive,
    /// the preset for machines with less installed memory: the launch splash is created on demand
    Low,
}

impl Mode {
    pub fn parse(text: &str) -> Mode {
        match text.trim().to_ascii_lowercase().as_str() {
            "adaptive" | "auto" => Mode::Adaptive,
            "low" | "low-memory" | "lowmemory" => Mode::Low,
            _ => Mode::Off,
        }
    }

    /// The mode a preset is chosen for: the installed memory in bytes. At most 8 GB the low-memory preset is the default of a first start (the prototype only offers the number).
    pub fn suggested_for(total_phys: u64) -> Mode {
        if total_phys == 0 {
            Mode::Off
        } else if total_phys <= 8 * GB + GB / 2 {
            // 8 GB machines report 7.4 to 8.0 GB as installed memory once the firmware has taken its share
            Mode::Low
        } else {
            Mode::Adaptive
        }
    }
}

/// What the person (or a measurement) asked for: nothing (the controller picks), or a tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Auto,
    Fixed(Mode),
}

impl Setting {
    /// `off`, `adaptive` and `low` are tiers; `auto`, an empty text and anything unknown leave the choice to the controller (the safe default is the adaptive controller, not nothing).
    pub fn parse(text: &str) -> Setting {
        match text.trim().to_ascii_lowercase().as_str() {
            "off" | "0" | "false" | "none" | "never" => Setting::Fixed(Mode::Off),
            "adaptive" => Setting::Fixed(Mode::Adaptive),
            "low" | "low-memory" | "lowmemory" => Setting::Fixed(Mode::Low),
            _ => Setting::Auto,
        }
    }

    /// The tier this machine runs: the fixed one, or the preset for the installed memory (adaptive while the memory is not known).
    pub fn resolve(self, total_phys: u64) -> Mode {
        match self {
            Setting::Fixed(mode) => mode,
            Setting::Auto => match Mode::suggested_for(total_phys) {
                Mode::Off => Mode::Adaptive,
                mode => mode,
            },
        }
    }
}

const MB: u64 = 1 << 20;
const GB: u64 = 1 << 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pressure {
    None,
    Elevated,
    Critical,
}

/// One look at the machine. `commit_headroom` is `u64::MAX` when it is not known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub total_phys: u64,
    pub avail_phys: u64,
    pub commit_headroom: u64,
    /// `QueryMemoryResourceNotification(LowMemoryResourceNotification)` is signalled
    pub low_signal: bool,
}

/// Available physical memory below which the pressure is elevated: 15 % of the installed memory, never less than 1.5 GB (an 8 GB machine: 1.5 GB, 16 GB: 2.4 GB, 32 GB: 4.8 GB).
pub fn elevated_below(total_phys: u64) -> u64 {
    (total_phys / 100 * 15).max(GB + GB / 2)
}

/// Available physical memory below which it is critical: 6 % of the installed memory, never less than 512 MB.
pub fn critical_below(total_phys: u64) -> u64 {
    (total_phys / 100 * 6).max(512 * MB)
}

const COMMIT_ELEVATED: u64 = 2 * GB;
const COMMIT_CRITICAL: u64 = 768 * MB;
/// A level that is already held is left only when the machine is this much (percent) clear of its threshold.
const LEAVE_MARGIN_PERCENT: u64 = 125;

fn reaches(sample: &Sample, level: Pressure, percent: u64) -> bool {
    let (phys, commit) = match level {
        Pressure::None => return true,
        Pressure::Elevated => (elevated_below(sample.total_phys), COMMIT_ELEVATED),
        Pressure::Critical => (critical_below(sample.total_phys), COMMIT_CRITICAL),
    };
    sample.avail_phys < phys / 100 * percent
        || sample.commit_headroom < commit / 100 * percent
        || (level == Pressure::Critical && sample.low_signal)
}

/// The pressure a sample means, given the pressure before it: a level is entered at its threshold and left 25 % beyond it, so a machine that hovers at a threshold does not flap.
pub fn classify(sample: &Sample, before: Pressure) -> Pressure {
    let percent = |level: Pressure| if before >= level { LEAVE_MARGIN_PERCENT } else { 100 };
    if reaches(sample, Pressure::Critical, percent(Pressure::Critical)) {
        Pressure::Critical
    } else if reaches(sample, Pressure::Elevated, percent(Pressure::Elevated)) {
        Pressure::Elevated
    } else {
        Pressure::None
    }
}

/// The hidden main window may be released only under measured system pressure in an enabled tier.
pub fn may_release_hidden_main(mode: Mode, measured: Pressure) -> bool {
    mode != Mode::Off && measured != Pressure::None
}

/// Launch splash behavior for a selected tier and pressure. The Material tray menu always stays resident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplashPolicy {
    pub warm_splash: bool,
    /// The idle splash is destroyed after this long. `None` keeps it.
    pub splash_idle_release: Option<Duration>,
}

const fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

pub fn splash_policy(mode: Mode, pressure: Pressure) -> SplashPolicy {
    match (mode, pressure) {
        (Mode::Off, _) | (Mode::Adaptive, Pressure::None) => SplashPolicy { warm_splash: true, splash_idle_release: None },
        (Mode::Adaptive, Pressure::Elevated) | (Mode::Low, Pressure::None) => SplashPolicy { warm_splash: false, splash_idle_release: Some(secs(60)) },
        (Mode::Adaptive, Pressure::Critical) => SplashPolicy { warm_splash: false, splash_idle_release: Some(secs(15)) },
        (Mode::Low, Pressure::Elevated) => SplashPolicy { warm_splash: false, splash_idle_release: Some(secs(30)) },
        (Mode::Low, Pressure::Critical) => SplashPolicy { warm_splash: false, splash_idle_release: Some(secs(10)) },
    }
}

/// A positive `SUNSHINE_GUI_MEM_TRAY_IDLE_SECS` replaces the measurement splash delay.
pub fn override_secs(text: Option<&str>) -> Option<Duration> {
    let seconds: u64 = text?.trim().parse().ok()?;
    (seconds > 0).then(|| Duration::from_secs(seconds))
}

/// Parses the simulated pressure used by the splash measurement hook.
pub fn parse_fake_pressure(text: &str) -> Option<Pressure> {
    match text.trim().to_ascii_lowercase().as_str() {
        "none" | "ok" => Some(Pressure::None),
        "elevated" | "low" => Some(Pressure::Elevated),
        "critical" => Some(Pressure::Critical),
        _ => None,
    }
}

// ---- the guard ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

/// A window that comes back after a release must be as quick as one that was never released. The ledger keeps the times the person waited for a kept window (hidden -> shown) and for a released one
/// (asked -> shown) and says, per pressure level (the "situation" a release happens in), whether releasing is still allowed. A situation in which the released window was measurably slower is blocked
/// for `BLOCK_SECS`; after that it is tried again (the machine, the prewarm and the runtime may have changed).
pub const KEPT_FLOOR_MS: u32 = 100;
const MIN_EXTRA_MS: u32 = 50;
const BLOCK_SECS: u64 = 6 * 3600;
const KEPT_KEEP: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// as quick as a kept window
    Fine,
    /// releasing is stopped in this situation until the block ends
    Blocked,
}

#[derive(Clone, Debug, Default)]
pub struct Guard {
    kept: Vec<u32>,
    cold_open_ms: Option<u32>,
    blocked_until: [u64; 3],
}

impl Guard {
    /// What a kept window costs, as the median of what was seen, never below the floor (a figure below it is the noise of a timer, not a goal).
    pub fn baseline_ms(&self) -> u32 {
        let mut kept = self.kept.clone();
        kept.sort_unstable();
        kept.get(kept.len() / 2).copied().unwrap_or(0).max(KEPT_FLOOR_MS)
    }

    /// The kept median plus a quarter, with 50 ms for measurement variation.
    pub fn limit_ms(&self) -> u32 {
        let base = self.baseline_ms();
        base.saturating_add(MIN_EXTRA_MS.max(base / 4))
    }

    pub fn record_kept(&mut self, ms: u32) {
        self.kept.push(ms);
        if self.kept.len() > KEPT_KEEP {
            self.kept.remove(0);
        }
    }

    pub fn record_cold_open(&mut self, ms: u32) {
        self.cold_open_ms = Some(ms);
    }

    /// A plain click has no lead time. Qualify its cost from a window that
    /// already had to be created before releasing a window the user could keep.
    pub fn cold_open_qualified(&self) -> bool {
        self.cold_open_ms.is_some_and(|ms| ms <= self.limit_ms())
    }

    pub fn record_released(&mut self, pressure: Pressure, ms: u32, now_secs: u64) -> Verdict {
        if ms <= self.limit_ms() {
            return Verdict::Fine;
        }
        self.blocked_until[pressure as usize] = now_secs.saturating_add(BLOCK_SECS);
        Verdict::Blocked
    }

    pub fn allows_release(&self, pressure: Pressure, now_secs: u64) -> bool {
        self.blocked_until[pressure as usize] <= now_secs
    }

    /// The blocks, to be kept between runs: `[none, elevated, critical]` as seconds since the epoch (0 = not blocked).
    pub fn blocks(&self) -> [u64; 3] {
        self.blocked_until
    }

    pub fn with_blocks(blocks: [u64; 3]) -> Guard {
        Guard { blocked_until: blocks, ..Guard::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(total_gb: u64, avail_mb: u64) -> Sample {
        Sample { total_phys: total_gb * GB, avail_phys: avail_mb * MB, commit_headroom: u64::MAX, low_signal: false }
    }

    #[test]
    fn modes_are_parsed_from_words_and_anything_else_is_off() {
        assert_eq!(Mode::parse("adaptive"), Mode::Adaptive);
        assert_eq!(Mode::parse(" LOW "), Mode::Low);
        assert_eq!(Mode::parse("low-memory"), Mode::Low);
        assert_eq!(Mode::parse(""), Mode::Off);
        assert_eq!(Mode::parse("yes"), Mode::Off);
    }

    #[test]
    fn nothing_asked_means_the_controller_picks_and_off_is_only_for_those_who_say_so() {
        assert_eq!(Setting::parse(""), Setting::Auto);
        assert_eq!(Setting::parse("auto"), Setting::Auto);
        assert_eq!(Setting::parse("whatever"), Setting::Auto);
        assert_eq!(Setting::parse("OFF"), Setting::Fixed(Mode::Off));
        assert_eq!(Setting::parse("adaptive"), Setting::Fixed(Mode::Adaptive));
        assert_eq!(Setting::parse(" low "), Setting::Fixed(Mode::Low));
        // auto: small machines get the low tier, the others the adaptive one, an unknown size the adaptive one
        assert_eq!(Setting::Auto.resolve(4 * GB), Mode::Low);
        assert_eq!(Setting::Auto.resolve(16 * GB), Mode::Adaptive);
        assert_eq!(Setting::Auto.resolve(0), Mode::Adaptive);
        // a fixed tier is kept whatever the memory
        assert_eq!(Setting::Fixed(Mode::Off).resolve(4 * GB), Mode::Off);
        assert_eq!(Setting::Fixed(Mode::Low).resolve(64 * GB), Mode::Low);
    }

    #[test]
    fn the_preset_is_chosen_by_installed_memory() {
        assert_eq!(Mode::suggested_for(0), Mode::Off);
        assert_eq!(Mode::suggested_for(4 * GB), Mode::Low);
        assert_eq!(Mode::suggested_for(7_900 * MB), Mode::Low);
        assert_eq!(Mode::suggested_for(8 * GB), Mode::Low);
        assert_eq!(Mode::suggested_for(16 * GB), Mode::Adaptive);
        assert_eq!(Mode::suggested_for(32 * GB), Mode::Adaptive);
    }

    #[test]
    fn thresholds_follow_the_installed_memory_with_floors() {
        assert_eq!(elevated_below(8 * GB), GB + GB / 2, "8 GB: the floor");
        assert_eq!(elevated_below(32 * GB), 32 * GB / 100 * 15);
        assert_eq!(critical_below(4 * GB), 512 * MB, "4 GB: the floor");
        assert_eq!(critical_below(64 * GB), 64 * GB / 100 * 6);
        for total in [4, 8, 16, 32, 64] {
            assert!(critical_below(total * GB) < elevated_below(total * GB));
        }
    }

    #[test]
    fn plenty_of_memory_is_no_pressure() {
        assert_eq!(classify(&sample(16, 6_000), Pressure::None), Pressure::None);
        assert_eq!(classify(&sample(8, 3_000), Pressure::None), Pressure::None);
    }

    #[test]
    fn short_memory_is_elevated_then_critical() {
        assert_eq!(classify(&sample(16, 2_000), Pressure::None), Pressure::Elevated);
        assert_eq!(classify(&sample(8, 1_400), Pressure::None), Pressure::Elevated);
        assert_eq!(classify(&sample(16, 800), Pressure::None), Pressure::Critical);
        assert_eq!(classify(&sample(8, 400), Pressure::None), Pressure::Critical);
    }

    #[test]
    fn the_system_low_memory_signal_is_critical_whatever_the_figures_say() {
        let mut s = sample(16, 6_000);
        s.low_signal = true;
        assert_eq!(classify(&s, Pressure::None), Pressure::Critical);
        s.low_signal = false;
        assert_eq!(classify(&s, Pressure::Critical), Pressure::None, "plenty of memory leaves any level");
    }

    #[test]
    fn little_commit_headroom_counts_as_pressure_with_memory_free() {
        let mut s = sample(16, 6_000);
        s.commit_headroom = GB + GB / 2;
        assert_eq!(classify(&s, Pressure::None), Pressure::Elevated);
        s.commit_headroom = 500 * MB;
        assert_eq!(classify(&s, Pressure::None), Pressure::Critical);
    }

    #[test]
    fn a_level_is_left_only_with_a_margin() {
        // 16 GB: elevated below 2457 MB; held until 25 % beyond that (3071 MB)
        assert_eq!(classify(&sample(16, 2_600), Pressure::None), Pressure::None);
        assert_eq!(classify(&sample(16, 2_600), Pressure::Elevated), Pressure::Elevated, "held");
        assert_eq!(classify(&sample(16, 3_200), Pressure::Elevated), Pressure::None, "left");
        // critical drops to elevated first, not to none
        assert_eq!(classify(&sample(16, 1_300), Pressure::Critical), Pressure::Elevated);
    }

    #[test]
    fn off_changes_nothing_at_any_pressure() {
        for pressure in [Pressure::None, Pressure::Elevated, Pressure::Critical] {
            assert!(!may_release_hidden_main(Mode::Off, pressure));
            assert_eq!(splash_policy(Mode::Off, pressure), SplashPolicy { warm_splash: true, splash_idle_release: None });
        }
    }

    #[test]
    fn hidden_main_requires_system_pressure_in_both_enabled_tiers() {
        for mode in [Mode::Adaptive, Mode::Low] {
            assert!(!may_release_hidden_main(mode, Pressure::None));
            for pressure in [Pressure::Elevated, Pressure::Critical] {
                assert!(may_release_hidden_main(mode, pressure));
            }
        }
    }

    #[test]
    fn adaptive_keeps_the_splash_warm_while_memory_is_plentiful() {
        assert_eq!(splash_policy(Mode::Adaptive, Pressure::None), SplashPolicy { warm_splash: true, splash_idle_release: None });
    }

    #[test]
    fn pressure_only_shortens_splash_delays() {
        for mode in [Mode::Adaptive, Mode::Low] {
            let levels = [Pressure::None, Pressure::Elevated, Pressure::Critical].map(|p| splash_policy(mode, p));
            for pair in levels.windows(2) {
                assert!(pair[1].splash_idle_release.unwrap_or(Duration::MAX) <= pair[0].splash_idle_release.unwrap_or(Duration::MAX));
            }
        }
    }

    #[test]
    fn low_never_warms_the_splash() {
        for pressure in [Pressure::None, Pressure::Elevated, Pressure::Critical] {
            assert!(!splash_policy(Mode::Low, pressure).warm_splash);
        }
    }

    #[test]
    fn a_kept_window_sets_the_bar_but_never_below_the_floor() {
        let mut guard = Guard::default();
        assert_eq!(guard.baseline_ms(), KEPT_FLOOR_MS);
        for ms in [20, 40, 30] {
            guard.record_kept(ms);
        }
        assert_eq!(guard.baseline_ms(), KEPT_FLOOR_MS, "a timer's noise is not the goal");
        let mut slow = Guard::default();
        for ms in [600, 640, 580] {
            slow.record_kept(ms);
        }
        assert_eq!(slow.baseline_ms(), 600);
        assert_eq!(slow.limit_ms(), 750, "a quarter more");
        assert_eq!(Guard::default().limit_ms(), 150, "at least 50 ms more");
    }

    #[test]
    fn a_released_window_as_quick_as_a_kept_one_keeps_releasing_allowed() {
        let mut guard = Guard::default();
        for ms in [110, 145, 130] {
            assert_eq!(guard.record_released(Pressure::None, ms, 1_000), Verdict::Fine);
        }
        assert!(guard.allows_release(Pressure::None, 1_000));
    }

    #[test]
    fn a_clearly_slower_release_stops_releasing_in_that_situation_only() {
        let mut guard = Guard::default();
        // a cold reopen of two seconds against a limit of 150 ms
        assert_eq!(guard.record_released(Pressure::None, 2_100, 1_000), Verdict::Blocked);
        assert!(!guard.allows_release(Pressure::None, 1_001));
        assert!(guard.allows_release(Pressure::Elevated, 1_001), "another situation has its own record");
        assert!(guard.allows_release(Pressure::None, 1_000 + 6 * 3600), "tried again after the block");
    }

    #[test]
    fn release_requires_a_measured_fast_cold_open() {
        let mut guard = Guard::default();
        assert!(!guard.cold_open_qualified());
        guard.record_cold_open(515);
        guard.record_kept(91);
        assert!(!guard.cold_open_qualified());
        guard.record_cold_open(140);
        assert!(guard.cold_open_qualified());
    }

    #[test]
    fn the_first_slow_release_blocks_that_pressure() {
        let mut guard = Guard::default();
        assert_eq!(guard.record_released(Pressure::Elevated, 151, 10), Verdict::Blocked);
        assert!(!guard.allows_release(Pressure::Elevated, 11));
        assert!(guard.allows_release(Pressure::None, 11));
    }

    #[test]
    fn the_blocks_survive_a_restart() {
        let mut guard = Guard::default();
        guard.record_released(Pressure::Critical, 3_000, 500);
        let restored = Guard::with_blocks(guard.blocks());
        assert!(!restored.allows_release(Pressure::Critical, 600));
        assert!(restored.allows_release(Pressure::None, 600));
    }

    #[test]
    fn overrides_are_whole_seconds_above_zero() {
        assert_eq!(override_secs(Some("90")), Some(Duration::from_secs(90)));
        assert_eq!(override_secs(Some(" 15 ")), Some(Duration::from_secs(15)));
        assert_eq!(override_secs(Some("0")), None);
        assert_eq!(override_secs(Some("soon")), None);
        assert_eq!(override_secs(None), None);
    }

    #[test]
    fn fake_pressure_words() {
        assert_eq!(parse_fake_pressure("critical\n"), Some(Pressure::Critical));
        assert_eq!(parse_fake_pressure("Elevated"), Some(Pressure::Elevated));
        assert_eq!(parse_fake_pressure("none"), Some(Pressure::None));
        assert_eq!(parse_fake_pressure("?"), None);
    }
}
