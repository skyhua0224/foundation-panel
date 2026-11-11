//! Is the game's window up? The search the launch splash ends on.
//!
//! A game's main window is a top-level, visible, unowned window of a process of the app's tree: the process the host started, the game's own processes the
//! launcher reported, and everything below them. Windows of the splash itself, tool windows, cloaked ones (other virtual desktops, suspended apps), tiny
//! ones and the consoles of a launcher do not count. When the tree has no live process (a launcher that started the game and left, Steam) a large window that
//! was not there when the launch began counts instead; a resumed app has no "before", so for it only the tree counts.
//!
//! The decision is pure (`game_window_present`); `snapshot_windows` and `snapshot_processes` are the thin Win32 readers.

use std::collections::{HashMap, HashSet};

/// A window smaller than this is a dialog or a tool, not a game.
pub const MIN_WIDTH: i32 = 320;
pub const MIN_HEIGHT: i32 = 200;
/// Without a live process of the tree, a window must cover at least this share of the display (percent).
pub const FALLBACK_COVER_PERCENT: i64 = 60;

/// Window classes that are never the game: the consoles of a launcher and input-method windows.
const NEVER: &[&str] = &["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS", "IME", "MSCTFIME UI", "Default IME", "Progman", "WorkerW", "Shell_TrayWnd"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// the window handle, only to tell windows apart
    pub handle: isize,
    pub pid: u32,
    pub visible: bool,
    pub cloaked: bool,
    pub tool_window: bool,
    pub owned: bool,
    pub width: i32,
    pub height: i32,
    pub class: String,
}

/// Every process of the tree: the roots and all their descendants, from a pid -> parent pid table. A process that is not in the table (it ended) still counts as
/// a root, so a child that outlived it is found.
pub fn tree_of(roots: &[u32], parents: &HashMap<u32, u32>) -> HashSet<u32> {
    let mut tree: HashSet<u32> = roots.iter().copied().filter(|pid| *pid != 0).collect();
    loop {
        let before = tree.len();
        for (pid, parent) in parents {
            if tree.contains(parent) && *pid != 0 {
                tree.insert(*pid);
            }
        }
        if tree.len() == before {
            return tree;
        }
    }
}

fn is_candidate(window: &WindowInfo, own_pid: u32) -> bool {
    window.visible
        && !window.cloaked
        && !window.tool_window
        && !window.owned
        && window.pid != own_pid
        && window.width >= MIN_WIDTH
        && window.height >= MIN_HEIGHT
        && !NEVER.contains(&window.class.as_str())
}

/// `windows`: every top-level window now. `tree`: the app's processes. `alive`: which of them still run. `before`: the handles of the windows that were up when
/// the launch began (None for a resume). `display_area`: the area of the display the splash is on, in pixels.
pub fn game_window_present(
    windows: &[WindowInfo],
    own_pid: u32,
    tree: &HashSet<u32>,
    any_alive: bool,
    before: Option<&HashSet<isize>>,
    display_area: i64,
) -> bool {
    if windows.iter().any(|w| tree.contains(&w.pid) && is_candidate(w, own_pid)) {
        return true;
    }
    if any_alive {
        return false;
    }
    let Some(before) = before else { return false };
    windows.iter().any(|w| {
        is_candidate(w, own_pid)
            && !before.contains(&w.handle)
            && display_area > 0
            && (w.width as i64) * (w.height as i64) * 100 >= display_area * FALLBACK_COVER_PERCENT
    })
}

#[cfg(target_os = "windows")]
pub use win::{snapshot_processes, snapshot_windows};

