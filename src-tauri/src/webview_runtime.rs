//! WebView2 availability check performed before Tauri creates a webview.
//!
//! The registry paths and the bootstrapper guidance follow
//! `panel-ms-webview2-distribution`, section “Detect if a WebView2
//! Runtime is already installed”.

#[cfg(target_os = "windows")]
const WEBVIEW2_CLIENT_ID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// The service uses this exit code to distinguish an expected missing-runtime
/// stop from a crash. It must stay in sync with the host policy.
pub const WEBVIEW2_MISSING_EXIT_CODE: i32 = 0x5352;

/// Check the runtime before any `WebviewWindowBuilder` is reached.
///
/// The environment override is intentionally test-only in effect: it lets a
/// Windows session exercise the native fallback without uninstalling the
/// machine's runtime.

/// Which copy the missing-runtime dialog uses for a Windows UI language id.
pub(crate) fn webview2_dialog_language(language_id: u16) -> &'static str {
    let primary = language_id & 0x03ff;
    let region = language_id >> 10;
    if primary == 0x0004 && matches!(region, 0x01 | 0x03 | 0x05) {
        "zh_TW"
    } else if primary == 0x0004 {
        "zh"
    } else if primary == 0x0011 {
        "ja"
    } else {
        "en"
    }
}

pub fn ensure_available() -> bool {
    if std::env::var_os("SUNSHINE_GUI_FORCE_NO_WEBVIEW2").is_some() {
        #[cfg(target_os = "windows")]
        show_missing_runtime_dialog();
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        if runtime_is_available() {
            return true;
        }
        show_missing_runtime_dialog();
        return false;
    }

    #[cfg(not(target_os = "windows"))]
    true
}

#[cfg(target_os = "windows")]
fn version_is_installed(value: &str) -> bool {
    let mut components = value.trim().split('.');
    let mut has_nonzero = false;
    let mut count = 0;
    while let Some(component) = components.next() {
        let Ok(number) = component.parse::<u64>() else {
            return false;
        };
        has_nonzero |= number != 0;
        count += 1;
    }
    count >= 3 && has_nonzero
}

#[cfg(target_os = "windows")]
fn runtime_is_available() -> bool {
    installed_version().is_some()
}

/// The version of the installed WebView2 Runtime (the `pv` value the Evergreen runtime registers), when there is one.
#[cfg(target_os = "windows")]
pub(crate) fn installed_version() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let machine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let user = RegKey::predef(HKEY_CURRENT_USER);
    let paths = [
        format!(
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{}",
            WEBVIEW2_CLIENT_ID
        ),
        format!(
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{}",
            WEBVIEW2_CLIENT_ID
        ),
    ];

    paths.iter().find_map(|path| {
        [&machine, &user].into_iter().find_map(|root| {
            root.open_subkey(path)
                .ok()
                .and_then(|key| key.get_value::<String, _>("pv").ok())
                .filter(|version| version_is_installed(version))
        })
    })
}

