//! What the tray icon does when it is clicked: which of the three menus (V1 compact list, V2 status card + list, V3 quick panel) the RIGHT click opens, what the LEFT click and the double click do
//! (open the main window, open one of the menus, nothing), and a switch back to the system's own menu. The record is edited in the Panel's 个性化 page (foundation-ui MTraySettings, whose
//! `src/tray/trayPrefs.js` is the same shape and the same rules - both are tested) and kept in `tray-prefs.json` beside the other settings of the GUI.
//!
//! The arithmetic of a click is here, without a window or a timer, so that it is tested: `ClickGate` turns the events of the icon (button down, button up, double click) into "do this now" /
//! "do this after the double-click time unless a double click comes" / "ignore".

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const FILE: &str = "tray-prefs.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    V1,
    V2,
    V3,
}

impl Variant {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "v1" => Some(Self::V1),
            "v2" => Some(Self::V2),
            "v3" => Some(Self::V3),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
            Self::V3 => "v3",
        }
    }
    /// Width of the menu card in CSS px (foundation-ui tray.css), used while the page measures the card.
    pub fn width_css(self) -> f64 {
        match self {
            Self::V1 => 304.0,
            Self::V2 => 344.0,
            Self::V3 => 360.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the Panel's main window.
    Panel,
    /// Open one of the three menus at the icon.
    Menu(Variant),
    None,
}

impl Action {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "panel" => Some(Self::Panel),
            "none" => Some(Self::None),
            other => Variant::parse(other).map(Self::Menu),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Right,
    Click,
    Double,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrayPrefs {
    pub right: String,
    pub click: String,
    pub double: String,
    pub force_native: bool,
    /// The tiles of the quick panel (V3), in order, once the person has edited it; None = the default of foundation-ui (`DEFAULT_TILES`).
    pub tiles: Option<Vec<String>>,
    /// The sliders of the three menus (V1, V2 and the quick panel read this one list, and so do the native menu's stepped submenus), in order; None = the default (the bitrate cap, then the scale).
    /// A list saved before the default changed stays as it was saved.
    pub sliders: Option<Vec<String>>,
    /// The density of the three menus: "auto" (a finger or a pen that opens the menu, or slate posture, makes it touch; the page corrects the guess with its first pointer), "standard" or "touch".
    /// A record saved before this existed has none and reads as "auto".
    pub density: String,
}

/// The densities a record may hold (foundation-ui `TRAY_DENSITIES`).
pub const DENSITIES: [&str; 3] = ["auto", "standard", "touch"];

impl Default for TrayPrefs {
    fn default() -> Self {
        Self { right: "v1".into(), click: "panel".into(), double: "none".into(), force_native: false, tiles: None, sliders: None, density: "auto".into() }
    }
}

/// The tile ids of the quick panel (foundation-ui `TILES`, src/tray/trayModel.js; trayWiring.test.js compares the two lists). An id outside it is dropped.
pub const TILE_IDS: [&str; 17] = [
    "open_main_panel", "open_desktop", "connect_device", "download_client", "vdd", "auto_start", "check_update", "open_sunshine", "show_toolbar", "host_performance", "rtss_control", "log_console",
    "prevent_sleep", "restart_user_mode", "vdd_toggle_keep_enabled", "vdd_toggle_headless_create", "reset_display",
];
/// The slider keys of the quick panel (foundation-ui `SLIDER_KEYS`).
pub const SLIDER_KEYS: [&str; 4] = ["resolution", "refresh", "dpi", "bitrate"];

/// A list of known ids without repeats, or None when nothing was stored (the default). An empty list is a real choice.
fn known_ids(list: Option<Vec<String>>, known: &[&str]) -> Option<Vec<String>> {
    let list = list?;
    let mut out: Vec<String> = Vec::new();
    for id in list {
        if known.contains(&id.as_str()) && !out.contains(&id) {
            out.push(id);
        }
    }
    Some(out)
}

impl TrayPrefs {
    /// A valid record from whatever was stored: an unknown value takes the default; a right click is always one of the three menus.
    pub fn normalized(self) -> Self {
        let d = Self::default();
        Self {
            right: if Variant::parse(&self.right).is_some() { self.right } else { d.right },
            click: if Action::parse(&self.click).is_some() { self.click } else { d.click },
            double: if Action::parse(&self.double).is_some() { self.double } else { d.double },
            force_native: self.force_native,
            tiles: known_ids(self.tiles, &TILE_IDS),
            sliders: known_ids(self.sliders, &SLIDER_KEYS),
            density: if DENSITIES.contains(&self.density.as_str()) { self.density } else { d.density },
        }
    }

    pub fn action(&self, gesture: Gesture) -> Action {
        let p = self.clone().normalized();
        match gesture {
            Gesture::Right => Action::parse(&p.right).unwrap_or(Action::Menu(Variant::V1)),
            Gesture::Click => Action::parse(&p.click).unwrap_or(Action::Panel),
            Gesture::Double => Action::parse(&p.double).unwrap_or(Action::None),
        }
    }

    /// A single click waits for the double-click time only when a double click has an action of its own.
    pub fn single_click_waits(&self) -> bool {
        self.action(Gesture::Click) != Action::None && self.action(Gesture::Double) != Action::None
    }
}

/// What the gate decided for an event of the icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Now(Action),
    /// Run the action after the wait unless `fire(ticket)` says a double click came first.
    After(Duration, u64, Action),
    Ignore,
}

/// Button events of the left button -> decisions. Windows delivers: down, up, (double click), up - the second up of a double click must not count as a click.
#[derive(Default)]
pub struct ClickGate {
    ticket: u64,
    after_double: bool,
}

impl ClickGate {
    pub fn left_down(&mut self) {
        self.after_double = false;
    }

    pub fn left_up(&mut self, prefs: &TrayPrefs, wait: Duration) -> Decision {
        if std::mem::take(&mut self.after_double) {
            return Decision::Ignore;
        }
        let action = prefs.action(Gesture::Click);
        if action == Action::None {
            return Decision::Ignore;
        }
        if prefs.single_click_waits() {
            self.ticket = self.ticket.wrapping_add(1);
            return Decision::After(wait, self.ticket, action);
        }
        Decision::Now(action)
    }

    pub fn double_click(&mut self, prefs: &TrayPrefs) -> Decision {
        self.ticket = self.ticket.wrapping_add(1); // a click that was waiting is cancelled
        self.after_double = true;
        match prefs.action(Gesture::Double) {
            Action::None => Decision::Ignore,
            action => Decision::Now(action),
        }
    }

    /// The wait of a click is over: true when it is still the latest (no double click came, no later click replaced it).
    pub fn fire(&mut self, ticket: u64) -> bool {
        self.ticket == ticket
    }
}

/// The double-click time of the system (the wait of a single click beside a double-click action).
pub fn double_click_wait() -> Duration {
    #[cfg(target_os = "windows")]
    {
        let ms = unsafe { ::windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() };
        Duration::from_millis(u64::from(ms).clamp(200, 900))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Duration::from_millis(500)
    }
}

/// A menu that was closed because the tray icon was pressed again must not open again on the same click (the press took the focus away, the release would open it): the time of the last such close.
static LAST_ICON_CLOSE: Mutex<Option<Instant>> = Mutex::new(None);
pub fn note_closed_by_focus_loss() {
    *LAST_ICON_CLOSE.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
}
pub fn just_closed_by_icon_press() -> bool {
    LAST_ICON_CLOSE.lock().unwrap_or_else(|p| p.into_inner()).is_some_and(|t| t.elapsed() < Duration::from_millis(350))
}

// ---- storage ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

static CURRENT: Mutex<Option<TrayPrefs>> = Mutex::new(None);

fn path() -> Option<std::path::PathBuf> {
    crate::test_isolation::config_dir().map(|dir| dir.join("Sunshine GUI").join(FILE))
}

pub fn parse(text: &str) -> TrayPrefs {
    serde_json::from_str::<TrayPrefs>(text).map(TrayPrefs::normalized).unwrap_or_default()
}

pub fn current() -> TrayPrefs {
    let mut guard = CURRENT.lock().unwrap_or_else(|p| p.into_inner());
    guard.get_or_insert_with(|| path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| parse(&t)).unwrap_or_default()).clone()
}