#[cfg(target_os = "windows")]
mod win {
    use super::WindowInfo;
    use std::collections::HashMap;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetWindow, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
        WS_EX_TOOLWINDOW,
    };

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
        let mut cloaked = 0u32;
        let _ = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, std::mem::size_of::<u32>() as u32) };
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
        let owned = unsafe { GetWindow(hwnd, GW_OWNER) }.map(|owner| !owner.0.is_null()).unwrap_or(false);
        let mut rect = RECT::default();
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        let mut class = [0u16; 128];
        let length = unsafe { GetClassNameW(hwnd, &mut class) }.max(0) as usize;
        out.push(WindowInfo {
            handle: hwnd.0 as isize,
            pid,
            visible,
            cloaked: cloaked != 0,
            tool_window: ex_style & WS_EX_TOOLWINDOW.0 != 0,
            owned,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            class: String::from_utf16_lossy(&class[..length.min(class.len())]),
        });
        TRUE
    }

    /// Every top-level window of the interactive desktop this process runs on.
    pub fn snapshot_windows() -> Vec<WindowInfo> {
        let mut out: Vec<WindowInfo> = Vec::new();
        let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut out as *mut Vec<WindowInfo> as isize)) };
        out
    }

    /// pid -> parent pid of every process.
    pub fn snapshot_processes() -> HashMap<u32, u32> {
        let mut table = HashMap::new();
        let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else { return table };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
            loop {
                table.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                    break;
                }
            }
        }
        let _ = unsafe { CloseHandle(snapshot) };
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(handle: isize, pid: u32, width: i32, height: i32) -> WindowInfo {
        WindowInfo { handle, pid, visible: true, cloaked: false, tool_window: false, owned: false, width, height, class: "UnityWndClass".to_string() }
    }

    fn set(pids: &[u32]) -> HashSet<u32> {
        pids.iter().copied().collect()
    }

    #[test]
    fn the_tree_is_the_roots_and_everything_below_them() {
        let parents: HashMap<u32, u32> = [(11, 10), (12, 11), (20, 1), (13, 10)].into_iter().collect();
        assert_eq!(tree_of(&[10], &parents), set(&[10, 11, 12, 13]));
        // the root has ended and is not in the table: its child is still found
        assert_eq!(tree_of(&[99], &[(100, 99)].into_iter().collect()), set(&[99, 100]));
        assert_eq!(tree_of(&[0], &parents), set(&[]));
    }

    #[test]
    fn a_window_of_the_tree_is_the_game() {
        let tree = set(&[10, 11]);
        let windows = [window(1, 20, 1920, 1080), window(2, 11, 1280, 720)];
        assert!(game_window_present(&windows, 5, &tree, true, None, 1920 * 1080));
        assert!(!game_window_present(&windows[..1], 5, &tree, true, None, 1920 * 1080), "another process's window is not the game");
    }

    #[test]
    fn what_cannot_be_a_game_window_is_not() {
        let tree = set(&[10]);
        let base = window(1, 10, 1280, 720);
        let cases = [
            WindowInfo { visible: false, ..base.clone() },
            WindowInfo { cloaked: true, ..base.clone() },
            WindowInfo { tool_window: true, ..base.clone() },
            WindowInfo { owned: true, ..base.clone() },
            WindowInfo { width: 200, ..base.clone() },
            WindowInfo { height: 100, ..base.clone() },
            WindowInfo { class: "ConsoleWindowClass".to_string(), ..base.clone() },
            WindowInfo { pid: 5, ..base.clone() },
        ];
        for case in cases {
            assert!(!game_window_present(std::slice::from_ref(&case), 5, &tree, true, None, 0), "{case:?}");
        }
        assert!(game_window_present(std::slice::from_ref(&base), 5, &tree, true, None, 0));
    }

    #[test]
    fn the_splash_itself_never_ends_the_splash() {
        let tree = set(&[5]);
        assert!(!game_window_present(&[window(1, 5, 1920, 1080)], 5, &tree, true, None, 1920 * 1080));
    }

    #[test]
    fn a_launcher_that_left_is_found_by_the_new_large_window() {
        let tree = set(&[10]);
        let before: HashSet<isize> = [1, 2].into_iter().collect();
        let area = 1920 * 1080;
        // nothing of the tree is alive; a window that was there before does not count, a small new one does not, a large new one does
        assert!(!game_window_present(&[window(1, 77, 1920, 1080)], 5, &tree, false, Some(&before), area));
        assert!(!game_window_present(&[window(3, 77, 800, 600)], 5, &tree, false, Some(&before), area));
        assert!(game_window_present(&[window(3, 77, 1920, 1080)], 5, &tree, false, Some(&before), area));
        // a live process of the tree means the game is still to come: no guessing
        assert!(!game_window_present(&[window(3, 77, 1920, 1080)], 5, &tree, true, Some(&before), area));
        // a resume has no before
        assert!(!game_window_present(&[window(3, 77, 1920, 1080)], 5, &tree, false, None, area));
    }

    #[test]
    fn the_shell_is_not_a_game_even_when_large() {
        let before = HashSet::new();
        let mut desktop = window(9, 1, 1920, 1080);
        desktop.class = "Progman".to_string();
        assert!(!game_window_present(&[desktop], 5, &set(&[10]), false, Some(&before), 1920 * 1080));
    }
}
