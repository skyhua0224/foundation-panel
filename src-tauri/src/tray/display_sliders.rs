//! The sliders of the tray's quick panel (V3): resolution, refresh rate, scale, bitrate cap.
//!
//! The menu page draws what `state.sliders` says and sends back `tray_popup_set(token, key, value)` when a slider is released (popup_input.rs `set`). Where each value goes:
//!   resolution, refresh  Win32 on the TARGET display: EnumDisplaySettingsExW lists its modes, ChangeDisplaySettingsExW (CDS_TEST first, then CDS_UPDATEREGISTRY) sets one. The Panel had no
//!                        call for this, and the host has no endpoint for it (its display_device module changes modes only for a stream's start and end), so it is done here, in the Panel.
//!   scale                `system::set_desktop_dpi` (tools/SetDpi.exe, 100-300 %): the call of the 桌面缩放 section. It takes no monitor, so it changes the PRIMARY display; the slider is offered
//!                        only when the target is the primary display.
//!   bitrate cap          `max_bitrate` (Kbps, 0 = 自动) in the host's configuration: the same read-modify-write as `save_microphone_backend` (DEVICE_CONFIG_LOCK, vdd::read_full_sunshine_config,
//!                        sunshine::post_sunshine_config), because the config API replaces the whole table. It applies to the next connection (the host's own table calls it a client setting).
//!
//! The target: the display that is being streamed; when nothing is streamed, the virtual display (基地虚拟显示器) if it exists; else none, and the display sliders are not drawn.
//! A stream's display is the virtual one when `output_name` is the virtual display's; else the physical display whose current mode is the stream's, else the primary one.

use serde_json::{Value, json};

/// The monitor name of the virtual display's EDID (foundation-sunshine globals.cpp `ZAKO_NAME`): how its Win32 monitor is told from a real one.
pub(super) const VIRTUAL_MONITOR_NAME: &str = "Zako HDR";
/// The smallest resolution the slider offers (the lists of physical monitors carry modes nobody streams).
const MIN_WIDTH: u32 = 800;
const MIN_HEIGHT: u32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

#[derive(Clone, Debug)]
pub(super) struct Output {
    /// GDI name, `\\.\DISPLAY3`
    pub device: String,
    /// what to call it: the monitor's name
    pub name: String,
    pub virtual_display: bool,
    pub primary: bool,
    pub current: Mode,
    /// every mode the display lists (any order, duplicates allowed)
    pub modes: Vec<Mode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// the display of a running stream
    Stream,
    /// the virtual display, nothing streams
    Virtual,
}

/// What decides the target.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Facts {
    pub virtual_active: bool,
    /// `output_name` of the configuration is the virtual display's
    pub output_is_virtual: bool,
    /// width, height, fps of the first stream
    pub stream: Option<(u32, u32, u32)>,
}

/// The target display and why it is the target, or none.
pub(super) fn choose(outputs: &[Output], facts: &Facts) -> Option<(usize, Kind)> {
    let virtual_at = outputs.iter().position(|o| o.virtual_display);
    match facts.stream {
        Some((w, h, fps)) => {
            if facts.virtual_active && facts.output_is_virtual {
                return virtual_at.map(|i| (i, Kind::Stream));
            }
            let physical = || outputs.iter().enumerate().filter(|(_, o)| !o.virtual_display);
            // the display that has the stream's mode now (the host sets it for the stream), the nearest rate first; else the primary one
            let mut same: Vec<(usize, u32)> = physical().filter(|(_, o)| o.current.width == w && o.current.height == h).map(|(i, o)| (i, o.current.hz.abs_diff(fps))).collect();
            same.sort_by_key(|(_, d)| *d);
            same.first().map(|(i, _)| *i).or_else(|| physical().find(|(_, o)| o.primary).map(|(i, _)| i)).or_else(|| physical().next().map(|(i, _)| i)).map(|i| (i, Kind::Stream))
        }
        None if facts.virtual_active => virtual_at.map(|i| (i, Kind::Virtual)),
        None => None,
    }
}

