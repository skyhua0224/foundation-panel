//! An install that lost files: an update that failed halfway, a cleaning tool, a restore from a
//! backup. The Web UI folder (`assets\web`) is the case that shows worst: the host then answers
//! with its built-in English sign-in page and the pages inside the Control Panel stay empty.
//!
//! The Control Panel looks for the folder a little after it started and, when it is still missing a
//! minute later (an update that is running moves it aside for a moment), asks to repair: the
//! installer of the newest release is downloaded and run over the installed copy; settings and
//! paired devices stay.
use std::path::Path;

/// The pages the host serves first. Without them the Web UI cannot show anything.
const REQUIRED_FILES: &[&str] = &["assets\\web\\index.html", "assets\\web\\login.html"];

/// What an install folder lacks. Empty for a folder that is not an install (no `sunshine.exe`):
/// a development tree or a portable copy is not asked to repair itself.
pub(crate) fn missing_files(install_dir: &Path) -> Vec<&'static str> {
    if !install_dir.join("sunshine.exe").is_file() {
        return Vec::new();
    }
    REQUIRED_FILES
        .iter()
        .copied()
        .filter(|relative| !install_dir.join(relative).is_file())
        .collect()
}

#[tauri::command]
pub fn install_integrity_status() -> Vec<String> {
    missing_files(&crate::sunshine::install_dir())
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// The words of the question, in the language of the person's Windows.
fn repair_question(chinese: bool, missing: &[&str]) -> (&'static str, String) {
    let list = missing.join("\n");
    if chinese {
        (
            "Sunshine 安装不完整",
            format!(
                "Sunshine 的安装目录缺少文件，网页管理界面和面板里的内嵌页面无法显示。\n\n缺少：\n{list}\n\n现在修复吗？会下载最新版安装程序并重新安装，设置和已配对的设备会保留。"
            ),
        )
    } else {
        (
            "Sunshine installation is incomplete",
            format!(
                "Files are missing from the Sunshine folder, so the web interface and the pages inside the Control Panel cannot be shown.\n\nMissing:\n{list}\n\nRepair now? The newest installer is downloaded and run over the installed copy; settings and paired devices stay."
            ),
        )
    }
}

#[cfg(target_os = "windows")]
fn ask_to_repair(missing: &[&str]) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    use windows::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_ICONWARNING, MB_SETFOREGROUND, MB_YESNO, MessageBoxW,
    };
    use windows::core::PCWSTR;

    // the primary language id (low ten bits) of Chinese is 0x04
    let chinese = unsafe { GetUserDefaultUILanguage() } & 0x3ff == 0x04;
    let (title, body) = repair_question(chinese, missing);
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let body: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    let answer = unsafe {
        MessageBoxW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONWARNING | MB_SETFOREGROUND,
        )
    };
    answer == IDYES
}

/// Check the install some time after the start, twice, and ask once.
pub fn start(app: tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    std::thread::spawn(move || {
        use log::{info, warn};
        use std::time::Duration;

        std::thread::sleep(Duration::from_secs(25));
        let install_dir = crate::sunshine::install_dir();
        if missing_files(&install_dir).is_empty() {
            return;
        }
        // an update in progress moves the folder aside for a moment
        std::thread::sleep(Duration::from_secs(75));
        let missing = missing_files(&install_dir);
        if missing.is_empty() {
            return;
        }
        warn!("安装不完整，缺少: {}", missing.join(", "));
        if !ask_to_repair(&missing) {
            info!("安装不完整，用户暂不修复");
            return;
        }
        tauri::async_runtime::spawn(async move {
            if let Err(error) = crate::update::repair_installation(app).await {
                warn!("修复安装失败: {error}");
            }
        });
    });
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install_with(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for relative in files {
            let path = dir.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"x").unwrap();
        }
        dir
    }

    #[test]
    fn a_complete_install_lacks_nothing() {
        let dir = install_with(&["sunshine.exe", "assets\\web\\index.html", "assets\\web\\login.html"]);
        assert!(missing_files(dir.path()).is_empty());
    }

    #[test]
    fn a_missing_web_folder_is_named() {
        let dir = install_with(&["sunshine.exe", "assets\\gui\\sunshine-gui.exe"]);
        assert_eq!(
            missing_files(dir.path()),
            vec!["assets\\web\\index.html", "assets\\web\\login.html"]
        );
    }

    #[test]
    fn one_missing_page_is_named() {
        let dir = install_with(&["sunshine.exe", "assets\\web\\index.html"]);
        assert_eq!(missing_files(dir.path()), vec!["assets\\web\\login.html"]);
    }

    #[test]
    fn a_folder_that_is_not_an_install_is_left_alone() {
        let dir = install_with(&["assets\\gui\\sunshine-gui.exe"]);
        assert!(missing_files(dir.path()).is_empty());
    }

    #[test]
    fn the_question_names_the_missing_files_in_both_languages() {
        let missing = ["assets\\web\\index.html"];
        let (title, body) = repair_question(true, &missing);
        assert!(title.contains("安装不完整") && body.contains("assets\\web\\index.html"));
        let (title, body) = repair_question(false, &missing);
        assert!(title.contains("incomplete") && body.contains("assets\\web\\index.html"));
    }
}
