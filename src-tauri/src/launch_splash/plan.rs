//! The launch splash's decisions, pure: when to show, update and hide, from the host's launch state and from what the window search finds.
//!
//! The host decides the state (stage, notice, deadline, the processes to look for) and publishes it in the tray state; this side owns the screen. The splash
//! ends on whichever comes first: the game's window is found (the host does not need to be told), the host says the launch is over, the deadline passes, or
//! Core goes away. An id that ended here is never shown again, so a host that keeps sending the old state cannot bring it back.

use crate::sunshine::TrayLaunchState;

/// A resumed app already has its window: the splash waits this long for the search to find it before showing itself.
pub const RESUME_GRACE_MS: i64 = 400;
/// How many ended ids are remembered.
const REMEMBER: usize = 8;

/// What the splash page is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: String,
    pub app_name: String,
    pub art: String,
    pub stage: String,
    pub notice: String,
    pub display: String,
}

impl Frame {
    fn of(launch: &TrayLaunchState) -> Self {
        Frame {
            id: launch.id.clone(),
            app_name: launch.app_name.clone(),
            art: launch.art.clone(),
            stage: launch.stage.clone(),
            notice: launch.notice.clone(),
            display: launch.display.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Put the splash on the display and draw this frame (a splash already up draws the new launch).
    Show(Frame),
    /// The same launch, a new stage or line.
    Update(Frame),
    /// Fade out and hide.
    Hide,
}

#[derive(Default)]
pub struct Plan {
    current: Option<TrayLaunchState>,
    shown: bool,
    last: Option<Frame>,
    resume_show_at: Option<i64>,
    ended: Vec<String>,
}

impl Plan {
    /// The launch being followed (shown or waiting to be), for the window search.
    pub fn watching(&self) -> Option<&TrayLaunchState> {
        self.current.as_ref()
    }

    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// Whether a resume is still waiting out its grace: the search has to run for it too.
    pub fn is_resume(&self) -> bool {
        self.current.as_ref().is_some_and(|c| c.kind == "resume")
    }

    fn end(&mut self) -> Vec<Cmd> {
        if let Some(current) = self.current.take() {
            self.ended.push(current.id);
            if self.ended.len() > REMEMBER {
                self.ended.remove(0);
            }
        }
        let was_shown = std::mem::take(&mut self.shown);
        self.last = None;
        self.resume_show_at = None;
        if was_shown { vec![Cmd::Hide] } else { vec![] }
    }

    /// The host's state arrived.
    pub fn on_state(&mut self, state: &TrayLaunchState, now_ms: i64) -> Vec<Cmd> {
        if state.id.is_empty() || self.ended.contains(&state.id) {
            return vec![];
        }
        let known = self.current.as_ref().is_some_and(|c| c.id == state.id);
        if !known {
            if !state.active {
                // a launch that was over before it was seen
                self.ended.push(state.id.clone());
                return vec![];
            }
            // a new launch replaces the one before it without a flicker: the window keeps up and draws the new one
            self.current = Some(state.clone());
            self.resume_show_at = None;
            if state.kind == "resume" {
                let hide = std::mem::take(&mut self.shown);
                self.last = None;
                self.resume_show_at = Some(now_ms + RESUME_GRACE_MS);
                return if hide { vec![Cmd::Hide] } else { vec![] };
            }
            let frame = Frame::of(state);
            self.shown = true;
            self.last = Some(frame.clone());
            return vec![Cmd::Show(frame)];
        }
        if !state.active {
            self.current = Some(state.clone());
            return self.end();
        }
        self.current = Some(state.clone());
        if self.shown {
            let frame = Frame::of(state);
            if self.last.as_ref() != Some(&frame) {
                self.last = Some(frame.clone());
                return vec![Cmd::Update(frame)];
            }
        }
        vec![]
    }

    /// Time passed; `window_found` is what the search saw in the game's processes.
    pub fn on_tick(&mut self, window_found: bool, now_ms: i64) -> Vec<Cmd> {
        let Some(current) = self.current.as_ref() else { return vec![] };
        if now_ms >= current.deadline_ms {
            return self.end();
        }
        // a failure stays up until its time is out: a window that appears after it does not take the reason away
        if window_found && current.stage != "failed" {
            return self.end();
        }
        if !self.shown && self.resume_show_at.is_some_and(|at| now_ms >= at) {
            let frame = Frame::of(current);
            self.shown = true;
            self.resume_show_at = None;
            self.last = Some(frame.clone());
            return vec![Cmd::Show(frame)];
        }
        vec![]
    }

    /// Core went away: whatever is up goes.
    pub fn on_disconnect(&mut self) -> Vec<Cmd> {
        self.end()
    }
}

/// The number of a Windows display name (`\\.\DISPLAY3` is 3).
fn display_number(text: &str) -> Option<u32> {
    let upper = text.to_ascii_uppercase();
    let at = upper.find("DISPLAY")? + "DISPLAY".len();
    let digits: String = upper[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Where the splash goes: the index of the monitor whose name is the host's display (`\\.\DISPLAYn`), else the primary one, else the first. None without monitors.
pub fn pick_display(display: &str, names: &[String], primary: Option<usize>) -> Option<usize> {
    if let Some(wanted) = display_number(display) {
        if let Some(found) = names.iter().position(|name| display_number(name) == Some(wanted)) {
            return Some(found);
        }
    }
    primary.filter(|index| *index < names.len()).or(if names.is_empty() { None } else { Some(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(id: &str, kind: &str, stage: &str) -> TrayLaunchState {
        TrayLaunchState {
            id: id.to_string(),
            active: true,
            kind: kind.to_string(),
            app_id: "13".to_string(),
            app_name: "Genshin Impact".to_string(),
            stage: stage.to_string(),
            started_at_ms: 1_000,
            deadline_ms: 61_000,
            ..Default::default()
        }
    }

    fn frame(id: &str, stage: &str) -> Frame {
        Frame { id: id.to_string(), app_name: "Genshin Impact".to_string(), art: String::new(), stage: stage.to_string(), notice: String::new(), display: String::new() }
    }

    #[test]
    fn a_launch_shows_at_once_and_follows_the_stages() {
        let mut plan = Plan::default();
        assert_eq!(plan.on_state(&launch("a", "launch", "starting"), 1_000), vec![Cmd::Show(frame("a", "starting"))]);
        assert!(plan.is_shown());
        assert_eq!(plan.on_state(&launch("a", "launch", "preparing"), 1_500), vec![Cmd::Update(frame("a", "preparing"))]);
        // the same state again is nothing to draw
        assert_eq!(plan.on_state(&launch("a", "launch", "preparing"), 1_600), vec![]);
    }

    #[test]
    fn the_window_found_ends_it_for_good() {
        let mut plan = Plan::default();
        plan.on_state(&launch("a", "launch", "waiting-window"), 1_000);
        assert_eq!(plan.on_tick(false, 2_000), vec![]);
        assert_eq!(plan.on_tick(true, 3_000), vec![Cmd::Hide]);
        assert!(plan.watching().is_none());
        // the host has not heard: it keeps sending the active state, and the splash stays gone
        assert_eq!(plan.on_state(&launch("a", "launch", "waiting-window"), 3_100), vec![]);
        assert!(!plan.is_shown());
    }

    #[test]
    fn the_host_ending_the_launch_hides_it() {
        let mut plan = Plan::default();
        plan.on_state(&launch("a", "launch", "starting"), 1_000);
        let mut over = launch("a", "launch", "starting");
        over.active = false;
        assert_eq!(plan.on_state(&over, 2_000), vec![Cmd::Hide]);
        assert_eq!(plan.on_state(&launch("a", "launch", "starting"), 2_100), vec![]);
    }

    #[test]
    fn it_times_out_at_the_hosts_deadline_even_if_the_host_goes_quiet() {
        let mut plan = Plan::default();
        plan.on_state(&launch("a", "launch", "waiting-window"), 1_000);
        assert_eq!(plan.on_tick(false, 60_999), vec![]);
        assert_eq!(plan.on_tick(false, 61_000), vec![Cmd::Hide]);
    }

    #[test]
    fn a_failure_stays_until_its_time_and_a_window_does_not_take_the_reason_away() {
        let mut plan = Plan::default();
        plan.on_state(&launch("a", "launch", "waiting-window"), 1_000);
        let mut failed = launch("a", "launch", "failed");
        failed.notice = "timed-out".to_string();
        failed.deadline_ms = 6_000;
        let shown = plan.on_state(&failed, 1_000);
        assert!(matches!(shown.as_slice(), [Cmd::Update(f)] if f.stage == "failed" && f.notice == "timed-out"));
        assert_eq!(plan.on_tick(true, 2_000), vec![]);
        assert_eq!(plan.on_tick(false, 6_000), vec![Cmd::Hide]);
    }

    #[test]
    fn a_new_launch_takes_over_the_screen_without_hiding_it() {
        let mut plan = Plan::default();
        plan.on_state(&launch("a", "launch", "waiting-window"), 1_000);
        assert_eq!(plan.on_state(&launch("b", "launch", "starting"), 2_000), vec![Cmd::Show(frame("b", "starting"))]);
        assert_eq!(plan.watching().map(|l| l.id.as_str()), Some("b"));
    }

    #[test]
    fn a_launch_that_was_over_before_it_was_seen_is_not_shown() {
        let mut plan = Plan::default();
        let mut over = launch("a", "launch", "starting");
        over.active = false;
        assert_eq!(plan.on_state(&over, 1_000), vec![]);
        assert_eq!(plan.on_state(&launch("a", "launch", "starting"), 1_100), vec![]);
        assert!(plan.watching().is_none());
    }

    #[test]
    fn a_resume_whose_window_is_found_in_the_grace_is_never_shown() {
        let mut plan = Plan::default();
        assert_eq!(plan.on_state(&launch("r", "resume", "starting"), 1_000), vec![]);
        assert!(plan.is_resume() && !plan.is_shown());
        assert_eq!(plan.on_tick(true, 1_200), vec![]);
        assert!(plan.watching().is_none());
    }

    #[test]
    fn a_resume_whose_window_is_missing_after_the_grace_shows() {
        let mut plan = Plan::default();
        plan.on_state(&launch("r", "resume", "starting"), 1_000);
        assert_eq!(plan.on_tick(false, 1_399), vec![]);
        assert_eq!(plan.on_tick(false, 1_400), vec![Cmd::Show(frame("r", "starting"))]);
        assert_eq!(plan.on_tick(true, 1_500), vec![Cmd::Hide]);
    }

    #[test]
    fn the_host_ending_a_waiting_resume_shows_nothing() {
        let mut plan = Plan::default();
        plan.on_state(&launch("r", "resume", "starting"), 1_000);
        let mut over = launch("r", "resume", "starting");
        over.active = false;
        assert_eq!(plan.on_state(&over, 1_100), vec![]);
        assert!(plan.watching().is_none());
    }

    #[test]
    fn core_going_away_hides_what_is_up_and_only_that() {
        let mut plan = Plan::default();
        assert_eq!(plan.on_disconnect(), vec![]);
        plan.on_state(&launch("a", "launch", "starting"), 1_000);
        assert_eq!(plan.on_disconnect(), vec![Cmd::Hide]);
    }

    #[test]
    fn the_ended_ids_are_remembered_a_few_at_a_time() {
        let mut plan = Plan::default();
        for n in 0..20 {
            plan.on_state(&launch(&format!("l{n}"), "launch", "starting"), 1_000);
            plan.on_tick(true, 1_100);
        }
        assert!(plan.ended.len() <= REMEMBER);
        assert_eq!(plan.on_state(&launch("l19", "launch", "starting"), 2_000), vec![]);
    }

    #[test]
    fn the_empty_id_of_a_host_that_never_launched_is_ignored() {
        let mut plan = Plan::default();
        assert_eq!(plan.on_state(&TrayLaunchState::default(), 1_000), vec![]);
        assert!(plan.watching().is_none());
    }

    #[test]
    fn the_splash_goes_to_the_display_the_host_names_else_the_primary_one() {
        let names: Vec<String> = ["\\\\.\\DISPLAY1", "\\\\.\\DISPLAY3"].iter().map(|s| s.to_string()).collect();
        assert_eq!(pick_display("\\\\.\\DISPLAY3", &names, Some(0)), Some(1));
        assert_eq!(pick_display("\\\\.\\display1", &names, Some(1)), Some(0), "case does not matter");
        assert_eq!(pick_display("", &names, Some(1)), Some(1), "no display named: the primary one");
        assert_eq!(pick_display("\\\\.\\DISPLAY9", &names, Some(1)), Some(1), "a display that is not there: the primary one");
        assert_eq!(pick_display("\\\\.\\DISPLAY9", &names, None), Some(0));
        assert_eq!(pick_display("\\\\.\\DISPLAY1", &[], None), None);
        assert_eq!(pick_display("garbage", &names, Some(7)), Some(0), "a primary index that does not exist is not trusted");
    }
}