/// "2560x1440" -> (2560, 1440)
pub(super) fn parse_resolution(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once(['x', 'X', '×'])?;
    let (w, h) = (w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?);
    (w >= 320 && h >= 320 && w <= 16384 && h <= 16384).then_some((w, h))
}

/// The resolutions the slider offers: every distinct size of the display's modes (at least 800x600), plus the one in use, smallest first.
pub(super) fn resolution_options(output: &Output) -> Vec<(u32, u32)> {
    let mut list: Vec<(u32, u32)> = output.modes.iter().filter(|m| m.width >= MIN_WIDTH && m.height >= MIN_HEIGHT).map(|m| (m.width, m.height)).collect();
    list.push((output.current.width, output.current.height));
    list.sort_by_key(|(w, h)| (u64::from(*w) * u64::from(*h), *w));
    list.dedup();
    list
}

/// The refresh rates the display has at its current resolution (plus the one in use), lowest first.
pub(super) fn refresh_options(output: &Output) -> Vec<u32> {
    let mut list: Vec<u32> = output.modes.iter().filter(|m| m.width == output.current.width && m.height == output.current.height && m.hz > 1).map(|m| m.hz).collect();
    list.push(output.current.hz);
    list.sort_unstable();
    list.dedup();
    list
}

/// The mode to ask for when the person picks a resolution (`want_hz` None) or a refresh rate (`want_size` None). A new resolution keeps the refresh rate when the display has it at that size, else
/// takes the nearest one it has (the higher on a tie). None when the display has no such mode.
pub(super) fn pick_mode(output: &Output, want_size: Option<(u32, u32)>, want_hz: Option<u32>) -> Option<Mode> {
    let (w, h) = want_size.unwrap_or((output.current.width, output.current.height));
    let rates: Vec<u32> = output.modes.iter().filter(|m| m.width == w && m.height == h).map(|m| m.hz).collect();
    let hz = match want_hz {
        Some(hz) => rates.contains(&hz).then_some(hz)?,
        None => *rates.iter().min_by_key(|r| (r.abs_diff(output.current.hz), u32::MAX - **r))?,
    };
    Some(Mode { width: w, height: h, hz })
}

fn mode_text(m: Mode) -> String {
    format!("{}×{}@{}", m.width, m.height, m.hz)
}

/// The `sliders` of the menu state. `dpi` is the scale in percent (None: unknown), `bitrate_kbps` the cap (None: the host is not connected, no slider).
pub(super) fn sliders_json(outputs: &[Output], target: Option<(usize, Kind)>, dpi: Option<u32>, bitrate_kbps: Option<u64>) -> Value {
    let bitrate = bitrate_kbps.map(|kbps| json!({ "kbps": kbps }));
    let Some((at, kind)) = target.filter(|(i, _)| *i < outputs.len()) else {
        return json!({ "target": Value::Null, "resolution": Value::Null, "refresh": Value::Null, "dpi": Value::Null, "bitrate": bitrate });
    };
    let o = &outputs[at];
    // the virtual display has no name here: the menu says 基地虚拟显示器 in the person's language
    let name = if o.virtual_display { String::new() } else { o.name.clone() };
    let size = |(w, h): (u32, u32)| format!("{w}x{h}");
    // the scale is the primary display's (SetDpi.exe takes no monitor here)
    let scale = o.primary.then_some(dpi).flatten().map(|v| json!({ "value": v }));
    json!({
        "target": { "kind": if kind == Kind::Stream { "stream" } else { "vdd" }, "name": name, "mode": mode_text(o.current), "dpiNote": !o.primary },
        "resolution": { "options": resolution_options(o).into_iter().map(size).collect::<Vec<_>>(), "value": size((o.current.width, o.current.height)) },
        "refresh": { "options": refresh_options(o), "value": o.current.hz },
        "dpi": scale,
        "bitrate": bitrate,
    })
}

/// What Windows answers to a mode change, in words.
pub(super) fn change_words(code: i32) -> String {
    match code {
        1 => "要重新启动电脑才能生效".to_string(),
        -1 => "显示驱动没有应用这个模式".to_string(),
        -2 => "显示器不支持这个模式".to_string(),
        -3 => "没有写入显示设置".to_string(),
        -4 | -5 => "模式参数无效".to_string(),
        -6 => "当前显示器配置下不能改这个模式".to_string(),
        other => format!("Windows 返回 {other}"),
    }
}