#[cfg(target_os = "windows")]
fn show_missing_runtime_dialog() {
    use std::path::PathBuf;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    use windows::Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{IDNO, SW_SHOWNORMAL};
    use windows::core::PCWSTR;

    const DOWNLOAD_BUTTON: i32 = 1001;
    const WEB_BUTTON: i32 = 1002;
    const CLOSE_BUTTON: i32 = 1003;
    const DOWNLOAD_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
    const WEB_URL: &str = "https://localhost:47990";

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn launch(target: &str) {
        let operation = wide("open");
        let target = wide(target);
        unsafe {
            let _ = ShellExecuteW(
                Some(HWND(std::ptr::null_mut())),
                PCWSTR(operation.as_ptr()),
                PCWSTR(target.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
    }

    fn shipped_bootstrapper() -> Option<PathBuf> {
        let executable = std::env::current_exe().ok()?;
        let directory = executable.parent()?;
        [
            "MicrosoftEdgeWebview2Setup.exe",
            "WebView2Bootstrapper.exe",
            "webview2-bootstrapper.exe",
        ]
        .iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
    }

    // LANGID: low 10 bits are the language, the rest the region.
    // 0x04 Chinese; region 0x01 Taiwan, 0x03 Hong Kong and 0x05 Macao are Traditional.
    // 0x11 Japanese. Anything else stays English.
    let language = unsafe { GetUserDefaultUILanguage() };
    let primary = language & 0x03ff;
    let region = language >> 10;
    let (title, instruction, content, download, web, close) = if primary == 0x0004 && matches!(region, 0x01 | 0x03 | 0x05) {
        (
            "需要 Microsoft Edge WebView2 Runtime",
            "管理介面需要 Microsoft Edge WebView2 Runtime",
            "目前系統沒有可用的 WebView2 Runtime。請安裝 Runtime，或使用網頁管理介面。",
            "下載並安裝",
            "開啟網頁管理",
            "關閉",
        )
    } else if primary == 0x0004 {
        (
            "需要 Microsoft Edge WebView2 Runtime",
            "管理界面需要 Microsoft Edge WebView2 Runtime",
            "当前系统没有可用的 WebView2 Runtime。请安装 Runtime，或使用网页管理界面。",
            "下载并安装",
            "打开网页管理",
            "关闭",
        )
    } else if primary == 0x0011 {
        (
            "Microsoft Edge WebView2 Runtime が必要です",
            "管理画面には Microsoft Edge WebView2 Runtime が必要です",
            "使える WebView2 Runtime がありません。Runtime をインストールするか、ウェブ管理画面を使ってください。",
            "ダウンロードしてインストール",
            "ウェブ管理を開く",
            "閉じる",
        )
    } else {
        (
            "Microsoft Edge WebView2 Runtime required",
            "The management UI needs Microsoft Edge WebView2 Runtime",
            "No usable WebView2 Runtime was found. Install the Runtime or use the web management UI.",
            "Download and install",
            "Open web management",
            "Close",
        )
    };

    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: DOWNLOAD_BUTTON,
            pszButtonText: PCWSTR(wide(download).leak().as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: WEB_BUTTON,
            pszButtonText: PCWSTR(wide(web).leak().as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: CLOSE_BUTTON,
            pszButtonText: PCWSTR(wide(close).leak().as_ptr()),
        },
    ];
    let title = wide(title);
    let instruction = wide(instruction);
    let content = wide(content);
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: PCWSTR(title.as_ptr()),
        pszMainInstruction: PCWSTR(instruction.as_ptr()),
        pszContent: PCWSTR(content.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: WEB_BUTTON,
        ..Default::default()
    };
    let mut selected = IDNO.0;
    unsafe {
        let _ = TaskDialogIndirect(&config, Some(&mut selected), None, None);
    }

    match selected {
        DOWNLOAD_BUTTON => {
            if let Some(path) = shipped_bootstrapper() {
                if let Some(path) = path.to_str() {
                    launch(path);
                }
            } else {
                launch(DOWNLOAD_URL);
            }
        }
        WEB_BUTTON => launch(WEB_URL),
        _ => {}
    }
}

/// The words of the dialog that tells the person the window cannot be shown (title, instruction, content, "open the log folder", "close").
pub(crate) fn window_failed_text(language: &str) -> [&'static str; 5] {
    match language {
        "zh" => [
            "控制面板窗口无法显示",
            "控制面板的窗口反复崩溃，已停止自动重试",
            "已重新加载页面，并改用软件渲染，窗口仍然崩溃。请把日志文件 panel.log、Windows 版本（Win+R 输入 winver）和显卡型号发给我们。",
            "打开日志文件夹",
            "关闭",
        ],
        "zh_TW" => [
            "控制面板視窗無法顯示",
            "控制面板的視窗反覆當機，已停止自動重試",
            "已重新載入頁面，並改用軟體轉譯，視窗仍然當機。請把記錄檔 panel.log、Windows 版本（Win+R 輸入 winver）和顯示卡型號傳給我們。",
            "開啟記錄檔資料夾",
            "關閉",
        ],
        "ja" => [
            "コントロールパネルのウィンドウを表示できません",
            "ウィンドウが繰り返しクラッシュしたため、自動再試行を止めました",
            "ページを再読み込みし、ソフトウェアレンダリングに切り替えても改善しませんでした。ログファイル panel.log、Windows のバージョン（Win+R で winver）、GPU の型番をお送りください。",
            "ログフォルダーを開く",
            "閉じる",
        ],
        _ => [
            "The control panel window cannot be shown",
            "The control panel window keeps crashing; automatic retries have stopped",
            "The page was reloaded and software rendering was tried, and the window still crashes. Please send us the log file panel.log, your Windows version (Win+R, winver) and your graphics card model.",
            "Open the log folder",
            "Close",
        ],
    }
}

/// Tells the person that the window cannot be shown, from another thread (the window's own thread is the one in trouble).
#[cfg(target_os = "windows")]
pub(crate) fn show_window_failed_dialog() {
    let _ = std::thread::Builder::new()
        .name("window-failed-dialog".into())
        .spawn(|| {
            use windows::Win32::Globalization::GetUserDefaultUILanguage;
            use windows::Win32::UI::Controls::{
                TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
            };
            use windows::core::PCWSTR;

            const LOGS_BUTTON: i32 = 1001;
            const CLOSE_BUTTON: i32 = 1002;
            fn wide(value: &str) -> Vec<u16> {
                value.encode_utf16().chain(std::iter::once(0)).collect()
            }

            let [title, instruction, content, logs, close] =
                window_failed_text(webview2_dialog_language(unsafe { GetUserDefaultUILanguage() }));
            let (title, instruction, content) = (wide(title), wide(instruction), wide(content));
            let (logs, close) = (wide(logs), wide(close));
            let buttons = [
                TASKDIALOG_BUTTON { nButtonID: LOGS_BUTTON, pszButtonText: PCWSTR(logs.as_ptr()) },
                TASKDIALOG_BUTTON { nButtonID: CLOSE_BUTTON, pszButtonText: PCWSTR(close.as_ptr()) },
            ];
            let config = TASKDIALOGCONFIG {
                cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
                dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
                pszWindowTitle: PCWSTR(title.as_ptr()),
                pszMainInstruction: PCWSTR(instruction.as_ptr()),
                pszContent: PCWSTR(content.as_ptr()),
                cButtons: buttons.len() as u32,
                pButtons: buttons.as_ptr(),
                nDefaultButton: LOGS_BUTTON,
                ..Default::default()
            };
            let mut selected = 0;
            let _ = unsafe { TaskDialogIndirect(&config, Some(&mut selected), None, None) };
            if selected == LOGS_BUTTON {
                if let Some(directory) = crate::logger::panel_log_path().parent() {
                    let _ = std::process::Command::new("explorer.exe").arg(directory).spawn();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::version_is_installed;

    #[cfg(target_os = "windows")]
    #[test]
    fn registry_version_requires_a_nonzero_version() {
        assert!(!version_is_installed("0.0.0.0"));
        assert!(!version_is_installed(""));
        assert!(!version_is_installed("not-a-version"));
        assert!(version_is_installed("128.0.2739.42"));
    }

    #[test]
    fn the_window_failed_dialog_has_every_line_in_every_language() {
        for language in ["zh", "zh_TW", "ja", "en", "anything else"] {
            let text = super::window_failed_text(language);
            assert!(text.iter().all(|line| !line.is_empty()), "{language}");
        }
        assert!(super::window_failed_text("en")[2].contains("panel.log"));
    }

    #[test]
    fn dialog_language_follows_the_ui_language_id() {
        assert_eq!(super::webview2_dialog_language(0x0804), "zh");
        assert_eq!(super::webview2_dialog_language(0x0404), "zh_TW");
        assert_eq!(super::webview2_dialog_language(0x0c04), "zh_TW");
        assert_eq!(super::webview2_dialog_language(0x0411), "ja");
        assert_eq!(super::webview2_dialog_language(0x0409), "en");
        assert_eq!(super::webview2_dialog_language(0x0407), "en");
    }

}
