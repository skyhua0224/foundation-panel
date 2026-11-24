//! What opened the tray menu: a finger, a pen or a mouse. The menu draws itself bigger for a finger (foundation-ui touch density), and it needs the answer before the first pointer event of its own page.
//!
//! Three facts, from the most to the least reliable for "this very press was a touch":
//!   1. the input source of the message of the tray icon's click: `GetCurrentInputMessageSource` (Windows 8+, INPUT_MESSAGE_SOURCE.deviceType: IMDT_TOUCH 0x04, IMDT_PEN 0x08, IMDT_MOUSE 0x02, IMDT_TOUCHPAD
//!      0x10; the call answers for the message being dispatched on this thread and says IMDT_UNAVAILABLE outside one, which is why it is read in the icon's own callback and kept for two seconds), with the
//!      older `GetMessageExtraInfo` signature (0xFF515700 / 0xFF515780 under the 0xFFFFFF00 / 0xFFFFFF80 masks) as a second reading when the first one is unavailable;
//!   2. whether the machine is in slate (tablet) posture: `GetSystemMetrics(SM_CONVERTIBLESLATEMODE)` is 0 in slate mode (Windows 8+; 0x2003). It says nothing about this press, but a person who has folded the
//!      keyboard away uses fingers;
//!   3. whether there is a digitizer at all: `GetSystemMetrics(SM_MAXIMUMTOUCHES)` > 0. Most 2-in-1 laptops have one and are used with a mouse, so this alone never switches the density; it is reported for the
//!      page and the report.
//! The page corrects the guess with the `pointerType` of its first pointerdown (foundation-ui MTrayMenu), so a wrong guess costs one press, not a mode.
//! Sources: learn.microsoft.com win32/api/winuser GetCurrentInputMessageSource, INPUT_MESSAGE_DEVICE_TYPE, GetSystemMetrics, GetMessageExtraInfo.

use serde_json::{Value, json};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Unknown,
    Keyboard,
    Mouse,
    Touch,
    Pen,
    Touchpad,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Keyboard => "keyboard",
            Self::Mouse => "mouse",
            Self::Touch => "touch",
            Self::Pen => "pen",
            Self::Touchpad => "touchpad",
        }
    }
}

/// INPUT_MESSAGE_DEVICE_TYPE -> Source.
pub fn source_from_device_type(device_type: i32) -> Source {
    match device_type {
        0x01 => Source::Keyboard,
        0x02 => Source::Mouse,
        0x04 => Source::Touch,
        0x08 => Source::Pen,
        0x10 => Source::Touchpad,
        _ => Source::Unknown,
    }
}

/// The legacy reading: GetMessageExtraInfo carries 0xFF515700 (pen) or 0xFF515780 (touch) in its upper bits for messages that came from a pen or a finger.
pub fn source_from_extra_info(extra: isize) -> Source {
    let extra = (extra as usize) as u32;
    if extra & 0xFFFF_FF80 == 0xFF51_5780 {
        Source::Touch
    } else if extra & 0xFFFF_FF00 == 0xFF51_5700 {
        Source::Pen
    } else {
        Source::Unknown
    }
}

static LAST: Mutex<Option<(Source, Instant)>> = Mutex::new(None);
const FRESH: Duration = Duration::from_secs(2);

/// Called first thing in the tray icon's event callback (the message of the click is being dispatched): remembers what it came from.
pub fn note_icon_event() {
    #[cfg(target_os = "windows")]
    {
        let source = windows_source();
        if source != Source::Unknown {
            *LAST.lock().unwrap_or_else(|p| p.into_inner()) = Some((source, Instant::now()));
        }
    }
}

/// What the system says about the message being dispatched right now, as text for the log (diagnostics of the touch path: which messages of a gesture carry a device type).
pub fn describe_message_source() -> String {
    #[cfg(target_os = "windows")]
    {
        let mut message = ffi::InputMessageSource { device_type: 0, origin_id: 0 };
        // SAFETY: a plain out-parameter of the documented layout, and a call without arguments.
        let (ok, extra) = unsafe { (ffi::GetCurrentInputMessageSource(&mut message) != 0, ffi::GetMessageExtraInfo()) };
        format!("source ok={} device={:#x} extra={:#x}", ok, message.device_type, extra as usize as u32)
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from("source n/a")
    }
}