fn store(next: TrayPrefs) -> Result<(), String> {
    let file = path().ok_or("no config directory")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    *CURRENT.lock().unwrap_or_else(|p| p.into_inner()) = Some(next);
    Ok(())
}

/// Stores a record (normalised) and returns it; the caller refreshes what depends on it (the tray's menu mode).
pub fn save(prefs: TrayPrefs) -> Result<TrayPrefs, String> {
    let next = prefs.normalized();
    store(next.clone())?;
    Ok(next)
}

/// The settings page's record, with the layout of the quick panel left as stored: the layout is written only by `save_layout`, so a page that was opened before an edit in the menu cannot overwrite it.
pub fn merge_keeping_layout(mut incoming: TrayPrefs, stored: &TrayPrefs) -> TrayPrefs {
    incoming.tiles = stored.tiles.clone();
    incoming.sliders = stored.sliders.clone();
    incoming
}
pub fn save_keeping_layout(prefs: TrayPrefs) -> Result<TrayPrefs, String> {
    save(merge_keeping_layout(prefs, &current()))
}

/// Stores the layout of the quick panel (the rest of the record as stored) and returns the record.
pub fn save_layout(tiles: Option<Vec<String>>, sliders: Option<Vec<String>>) -> Result<TrayPrefs, String> {
    let mut next = current();
    next.tiles = tiles;
    next.sliders = sliders;
    save(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAIT: Duration = Duration::from_millis(500);

    fn prefs(click: &str, double: &str) -> TrayPrefs {
        TrayPrefs { right: "v1".into(), click: click.into(), double: double.into(), ..TrayPrefs::default() }
    }

    #[test]
    fn the_density_defaults_to_auto_and_an_old_record_without_it_still_reads() {
        assert_eq!(TrayPrefs::default().density, "auto");
        let old = parse(r#"{"right":"v3","click":"panel","double":"none","forceNative":false,"tiles":null,"sliders":null}"#);
        assert_eq!(old.density, "auto");
        assert_eq!(parse(r#"{"density":"touch"}"#).density, "touch");
        assert_eq!(TrayPrefs { density: "huge".into(), ..TrayPrefs::default() }.normalized().density, "auto");
    }

    #[test]
    fn restoring_the_defaults_writes_the_defaults_as_they_are() {
        // 恢复 Panel 默认偏好 saves `TrayPrefs::default()` through `save`, which normalises: the record on disk must be the default, with the layout left to the default (None)
        let d = TrayPrefs::default();
        assert_eq!(d.clone().normalized(), d);
        assert!(d.tiles.is_none() && d.sliders.is_none());
    }

    #[test]
    fn the_defaults_are_v1_on_the_right_click_the_main_window_on_the_click_and_nothing_on_a_double_click() {
        let d = TrayPrefs::default();
        assert_eq!((d.right.as_str(), d.click.as_str(), d.double.as_str(), d.force_native), ("v1", "panel", "none", false));
        assert_eq!(d.action(Gesture::Right), Action::Menu(Variant::V1));
        assert_eq!(d.action(Gesture::Click), Action::Panel);
        assert_eq!(d.action(Gesture::Double), Action::None);
    }

    #[test]
    fn unknown_values_take_the_defaults_and_a_right_click_is_always_a_menu() {
        let bad = TrayPrefs { right: "none".into(), click: "v9".into(), double: "".into(), force_native: true, ..TrayPrefs::default() }.normalized();
        assert_eq!((bad.right.as_str(), bad.click.as_str(), bad.double.as_str(), bad.force_native), ("v1", "panel", "none", true));
        assert_eq!(parse("not json"), TrayPrefs::default());
        assert_eq!(parse(r#"{"right":"v3","click":"v2","double":"panel","forceNative":true}"#).action(Gesture::Right), Action::Menu(Variant::V3));
        // a partial record keeps the rest at the defaults
        let partial = parse(r#"{"right":"v3"}"#);
        assert_eq!((partial.right.as_str(), partial.click.as_str()), ("v3", "panel"));
    }

    fn ids(list: &[&str]) -> Option<Vec<String>> {
        Some(list.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn the_layout_is_the_default_until_edited_and_keeps_known_ids_once_in_order() {
        assert_eq!((TrayPrefs::default().tiles, TrayPrefs::default().sliders), (None, None));
        let p = TrayPrefs { tiles: ids(&["vdd", "nope", "auto_start", "vdd"]), sliders: ids(&["bitrate", "dpi", "zzz"]), ..TrayPrefs::default() }.normalized();
        assert_eq!((p.tiles, p.sliders), (ids(&["vdd", "auto_start"]), ids(&["bitrate", "dpi"])));
        // an empty list is a real choice (nothing shown), not the default
        let none = TrayPrefs { tiles: ids(&[]), sliders: ids(&[]), ..TrayPrefs::default() }.normalized();
        assert_eq!((none.tiles, none.sliders), (ids(&[]), ids(&[])));
        // it survives the file, and an old record without it reads as the default
        let parsed = parse(r#"{"right":"v3","tiles":["open_desktop","check_update"],"sliders":["dpi","refresh"]}"#);
        assert_eq!((parsed.tiles, parsed.sliders), (ids(&["open_desktop", "check_update"]), ids(&["dpi", "refresh"])));
        assert_eq!(parse(r#"{"right":"v3"}"#).tiles, None);
        assert_eq!(parse(r#"{"tiles":"x"}"#), TrayPrefs::default(), "a wrong shape is the default record");
    }

    #[test]
    fn the_settings_page_cannot_overwrite_the_layout() {
        let stored = TrayPrefs { tiles: ids(&["vdd"]), sliders: ids(&["dpi", "bitrate"]), ..TrayPrefs::default() };
        let incoming = TrayPrefs { right: "v3".into(), tiles: None, sliders: ids(&["refresh"]), ..TrayPrefs::default() };
        let merged = merge_keeping_layout(incoming, &stored);
        assert_eq!((merged.right.as_str(), merged.tiles, merged.sliders), ("v3", ids(&["vdd"]), ids(&["dpi", "bitrate"])));
    }

    #[test]
    fn the_same_mapping_as_the_settings_page_for_every_value() {
        for (text, action) in [("panel", Action::Panel), ("v1", Action::Menu(Variant::V1)), ("v2", Action::Menu(Variant::V2)), ("v3", Action::Menu(Variant::V3)), ("none", Action::None)] {
            assert_eq!(prefs(text, text).action(Gesture::Click), action);
            assert_eq!(prefs(text, text).action(Gesture::Double), action);
        }
    }

    #[test]
    fn a_click_runs_at_once_when_a_double_click_has_no_action() {
        let mut gate = ClickGate::default();
        let p = prefs("panel", "none");
        gate.left_down();
        assert_eq!(gate.left_up(&p, WAIT), Decision::Now(Action::Panel));
        // the double click of the same gesture: nothing to do, and its second release is not a click
        assert_eq!(gate.double_click(&p), Decision::Ignore);
        assert_eq!(gate.left_up(&p, WAIT), Decision::Ignore);
        gate.left_down();
        assert_eq!(gate.left_up(&p, WAIT), Decision::Now(Action::Panel), "the next click counts again");
    }

    #[test]
    fn beside_a_double_click_action_the_click_waits_and_a_double_click_cancels_it() {
        let mut gate = ClickGate::default();
        let p = prefs("v1", "panel");
        gate.left_down();
        let Decision::After(wait, ticket, Action::Menu(Variant::V1)) = gate.left_up(&p, WAIT) else { panic!("the click waits") };
        assert_eq!(wait, WAIT);
        assert_eq!(gate.double_click(&p), Decision::Now(Action::Panel));
        assert!(!gate.fire(ticket), "the double click took the single click's place");
        assert_eq!(gate.left_up(&p, WAIT), Decision::Ignore, "the release that ends the double click");
    }

    #[test]
    fn a_waiting_click_fires_when_no_double_click_comes_and_a_later_click_replaces_it() {
        let mut gate = ClickGate::default();
        let p = prefs("v2", "v3");
        gate.left_down();
        let Decision::After(_, first, _) = gate.left_up(&p, WAIT) else { panic!() };
        assert!(gate.fire(first));
        gate.left_down();
        let Decision::After(_, second, _) = gate.left_up(&p, WAIT) else { panic!() };
        assert!(!gate.fire(first) && gate.fire(second));
    }

    #[test]
    fn a_click_set_to_nothing_does_nothing_and_the_double_click_still_works() {
        let mut gate = ClickGate::default();
        let p = prefs("none", "panel");
        gate.left_down();
        assert_eq!(gate.left_up(&p, WAIT), Decision::Ignore);
        assert_eq!(gate.double_click(&p), Decision::Now(Action::Panel));
        assert!(!p.single_click_waits());
    }

    #[test]
    fn the_single_click_waits_only_beside_a_double_click_action() {
        assert!(!prefs("panel", "none").single_click_waits());
        assert!(prefs("panel", "v1").single_click_waits());
        assert!(!prefs("none", "v1").single_click_waits());
    }

    #[test]
    fn the_cards_have_the_widths_of_the_stylesheet() {
        assert_eq!((Variant::V1.width_css(), Variant::V2.width_css(), Variant::V3.width_css()), (304.0, 344.0, 360.0));
        assert_eq!(Variant::parse("v2").map(Variant::as_str), Some("v2"));
    }
}
