//! WebView2 availability check performed before Tauri creates a webview.
//!
//! The registry paths and the bootstrapper guidance follow
//! `panel-ms-webview2-distribution-20261109`, section “Detect if a WebView2
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

    paths.iter().any(|path| {
        machine
            .open_subkey(path)
            .ok()
            .and_then(|key| key.get_value::<String, _>("pv").ok())
            .is_some_and(|version| version_is_installed(&version))
            || user
                .open_subkey(path)
                .ok()
                .and_then(|key| key.get_value::<String, _>("pv").ok())
                .is_some_and(|version| version_is_installed(&version))
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

    let chinese = unsafe { GetUserDefaultUILanguage() } & 0x03ff == 0x0004;
    let (title, instruction, content, download, web, close) = if chinese {
        (
            "需要 Microsoft Edge WebView2 Runtime",
            "管理界面需要 Microsoft Edge WebView2 Runtime",
            "当前系统没有可用的 WebView2 Runtime。请安装 Runtime，或使用网页管理界面。",
            "下载并安装",
            "打开网页管理",
            "关闭",
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
}