/// What the last click on the icon came from, if it was a moment ago.
pub fn last_source() -> Source {
    match *LAST.lock().unwrap_or_else(|p| p.into_inner()) {
        Some((source, at)) if at.elapsed() < FRESH => source,
        _ => Source::Unknown,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub source: Source,
    /// the machine is in slate (tablet) posture
    pub slate: bool,
    /// the machine has a digitizer
    pub capable: bool,
}

pub fn facts() -> Facts {
    #[cfg(target_os = "windows")]
    {
        Facts { source: last_source(), slate: windows_slate(), capable: windows_capable() }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Facts { source: last_source(), slate: false, capable: false }
    }
}

impl Facts {
    pub fn to_json(self) -> Value {
        json!({ "source": self.source.as_str(), "slate": self.slate, "capable": self.capable })
    }
}

/// The density the menu opens in for a preference ("auto" | "standard" | "touch") and the facts: a person's choice wins; "auto" is touch for a finger or a pen, and for slate posture when the press is not
/// known to be a mouse; a digitizer alone is not enough.
pub fn opening_density(pref: &str, facts: Facts) -> &'static str {
    match pref {
        "touch" => "touch",
        "standard" => "standard",
        _ => match facts.source {
            Source::Touch | Source::Pen => "touch",
            Source::Mouse | Source::Touchpad | Source::Keyboard => "standard",
            Source::Unknown if facts.slate => "touch",
            Source::Unknown => "standard",
        },
    }
}

#[cfg(target_os = "windows")]
mod ffi {
    #[repr(C)]
    pub struct InputMessageSource {
        pub device_type: i32,
        pub origin_id: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn GetCurrentInputMessageSource(source: *mut InputMessageSource) -> i32;
        pub fn GetMessageExtraInfo() -> isize;
        pub fn GetSystemMetrics(index: i32) -> i32;
    }
}

#[cfg(target_os = "windows")]
fn windows_source() -> Source {
    let mut message = ffi::InputMessageSource { device_type: 0, origin_id: 0 };
    // SAFETY: a plain out-parameter of the documented layout (INPUT_MESSAGE_SOURCE: two 32-bit enums).
    let ok = unsafe { ffi::GetCurrentInputMessageSource(&mut message) } != 0;
    let source = if ok { source_from_device_type(message.device_type) } else { Source::Unknown };
    if source != Source::Unknown {
        return source;
    }
    // SAFETY: no arguments, reads the calling thread's extra message information.
    source_from_extra_info(unsafe { ffi::GetMessageExtraInfo() })
}

/// SM_CONVERTIBLESLATEMODE (0x2003): 0 means slate mode.
#[cfg(target_os = "windows")]
fn windows_slate() -> bool {
    // SAFETY: GetSystemMetrics has no pointer arguments.
    unsafe { ffi::GetSystemMetrics(0x2003) == 0 }
}

/// SM_MAXIMUMTOUCHES (95): the largest number of contacts of the digitizers; 0 without one.
#[cfg(target_os = "windows")]
fn windows_capable() -> bool {
    // SAFETY: GetSystemMetrics has no pointer arguments.
    unsafe { ffi::GetSystemMetrics(95) > 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_types_are_the_documented_values() {
        assert_eq!(source_from_device_type(0x02), Source::Mouse);
        assert_eq!(source_from_device_type(0x04), Source::Touch);
        assert_eq!(source_from_device_type(0x08), Source::Pen);
        assert_eq!(source_from_device_type(0x10), Source::Touchpad);
        assert_eq!(source_from_device_type(0x01), Source::Keyboard);
        assert_eq!(source_from_device_type(0), Source::Unknown);
        assert_eq!(source_from_device_type(0x20), Source::Unknown);
    }

    #[test]
    fn the_legacy_signature_tells_touch_from_pen() {
        assert_eq!(source_from_extra_info(0xFF51_5780u32 as isize), Source::Touch);
        assert_eq!(source_from_extra_info(0xFF51_57FFu32 as isize), Source::Touch);
        assert_eq!(source_from_extra_info(0xFF51_5700u32 as isize), Source::Pen);
        assert_eq!(source_from_extra_info(0), Source::Unknown);
        assert_eq!(source_from_extra_info(0x1234), Source::Unknown);
    }

    #[test]
    fn the_density_follows_the_choice_then_the_press_then_the_posture_and_a_digitizer_alone_is_not_enough() {
        let f = |source, slate, capable| Facts { source, slate, capable };
        assert_eq!(opening_density("touch", f(Source::Mouse, false, false)), "touch");
        assert_eq!(opening_density("standard", f(Source::Touch, true, true)), "standard");
        assert_eq!(opening_density("auto", f(Source::Touch, false, false)), "touch");
        assert_eq!(opening_density("auto", f(Source::Pen, false, true)), "touch");
        assert_eq!(opening_density("auto", f(Source::Mouse, true, true)), "standard", "a mouse press in slate posture is still a mouse");
        assert_eq!(opening_density("auto", f(Source::Unknown, true, true)), "touch");
        assert_eq!(opening_density("auto", f(Source::Unknown, false, true)), "standard", "a touch screen alone: most 2-in-1 laptops are used with a mouse");
        assert_eq!(opening_density("nonsense", f(Source::Unknown, false, false)), "standard");
    }

    #[test]
    fn facts_go_to_the_page_as_words() {
        let v = Facts { source: Source::Touch, slate: true, capable: true }.to_json();
        assert_eq!(v["source"], "touch");
        assert_eq!(v["slate"], true);
        assert_eq!(v["capable"], true);
    }
}
