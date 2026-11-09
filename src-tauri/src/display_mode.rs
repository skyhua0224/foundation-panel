//! The display modes of a display, and switching it now: what the display-mode card of 显示与视频 › 显示器 uses (the resolution and the refresh rate;
//! the scaling goes through `system::set_desktop_dpi`). The lists are what the display driver reports for the display, so the card only offers modes
//! the display supports; a display is named the way Windows names it (`\\.\DISPLAY2`, the GDI device name of the configuration's display list), none means the main display.

use log::{error, info};
use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DisplayModeEntry {
    pub width: u32,
    pub height: u32,
    /// the refresh rates (Hz) this resolution supports, ascending
    pub hz: Vec<u32>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DisplayModeNow {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct DisplayModes {
    pub device: String,
    pub current: Option<DisplayModeNow>,
    pub modes: Vec<DisplayModeEntry>,
}

/// Groups the (width, height, hz) triples a display reports into one entry per resolution, ascending, each with its rates ascending and without repeats.
pub fn group_modes(raw: impl IntoIterator<Item = (u32, u32, u32)>) -> Vec<DisplayModeEntry> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut by_size: BTreeMap<(u32, u32), BTreeSet<u32>> = BTreeMap::new();
    for (width, height, hz) in raw {
        // 0 and 1 mean "the hardware default" in a DEVMODE: not a rate to offer
        if width == 0 || height == 0 || hz <= 1 {
            continue;
        }
        by_size.entry((width, height)).or_default().insert(hz);
    }
    let mut out: Vec<DisplayModeEntry> = by_size
        .into_iter()
        .map(|((width, height), hz)| DisplayModeEntry { width, height, hz: hz.into_iter().collect() })
        .collect();
    out.sort_by_key(|m| (u64::from(m.width) * u64::from(m.height), m.width));
    out
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplaySettingsW, CDS_TEST, CDS_UPDATEREGISTRY, DEVMODEW, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
        ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE,
    };

    fn wide(device: &Option<String>) -> Option<Vec<u16>> {
        device.as_ref().filter(|d| !d.is_empty()).map(|d| d.encode_utf16().chain(std::iter::once(0)).collect())
    }

    fn name_of(wide: &Option<Vec<u16>>) -> PCWSTR {
        wide.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()))
    }

    fn blank() -> DEVMODEW {
        // SAFETY: DEVMODEW is plain data; all zeroes is the state the API expects before dmSize is set
        let mut dm: DEVMODEW = unsafe { std::mem::zeroed() };
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        dm
    }

    pub fn list(device: Option<String>) -> Result<DisplayModes, String> {
        let name = wide(&device);
        let mut raw = Vec::new();
        let mut index = 0u32;
        loop {
            let mut dm = blank();
            // SAFETY: `name` outlives the call; `dm` is a valid DEVMODEW with dmSize set
            let ok = unsafe { EnumDisplaySettingsW(name_of(&name), ENUM_DISPLAY_SETTINGS_MODE(index), &mut dm) };
            if !ok.as_bool() {
                break;
            }
            raw.push((dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency));
            index += 1;
            if index > 8192 {
                break;
            }
        }
        if raw.is_empty() {
            return Err("the display reports no modes".to_string());
        }
        let mut now = blank();
        // SAFETY: as above
        let have_now = unsafe { EnumDisplaySettingsW(name_of(&name), ENUM_CURRENT_SETTINGS, &mut now) }.as_bool();
        Ok(DisplayModes {
            device: device.unwrap_or_default(),
            current: have_now.then_some(DisplayModeNow { width: now.dmPelsWidth, height: now.dmPelsHeight, hz: now.dmDisplayFrequency }),
            modes: group_modes(raw),
        })
    }

    pub fn set(device: Option<String>, width: u32, height: u32, hz: u32) -> Result<(), String> {
        let name = wide(&device);
        let mut dm = blank();
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        dm.dmDisplayFrequency = hz;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        // SAFETY: `name` and `dm` outlive the calls
        let test = unsafe { ChangeDisplaySettingsExW(name_of(&name), Some(&dm), None, CDS_TEST, None) };
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(format!("the display does not support {width}×{height} at {hz} Hz (code {})", test.0));
        }
        // SAFETY: as above
        let done = unsafe { ChangeDisplaySettingsExW(name_of(&name), Some(&dm), None, CDS_UPDATEREGISTRY, None) };
        if done != DISP_CHANGE_SUCCESSFUL {
            return Err(format!("Windows could not switch the display (code {})", done.0));
        }
        Ok(())
    }
}

/// The modes the display supports and the one it is in now.
#[tauri::command]
pub async fn list_display_modes(device: Option<String>) -> Result<DisplayModes, String> {
    #[cfg(target_os = "windows")]
    {
        win::list(device)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = device;
        Err("changing the display mode is available on Windows only".to_string())
    }
}

/// Switches the display to a mode it supports, now. The mode is tested first, so a mode the display refuses changes nothing.
#[tauri::command]
pub async fn set_display_mode(device: Option<String>, width: u32, height: u32, hz: u32) -> Result<(), String> {
    info!("🖥️ display mode: {}×{} @ {} Hz on {:?}", width, height, hz, device);
    #[cfg(target_os = "windows")]
    {
        win::set(device, width, height, hz).map_err(|e| {
            error!("❌ display mode: {e}");
            e
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (device, width, height, hz);
        Err("changing the display mode is available on Windows only".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_are_grouped_by_resolution_ascending_without_repeats() {
        let raw = [(2560, 1440, 144), (1920, 1080, 60), (2560, 1440, 60), (1920, 1080, 60), (1920, 1080, 144), (3840, 2160, 60), (800, 600, 0), (0, 0, 60), (1280, 720, 1)];
        let grouped = group_modes(raw);
        assert_eq!(grouped.iter().map(|m| (m.width, m.height)).collect::<Vec<_>>(), vec![(1920, 1080), (2560, 1440), (3840, 2160)]);
        assert_eq!(grouped[0].hz, vec![60, 144]);
        assert_eq!(grouped[1].hz, vec![60, 144]);
        assert_eq!(grouped[2].hz, vec![60]);
    }

    #[test]
    fn nothing_to_group_is_nothing() {
        assert!(group_modes([]).is_empty());
    }
}