// ---- Windows: list the displays and change a mode ---------------------------------------------------------------------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod win {
    use super::{Mode, Output, VIRTUAL_MONITOR_NAME, change_words};
    use windows::Win32::Graphics::Gdi::{
        CDS_TEST, CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE_SUCCESSFUL, DISPLAY_DEVICEW, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
        ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE, EnumDisplayDevicesW, EnumDisplaySettingsExW,
    };
    use windows::core::PCWSTR;

    const ATTACHED_TO_DESKTOP: u32 = 0x1;
    const PRIMARY_DEVICE: u32 = 0x4;

    fn text(wide: &[u16]) -> String {
        let end = wide.iter().position(|c| *c == 0).unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..end])
    }
    fn nul_terminated(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    fn devmode() -> DEVMODEW {
        DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() }
    }
    fn mode_of(dm: &DEVMODEW) -> Mode {
        Mode { width: dm.dmPelsWidth, height: dm.dmPelsHeight, hz: dm.dmDisplayFrequency }
    }

    /// The displays attached to the desktop, with the modes each lists (32 bit only: one entry per size and rate).
    pub(in crate::tray) fn outputs() -> Vec<Output> {
        let mut list = Vec::new();
        let mut index = 0u32;
        loop {
            let mut adapter = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            // SAFETY: `adapter` is a valid DISPLAY_DEVICEW with `cb` set; a null device name lists the adapters.
            if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), index, &mut adapter, 0) }.as_bool() {
                break;
            }
            index += 1;
            if adapter.StateFlags.0 & ATTACHED_TO_DESKTOP == 0 {
                continue;
            }
            let device = text(&adapter.DeviceName);
            let mut monitor = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            // SAFETY: `adapter.DeviceName` is NUL terminated; `monitor` is valid.
            let has_monitor = unsafe { EnumDisplayDevicesW(PCWSTR(adapter.DeviceName.as_ptr()), 0, &mut monitor, 0) }.as_bool();
            let name = if has_monitor && !text(&monitor.DeviceString).is_empty() { text(&monitor.DeviceString) } else { text(&adapter.DeviceString) };
            let wide = nul_terminated(&device);
            let mut now = devmode();
            // SAFETY: `wide` is NUL terminated; `now` is a valid DEVMODEW with `dmSize` set.
            if !unsafe { EnumDisplaySettingsExW(PCWSTR(wide.as_ptr()), ENUM_CURRENT_SETTINGS, &mut now, ENUM_DISPLAY_SETTINGS_FLAGS(0)) }.as_bool() {
                continue;
            }
            let mut modes = Vec::new();
            let mut n = 0u32;
            loop {
                let mut dm = devmode();
                // SAFETY: as above.
                if !unsafe { EnumDisplaySettingsExW(PCWSTR(wide.as_ptr()), ENUM_DISPLAY_SETTINGS_MODE(n), &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0)) }.as_bool() {
                    break;
                }
                n += 1;
                if dm.dmBitsPerPel == 32 {
                    modes.push(mode_of(&dm));
                }
                if n > 4096 {
                    break;
                }
            }
            modes.sort();
            modes.dedup();
            list.push(Output {
                device,
                virtual_display: name.eq_ignore_ascii_case(VIRTUAL_MONITOR_NAME) || name.to_ascii_lowercase().contains("zako"),
                name,
                primary: adapter.StateFlags.0 & PRIMARY_DEVICE != 0,
                current: mode_of(&now),
                modes,
            });
        }
        list
    }

    /// Sets a mode of a display (checked first with CDS_TEST, then written like the Settings app does).
    pub(in crate::tray) fn set_mode(device: &str, mode: Mode) -> Result<(), String> {
        let wide = nul_terminated(device);
        let mut dm = devmode();
        // SAFETY: as in `outputs`.
        if !unsafe { EnumDisplaySettingsExW(PCWSTR(wide.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0)) }.as_bool() {
            return Err("无法读取显示器当前模式".to_string());
        }
        dm.dmPelsWidth = mode.width;
        dm.dmPelsHeight = mode.height;
        dm.dmDisplayFrequency = mode.hz;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        // SAFETY: `wide` is NUL terminated, `dm` is a valid DEVMODEW; no window, no parameter.
        let test = unsafe { ChangeDisplaySettingsExW(PCWSTR(wide.as_ptr()), Some(&dm), None, CDS_TEST, None) };
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(change_words(test.0));
        }
        // SAFETY: as above.
        let done = unsafe { ChangeDisplaySettingsExW(PCWSTR(wide.as_ptr()), Some(&dm), None, CDS_UPDATEREGISTRY, None) };
        if done != DISP_CHANGE_SUCCESSFUL {
            return Err(change_words(done.0));
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub(super) use win::{outputs, set_mode};

#[cfg(not(target_os = "windows"))]
pub(super) fn outputs() -> Vec<Output> {
    Vec::new()
}
#[cfg(not(target_os = "windows"))]
pub(super) fn set_mode(_device: &str, _mode: Mode) -> Result<(), String> {
    Err("只有 Windows 能改显示器模式".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(width: u32, height: u32, hz: u32) -> Mode {
        Mode { width, height, hz }
    }
    fn output(name: &str, virtual_display: bool, primary: bool, current: Mode, modes: &[Mode]) -> Output {
        Output { device: format!("\\\\.\\{name}"), name: name.to_string(), virtual_display, primary, current, modes: modes.to_vec() }
    }
    fn desk() -> Vec<Output> {
        vec![
            output("DISPLAY1", false, true, mode(2560, 1440, 144), &[mode(1920, 1080, 60), mode(1920, 1080, 144), mode(2560, 1440, 60), mode(2560, 1440, 144), mode(640, 480, 60)]),
            output("DISPLAY2", false, false, mode(1920, 1080, 60), &[mode(1920, 1080, 60), mode(1280, 720, 60)]),
            output("DISPLAY3", true, false, mode(2560, 1440, 120), &[mode(1920, 1080, 60), mode(2560, 1440, 60), mode(2560, 1440, 120), mode(3840, 2160, 60)]),
        ]
    }

    #[test]
    fn the_target_is_the_display_that_is_streamed_else_the_virtual_one_else_none() {
        let outputs = desk();
        // nothing streams, the virtual display exists
        assert_eq!(choose(&outputs, &Facts { virtual_active: true, ..Facts::default() }), Some((2, Kind::Virtual)));
        // nothing streams, no virtual display: no target
        assert_eq!(choose(&outputs, &Facts::default()), None);
        // a stream of the virtual display
        assert_eq!(choose(&outputs, &Facts { virtual_active: true, output_is_virtual: true, stream: Some((2560, 1440, 120)) }), Some((2, Kind::Stream)));
        // a stream of a physical display: the one with the stream's mode, the nearest rate first
        assert_eq!(choose(&outputs, &Facts { virtual_active: true, output_is_virtual: false, stream: Some((1920, 1080, 60)) }), Some((1, Kind::Stream)));
        assert_eq!(choose(&outputs, &Facts { stream: Some((2560, 1440, 120)), ..Facts::default() }), Some((0, Kind::Stream)));
        // none has the stream's mode: the primary display
        assert_eq!(choose(&outputs, &Facts { stream: Some((3440, 1440, 60)), ..Facts::default() }), Some((0, Kind::Stream)));
        // the virtual display is the output but the system lists none: no target
        assert_eq!(choose(&outputs[..2], &Facts { virtual_active: true, output_is_virtual: true, stream: Some((1, 1, 1)) }), None);
    }

    #[test]
    fn the_resolution_slider_offers_the_sizes_the_display_lists_smallest_first() {
        let outputs = desk();
        assert_eq!(resolution_options(&outputs[0]), vec![(1920, 1080), (2560, 1440)], "640x480 is not offered");
        assert_eq!(resolution_options(&outputs[2]), vec![(1920, 1080), (2560, 1440), (3840, 2160)]);
        // a size in use that the list lacks is still offered
        let odd = output("X", false, true, mode(3440, 1440, 100), &[mode(1920, 1080, 60)]);
        assert_eq!(resolution_options(&odd), vec![(1920, 1080), (3440, 1440)]);
    }

    #[test]
    fn the_refresh_slider_offers_the_rates_at_the_current_size() {
        let outputs = desk();
        assert_eq!(refresh_options(&outputs[0]), vec![60, 144]);
        assert_eq!(refresh_options(&outputs[2]), vec![60, 120]);
    }

    #[test]
    fn a_new_resolution_keeps_the_rate_or_takes_the_nearest_and_an_unknown_mode_is_none() {
        let outputs = desk();
        assert_eq!(pick_mode(&outputs[0], Some((1920, 1080)), None), Some(mode(1920, 1080, 144)));
        assert_eq!(pick_mode(&outputs[2], Some((3840, 2160)), None), Some(mode(3840, 2160, 60)), "only 60 Hz at 4K: that one");
        assert_eq!(pick_mode(&outputs[2], Some((1920, 1080)), None), Some(mode(1920, 1080, 60)));
        assert_eq!(pick_mode(&outputs[0], None, Some(60)), Some(mode(2560, 1440, 60)));
        assert_eq!(pick_mode(&outputs[0], None, Some(75)), None, "the display has no 75 Hz");
        assert_eq!(pick_mode(&outputs[0], Some((1234, 567)), None), None);
        // a tie goes to the higher rate
        let tie = output("T", false, true, mode(1920, 1080, 100), &[mode(1920, 1080, 60), mode(1920, 1080, 144), mode(2560, 1440, 90), mode(2560, 1440, 110)]);
        assert_eq!(pick_mode(&tie, Some((2560, 1440)), None), Some(mode(2560, 1440, 110)));
    }

    #[test]
    fn the_resolution_text_is_what_the_menu_sends_back() {
        assert_eq!(parse_resolution("2560x1440"), Some((2560, 1440)));
        assert_eq!(parse_resolution("2560×1440"), Some((2560, 1440)));
        assert_eq!(parse_resolution("2560"), None);
        assert_eq!(parse_resolution("0x0"), None);
        assert_eq!(parse_resolution("99999x1080"), None);
        assert_eq!(parse_resolution("axb"), None);
    }

    #[test]
    fn the_slider_state_names_the_target_and_hides_what_has_no_target() {
        let outputs = desk();
        let none = sliders_json(&outputs, None, Some(150), Some(0));
        assert!(none["target"].is_null() && none["resolution"].is_null() && none["refresh"].is_null() && none["dpi"].is_null());
        assert_eq!(none["bitrate"]["kbps"], 0, "the bitrate cap needs no display");
        let offline = sliders_json(&outputs, None, None, None);
        assert!(offline["bitrate"].is_null(), "no connection, no bitrate slider");

        let vdd = sliders_json(&outputs, Some((2, Kind::Virtual)), Some(150), Some(80000));
        assert_eq!(vdd["target"]["kind"], "vdd");
        assert_eq!(vdd["target"]["name"], "", "the menu names the virtual display itself");
        assert_eq!(vdd["target"]["mode"], "2560×1440@120");
        assert_eq!(vdd["target"]["dpiNote"], true);
        assert!(vdd["dpi"].is_null(), "SetDpi.exe changes the primary display only");
        assert_eq!(vdd["resolution"]["value"], "2560x1440");
        assert_eq!(vdd["resolution"]["options"], json!(["1920x1080", "2560x1440", "3840x2160"]));
        assert_eq!(vdd["refresh"]["value"], 120);
        assert_eq!(vdd["bitrate"]["kbps"], 80000);

        let primary = sliders_json(&outputs, Some((0, Kind::Stream)), Some(150), Some(0));
        assert_eq!(primary["target"]["kind"], "stream");
        assert_eq!(primary["target"]["name"], "DISPLAY1");
        assert_eq!(primary["dpi"]["value"], 150);
        assert_eq!(primary["target"]["dpiNote"], false);
    }

    #[test]
    fn windows_answers_are_put_into_words() {
        assert_eq!(change_words(-2), "显示器不支持这个模式");
        assert!(change_words(1).contains("重新启动"));
        assert_eq!(change_words(-99), "Windows 返回 -99");
    }
}
