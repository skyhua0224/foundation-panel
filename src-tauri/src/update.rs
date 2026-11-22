use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use sha2::Digest;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::github_download::{
    self, DEFAULT_CONNECT_TIMEOUT, DEFAULT_IDLE_TIMEOUT, DEFAULT_RESPONSE_TIMEOUT,
    DownloadAttemptPhase, DownloadRequest,
};

mod cdk_store;
mod mirrorchyan;
mod metadata;
mod preferences;

pub(crate) use preferences::{include_prerelease as get_include_prerelease, save_last_check_time};

use metadata::{
    ReleaseAsset, ReleaseCandidate, ReleaseCatalog, UpToDateNote, UpdateChannel, candidate_is_newer_than,
    resolve_catalog, select_candidate, up_to_date_note,
};

/// The start of the error text of a manual check that could not say "up to date" because the prerelease information was unreachable;
/// the panel shows its own sentence for it (it matches this prefix).
pub(crate) const PRERELEASE_UNAVAILABLE_ERROR: &str = "prerelease_unavailable";
/// The start of the error text of a check that could not read the installed version; the panel says so in its own words (it matches this prefix).
pub(crate) const CURRENT_VERSION_UNKNOWN_ERROR: &str = "current_version_unknown";
const UPDATER_HELPER_ARG: &str = "--updater-helper";
/// The helper that waits for the installer runs from a copy of the Control Panel, never from the
/// installed `sunshine-gui.exe`: the installer replaces that file, and a program running from it
/// holds it open. The copy lives in `<install>\updates\updater-<pid>\`, a folder only administrators
/// can write to. The installer ends the installed Control Panel by name and by path before it copies
/// anything (it cannot replace a file in use); the copy has another name and another folder, so it
/// survives that and can start the Control Panel again afterwards.
const UPDATER_COPY_DIR_PREFIX: &str = "updater-";
const UPDATER_COPY_EXE: &str = "sunshine-updater.exe";
const UPDATE_RESULT_ARG: &str = "--update-result";

// ========== 常量定义 ==========
const UPDATE_CHECK_INTERVAL: u64 = 4 * 60 * 60; // 4小时（秒）

/// temp 目录中记录“已下载待安装”安装包的标记文件名。
const UPDATE_CACHE_MARKER_FILE: &str = "sunshine-update-cache.json";
/// 已下载安装包的最长保留期，超过后随临时目录清理一起删除。
const UPDATE_CACHE_MAX_AGE_SECS: u64 = 14 * 24 * 60 * 60;

/// Progress of the update download, readable after the panel window was
/// closed and opened again. The Rust task owns the transfer; the webview is
/// only a viewer.
#[derive(Clone, Debug, Serialize)]
struct UpdateDownloadStatus {
    active: bool,
    phase: String,
    progress: u32,
    downloaded: u64,
    total: u64,
    source: Option<String>,
    filename: String,
    path: Option<String>,
    error: Option<String>,
    /// Why the download did not come from Mirror酱 although it is the chosen source (`mirrorchyan::Miss::word`); None when it did, or when it was not chosen.
    fallback_reason: Option<String>,
}

impl Default for UpdateDownloadStatus {
    fn default() -> Self {
        Self {
            active: false,
            phase: String::new(),
            progress: 0,
            downloaded: 0,
            total: 0,
            source: None,
            filename: String::new(),
            path: None,
            error: None,
            fallback_reason: None,
        }
    }
}

static UPDATE_DOWNLOAD_STATUS: OnceLock<Mutex<UpdateDownloadStatus>> = OnceLock::new();

fn download_status() -> &'static Mutex<UpdateDownloadStatus> {
    UPDATE_DOWNLOAD_STATUS.get_or_init(|| Mutex::new(UpdateDownloadStatus::default()))
}

fn set_download_status(change: impl FnOnce(&mut UpdateDownloadStatus)) {
    if let Ok(mut status) = download_status().lock() {
        change(&mut status);
    }
}

/// The renderer calls this once on mount: a download started before the window
/// was closed reports its current state instead of disappearing.
#[tauri::command]
pub fn download_update_status() -> serde_json::Value {
    download_status()
        .lock()
        .ok()
        .and_then(|status| serde_json::to_value(&*status).ok())
        .unwrap_or_else(|| serde_json::json!({ "active": false }))
}

fn clear_tooltip_later(app_handle: AppHandle, delay: Duration) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        crate::tray::set_transient_tooltip(&app_handle, None);
    });
}

// ========== 数据结构定义 ==========

/// 更新信息，`is_latest` 决定前端以“查看更新内容”还是“下载新版本”模式展示。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: Option<String>,
    #[serde(default)]
    pub download_fallback_url: Option<String>,
    pub download_name: Option<String>,
    pub download_size: Option<u64>,
    #[serde(default)]
    pub download_sha256: Option<String>,
    #[serde(default)]
    pub download_type: Option<String>,
    pub release_page: String,
    /// `true` = 已是最新（只读浏览），`false`（默认）= 有可用更新
    #[serde(default)]
    pub is_latest: bool,
    /// 仅在 `is_latest` 时有值：`installed_prerelease_no_newer`（当前是预发布版本，没有更新的版本）。
    /// 预发布信息取不到时不返回“已是最新”，手动检查返回以 `prerelease_unavailable` 开头的错误。
    #[serde(default)]
    pub latest_note: Option<String>,
}

/// temp 目录中已下载安装包的缓存标记，用于跳过重复下载并免于被清理。
#[derive(Debug, Serialize, Deserialize)]
struct CachedInstallerMarker {
    filename: String,
    size: u64,
    downloaded_at: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum DownloadPhase {
    Connecting,
    Downloading,
    Retrying,
    Verifying,
    Complete,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DownloadErrorCode {
    SetupFailed,
    FilePreparationFailed,
    FileFinalizationFailed,
    SourcesExhausted,
}

#[derive(Debug, Serialize)]
pub(crate) struct DownloadCommandError {
    code: DownloadErrorCode,
    detail: String,
}

impl DownloadCommandError {
    fn new(code: DownloadErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdaterHelperState {
    installer_path: String,
    extension: String,
    expected_size: Option<u64>,
    expected_sha256: String,
    target_version: Option<String>,
    gui_exe_path: String,
    result_path: String,
    parent_pid: u32,
    /// The panel resolves its theme before elevation; older state files fall
    /// back to the helper's documented WinRT colour query.
    #[serde(default)]
    dark_theme: Option<bool>,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct UpdaterHelperLaunchContext {
    installer_path: String,
    expected_size: Option<u64>,
    expected_sha256: String,
    gui_exe_path: String,
    result_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UpdaterHelperResult {
    success: bool,
    exit_code: Option<i32>,
    target_version: Option<String>,
    message: String,
    finished_at: u64,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct UpdaterPanelState {
    title: String,
    subtitle: String,
    step: usize,
    failed: bool,
    animation_tick: u32,
}

#[cfg(target_os = "windows")]
struct UpdaterAnimationFrame {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

/// Resolved once when the window opens: the panel's theme from the helper state, else the WinRT colour query.
#[cfg(target_os = "windows")]
static UPDATER_DARK_THEME: OnceLock<bool> = OnceLock::new();

#[cfg(target_os = "windows")]
static UPDATER_PANEL_STATE: OnceLock<Arc<Mutex<UpdaterPanelState>>> = OnceLock::new();
static UPDATE_CHECKER_STARTED: OnceLock<()> = OnceLock::new();

#[cfg(target_os = "windows")]
static UPDATER_CONSTRUCTION_FRAMES: OnceLock<Option<Vec<UpdaterAnimationFrame>>> = OnceLock::new();

// ========== 版本和通道相关 ==========

async fn get_current_sunshine_version() -> Result<String, String> {
    crate::sunshine::get_sunshine_version().await
}

fn channel_description(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::StableOnly => "稳定版",
        UpdateChannel::StableAndPrerelease => "稳定版及测试版",
        UpdateChannel::PrereleaseOnly => "测试版",
    }
}

fn preferred_asset_type() -> &'static str {
    // The Panel only supports the Windows Installer update flow. Portable
    // archives may remain in metadata for other consumers, but are never
    // selected or downloaded here.
    "windows-x64-installer"
}

fn is_supported_installer_filename(filename: &str) -> bool {
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

fn selected_asset(candidate: &ReleaseCandidate) -> Option<&ReleaseAsset> {
    candidate.assets.iter().find(|asset| {
        asset.asset_type == preferred_asset_type() && is_supported_installer_filename(&asset.name)
    })
}

fn select_compatible_candidate(
    catalog: &ReleaseCatalog,
    channel: UpdateChannel,
) -> Option<ReleaseCandidate> {
    let compatible_catalog = ReleaseCatalog {
        latest: catalog
            .latest
            .as_ref()
            .filter(|candidate| selected_asset(candidate).is_some())
            .cloned(),
        pre_latest: catalog
            .pre_latest
            .as_ref()
            .filter(|candidate| selected_asset(candidate).is_some())
            .cloned(),
        prerelease_unavailable: catalog.prerelease_unavailable.clone(),
    };
    select_candidate(&compatible_catalog, channel)
}

fn create_update_info(candidate: &ReleaseCandidate) -> UpdateInfo {
    let asset = selected_asset(candidate);
    UpdateInfo {
        version: candidate.version.clone(),
        release_notes: candidate.release_notes.clone(),
        download_url: asset.map(|asset| asset.url.clone()),
        download_fallback_url: asset.and_then(|asset| asset.fallback_url.clone()),
        download_name: asset.map(|asset| asset.name.clone()),
        download_size: asset.map(|asset| asset.size),
        download_sha256: asset.map(|asset| asset.sha256.clone()),
        download_type: asset.map(|asset| asset.asset_type.clone()),
        release_page: candidate.release_page.clone(),
        is_latest: false,
        latest_note: None,
    }
}

/// The newest version a check found when it is newer than this one (None: up to date, or no check yet): the tray's menus show it without asking again.
static SEEN_UPDATE: Mutex<Option<String>> = Mutex::new(None);
fn note_seen_update(version: Option<String>) {
    *SEEN_UPDATE.lock().unwrap_or_else(|p| p.into_inner()) = version;
}
pub(crate) fn last_seen_update() -> Option<String> {
    SEEN_UPDATE.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// The installed version, or the error of a check that cannot tell whether anything is newer (and so reports no update).
fn known_current_version(read: Result<String, String>) -> Result<String, String> {
    match read {
        Ok(version) if !version.trim().is_empty() => Ok(version),
        Ok(_) => Err(format!("{CURRENT_VERSION_UNKNOWN_ERROR}: empty")),
        Err(error) => {
            warn!("⚠️ 获取 Sunshine 版本失败，无法确认当前版本，不报告更新: {error}");
            Err(format!("{CURRENT_VERSION_UNKNOWN_ERROR}: {error}"))
        }
    }
}

/// 检查更新核心逻辑。
///
/// `manual=true` 时已是最新也返回 `Some(info { is_latest: true })`，
/// `manual=false` 时已是最新返回 `None`。
pub async fn check_for_updates_internal(
    manual: bool,
    include_prerelease: bool,
) -> Result<Option<UpdateInfo>, String> {
    let channel = if include_prerelease {
        UpdateChannel::StableAndPrerelease
    } else {
        UpdateChannel::StableOnly
    };
    check_for_updates_in_channel(manual, channel).await
}

async fn check_for_updates_in_channel(
    manual: bool,
    channel: UpdateChannel,
) -> Result<Option<UpdateInfo>, String> {
    info!(
        "🔍 开始检查更新... (通道: {})",
        channel_description(channel)
    );

    // 获取当前 Sunshine 版本
    // Without the installed version nothing can be compared: "0.0.0" made every release look like an update (a false "update available").
    let current_version = known_current_version(get_current_sunshine_version().await)?;

    let catalog = resolve_catalog(channel).await?;
    let candidate = select_compatible_candidate(&catalog, channel)
        .ok_or_else(|| format!("未找到包含 {} 更新资产的发布版本", preferred_asset_type()))?;

    info!(
        "📊 当前 Sunshine 版本: {}, 候选版本: {} ({})",
        current_version,
        candidate.version,
        if candidate.prerelease {
            "预发布"
        } else {
            "稳定版"
        }
    );

    note_seen_update(candidate_is_newer_than(&current_version, &candidate).then(|| candidate.version.clone()));
    if !candidate_is_newer_than(&current_version, &candidate) {
        let note = up_to_date_note(&current_version, &catalog, channel);
        match (note, catalog.prerelease_unavailable.as_deref()) {
            (Some(UpToDateNote::PrereleaseUnavailable), reason) => {
                // Nothing newer was found, but the prerelease side could not be asked: "up to date" would be a claim the check cannot make.
                let reason = reason.unwrap_or("unknown");
                warn!("⚠️ 预发布信息此刻取不到，不能断言已是最新: {reason}");
                if manual {
                    return Err(format!("{PRERELEASE_UNAVAILABLE_ERROR}: {reason}"));
                }
            }
            (Some(UpToDateNote::InstalledPrereleaseNoNewer), _) => {
                info!("当前是预发布版本 {current_version}，没有更新的版本");
            }
            (None, _) => {}
        }
        if manual {
            let mut info = create_update_info(&candidate);
            info.is_latest = true;
            info.latest_note = note.map(|note| note.code().to_string());
            return Ok(Some(info));
        }
        return Ok(None);
    }

    Ok(Some(create_update_info(&candidate)))
}

/// 获取当前时间戳（秒）
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn try_run_updater_helper_from_args() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let Some(index) = args.iter().position(|arg| arg == UPDATER_HELPER_ARG) else {
        return false;
    };

    let Some(state_path) = args.get(index + 1) else {
        return true;
    };

    #[cfg(target_os = "windows")]
    {
        let Some(installer_path) = args.get(index + 2) else {
            return true;
        };
        let Some(expected_size_value) = args.get(index + 3) else {
            return true;
        };
        let Some(expected_sha256) = args
            .get(index + 4)
            .and_then(|value| normalize_sha256(value))
        else {
            return true;
        };
        let Some(gui_exe_path) = args.get(index + 5) else {
            return true;
        };
        let Some(result_path) = args.get(index + 6) else {
            return true;
        };
        let expected_size = if expected_size_value == "-" {
            None
        } else {
            match expected_size_value.parse::<u64>() {
                Ok(value) => Some(value),
                Err(_) => return true,
            }
        };
        let launch = UpdaterHelperLaunchContext {
            installer_path: installer_path.clone(),
            expected_size,
            expected_sha256,
            gui_exe_path: gui_exe_path.clone(),
            result_path: result_path.clone(),
        };
        // Stage one runs from the installed file (it was started through UAC from there). It
        // hands over to a copy and leaves, so that nothing keeps the installed file open.
        let running_as_copy = std::env::current_exe()
            .map(|path| is_updater_copy_path(&path))
            .unwrap_or(false);
        if !running_as_copy && start_updater_copy(&args).is_ok() {
            return true;
        }
        let _ = run_updater_helper(Path::new(state_path), &launch);
    }

    true
}

/// `<install>\updates\updater-<id>\sunshine-updater.exe`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_updater_copy_path(path: &Path) -> bool {
    let name_matches = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(UPDATER_COPY_EXE));
    let folder = path.parent();
    let folder_matches = folder
        .and_then(|folder| folder.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().starts_with(UPDATER_COPY_DIR_PREFIX));
    let updates_matches = folder
        .and_then(|folder| folder.parent())
        .and_then(|folder| folder.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("updates"));
    name_matches && folder_matches && updates_matches
}

#[cfg(target_os = "windows")]
fn updater_copies_root() -> PathBuf {
    crate::sunshine::install_dir().join("updates")
}

/// Copy the running Control Panel and the libraries beside it into a new folder and start the copy
/// with the same arguments.
#[cfg(target_os = "windows")]
fn start_updater_copy(args: &[String]) -> Result<(), String> {
    use std::process::Command;

    let current = std::env::current_exe().map_err(|error| format!("get current exe failed: {error}"))?;
    let source_dir = current
        .parent()
        .ok_or_else(|| "the Control Panel has no folder".to_string())?;
    let root = updater_copies_root();
    fs::create_dir_all(&root).map_err(|error| format!("create updater folder failed: {error}"))?;
    remove_old_updater_copies(&root);
    let folder = root.join(format!("{UPDATER_COPY_DIR_PREFIX}{}", std::process::id()));
    fs::create_dir_all(&folder).map_err(|error| format!("create updater copy folder failed: {error}"))?;
    let copy = folder.join(UPDATER_COPY_EXE);
    let result = (|| -> Result<(), String> {
        fs::copy(&current, &copy).map_err(|error| format!("copy the Control Panel failed: {error}"))?;
        // The libraries the program loads at start (WebView2Loader.dll) sit beside it.
        for entry in fs::read_dir(source_dir)
            .map_err(|error| format!("read the Control Panel folder failed: {error}"))?
            .flatten()
        {
            let path = entry.path();
            let is_library = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
            if is_library && path.is_file() {
                if let Some(name) = path.file_name() {
                    fs::copy(&path, folder.join(name))
                        .map_err(|error| format!("copy a library failed: {error}"))?;
                }
            }
        }
        if file_sha256(&current)? != file_sha256(&copy)? {
            return Err("the copy of the Control Panel does not match the original".to_string());
        }
        Command::new(&copy)
            .args(args.iter().skip(1))
            .spawn()
            .map_err(|error| format!("start the updater copy failed: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&folder);
    }
    result
}

#[cfg(not(target_os = "windows"))]
fn start_updater_copy(_args: &[String]) -> Result<(), String> {
    Err("the updater copy is a Windows mechanism".to_string())
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn file_sha256(path: &Path) -> Result<String, String> {
    use sha2::Digest as _;
    use std::io::Read as _;

    let mut input = fs::File::open(path).map_err(|error| format!("open {path:?} failed: {error}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("read {path:?} failed: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Delete the copies of earlier updates. A copy that still runs cannot be deleted and stays.
#[cfg(target_os = "windows")]
fn remove_old_updater_copies(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_copy = path.is_dir()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.to_ascii_lowercase().starts_with(UPDATER_COPY_DIR_PREFIX));
        if is_copy {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

/// The Control Panel calls this some time after it started: the update that left a copy has long
/// finished then.
pub(crate) fn clean_up_updater_copies_later() {
    #[cfg(target_os = "windows")]
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(90));
        remove_old_updater_copies(&updater_copies_root());
    });
}

pub fn emit_update_result_if_requested(app_handle: &AppHandle) {
    let args: Vec<String> = std::env::args().collect();
    let Some(index) = args.iter().position(|arg| arg == UPDATE_RESULT_ARG) else {
        return;
    };

    let Some(result_path) = args.get(index + 1) else {
        return;
    };

    let result_path = PathBuf::from(result_path);
    let app = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        match fs::read_to_string(&result_path)
            .ok()
            .and_then(|content| serde_json::from_str::<UpdaterHelperResult>(&content).ok())
        {
            Some(result) => {
                let _ = app.emit("update-install-result", result);
                if let Some(dir) = updater_cleanup_dir_for_result(&result_path) {
                    let _ = fs::remove_file(&result_path);
                    let _ = fs::remove_dir_all(dir);
                }
            }
            None => {
                let _ = app.emit(
                    "update-install-result",
                    serde_json::json!({
                        "success": false,
                        "exit_code": null,
                        "target_version": null,
                        "message": "Unable to read updater result.",
                        "finished_at": get_current_timestamp()
                    }),
                );
            }
        }
    });
}

fn updater_cleanup_dir_for_result(result_path: &Path) -> Option<PathBuf> {
    if result_path.file_name()?.to_string_lossy() != "result.json" {
        return None;
    }

    let parent = result_path.parent()?;
    let dir_name = parent.file_name()?.to_string_lossy();
    if !dir_name.starts_with("sunshine-updater-") {
        return None;
    }

    let temp_dir = std::env::temp_dir().canonicalize().ok()?;
    let parent = parent.canonicalize().ok()?;
    if parent.starts_with(temp_dir) {
        Some(parent)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn run_updater_helper(
    state_path: &Path,
    launch: &UpdaterHelperLaunchContext,
) -> Result<(), String> {
    let state_content =
        fs::read_to_string(state_path).map_err(|e| format!("read updater state failed: {}", e))?;
    let state: UpdaterHelperState = serde_json::from_str(&state_content)
        .map_err(|e| format!("parse updater state failed: {}", e))?;

    if state.extension != "exe"
        || state.installer_path != launch.installer_path
        || state.expected_size != launch.expected_size
        || state.expected_sha256 != launch.expected_sha256
        || state.gui_exe_path != launch.gui_exe_path
        || state.result_path != launch.result_path
    {
        return Err("updater state identity mismatch".to_string());
    }

    run_updater_panel(state)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_updater_worker(state: UpdaterHelperState, hwnd: isize) {
    update_updater_panel(hwnd, 0, "准备更新", "正在等待控制面板关闭...");
    wait_for_parent_exit(state.parent_pid);

    let prepare_result = if is_gui_component_installer(Path::new(&state.installer_path)) {
        update_updater_panel(
            hwnd,
            1,
            "正在更新 GUI",
            "Sunshine 核心和当前串流将保持运行。",
        );
        Ok(())
    } else {
        update_updater_panel(
            hwnd,
            1,
            "正在关闭 Sunshine",
            "正在关闭 Sunshine 服务，随后安装更新...",
        );
        stop_sunshine_for_update()
    };

    let install_result = prepare_result.and_then(|()| {
        update_updater_panel(
            hwnd,
            2,
            "正在安装更新",
            "这可能需要一两分钟，请不要重复启动。",
        );
        run_installer_and_wait(&state)
    });

    update_updater_panel(
        hwnd,
        3,
        "正在完成",
        "安装已结束，正在准备重新打开控制面板。",
    );
    let helper_result = match install_result {
        Ok(code) => UpdaterHelperResult {
            success: code == 0,
            exit_code: Some(code),
            target_version: state.target_version.clone(),
            message: if code == 0 {
                "Update installed successfully.".to_string()
            } else {
                format!("Installer exited with code {}.", code)
            },
            finished_at: get_current_timestamp(),
        },
        Err(error) => UpdaterHelperResult {
            success: false,
            exit_code: None,
            target_version: state.target_version.clone(),
            message: error,
            finished_at: get_current_timestamp(),
        },
    };

    if helper_result.success {
        update_updater_panel(hwnd, 3, "更新完成", "正在重新打开 Sunshine 控制面板。");
    } else {
        update_updater_panel(hwnd, 3, "更新未完成", "正在重新打开控制面板并显示结果。");
        // The helper stopped the service for the installer. An installer that refused or failed leaves
        // the installation as it was, and nothing else starts the service again.
        if !is_gui_component_installer(Path::new(&state.installer_path)) {
            start_sunshine_after_failed_update();
        }
    }

    let _ = write_updater_result(&state.result_path, &helper_result);
    let _ = restart_gui_with_update_result(&state.gui_exe_path, &state.result_path);

    std::thread::sleep(Duration::from_millis(900));
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(windows::Win32::Foundation::HWND(hwnd as *mut _)),
            windows::Win32::UI::WindowsAndMessaging::WM_APP + 2,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
}
fn is_gui_component_installer(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase().starts_with("sunshine-gui-setup-"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
// Native boundary rationale and painting rules: panel-ms-createwindowex,
// panel-ms-updatelayeredwindow, panel-ms-wmpaint,
// panel-ms-layeredwindows, panel-ms-uisettings-getcolorvalue
// (docs/authority/source-index.tsv).
fn run_updater_panel(state: UpdaterHelperState) -> Result<(), String> {
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT};
    use windows::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
        HWND_TOPMOST, MSG, PostQuitMessage, RegisterClassW, SW_SHOW, SWP_SHOWWINDOW,
        SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, WINDOW_STYLE,
        WM_APP, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };
    use windows::core::PCWSTR;

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let _ = unsafe { BeginPaint(hwnd, &mut ps) };
                let _ = unsafe { EndPaint(hwnd, &ps) };
                draw_updater_panel_cutout(hwnd);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_CLOSE => {
                update_updater_panel(
                    hwnd.0 as isize,
                    current_updater_step(),
                    "正在安装更新",
                    "更新正在进行，请稍等片刻。",
                );
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ =
                    unsafe { windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(hwnd), 1) };
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            WM_TIMER => {
                tick_updater_panel();
                draw_updater_panel_cutout(hwnd);
                LRESULT(0)
            }
            WM_NCHITTEST => LRESULT(windows::Win32::UI::WindowsAndMessaging::HTCAPTION as isize),
            x if x == WM_APP + 1 => {
                draw_updater_panel_cutout(hwnd);
                LRESULT(0)
            }
            x if x == WM_APP + 2 => {
                let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    let _ = UPDATER_DARK_THEME.set(state.dark_theme.unwrap_or_else(updater_dark_theme));
    let state_arc = Arc::new(Mutex::new(UpdaterPanelState {
        title: "准备更新".to_string(),
        subtitle: "正在启动更新助手...".to_string(),
        step: 0,
        failed: false,
        animation_tick: 0,
    }));
    let _ = UPDATER_PANEL_STATE.set(state_arc);

    let class_name = to_wide_null("SunshineUpdaterPanel");
    let window_title = to_wide_null("Sunshine 正在更新");
    let wnd_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(std::ptr::null_mut()),
        hbrBackground: HBRUSH(std::ptr::null_mut()),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };

    unsafe { RegisterClassW(&wnd_class) };

    let width = 560;
    let height = 240;
    let (x, y) = updater_panel_position(width, height);

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_title.as_ptr()),
            WINDOW_STYLE(WS_POPUP.0),
            x,
            y,
            width,
            height,
            None,
            None,
            Some(HINSTANCE(std::ptr::null_mut())),
            None,
        )
    }
    .map_err(|e| format!("create updater panel failed: {}", e))?;

    let hwnd_value = hwnd.0 as isize;
    std::thread::spawn(move || run_updater_worker(state, hwnd_value));

    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_SHOWWINDOW,
        )
    };
    let _ = unsafe { SetTimer(Some(hwnd), 1, 90, None) };
    let _ = unsafe { SetForegroundWindow(hwnd) };
    draw_updater_panel_cutout(hwnd);
    let _ = unsafe { windows::Win32::Graphics::Gdi::UpdateWindow(hwnd) };

    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        let _ = unsafe { TranslateMessage(&msg) };
        unsafe { DispatchMessageW(&msg) };
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn update_updater_panel(hwnd: isize, step: usize, title: &str, subtitle: &str) {
    if let Some(state) = UPDATER_PANEL_STATE.get() {
        if let Ok(mut state) = state.lock() {
            state.step = step;
            state.title = title.to_string();
            state.subtitle = subtitle.to_string();
            state.failed = title.contains("未完成");
        }
    }

    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(windows::Win32::Foundation::HWND(hwnd as *mut _)),
            windows::Win32::UI::WindowsAndMessaging::WM_APP + 1,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
}

#[cfg(target_os = "windows")]
fn tick_updater_panel() {
    if let Some(state) = UPDATER_PANEL_STATE.get() {
        if let Ok(mut state) = state.lock() {
            state.animation_tick = state.animation_tick.wrapping_add(1);
        }
    }
}

#[cfg(target_os = "windows")]
fn current_updater_step() -> usize {
    UPDATER_PANEL_STATE
        .get()
        .and_then(|state| state.lock().ok().map(|state| state.step))
        .unwrap_or(1)
}

#[cfg(target_os = "windows")]
fn to_wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn updater_panel_position(width: i32, height: i32) -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    if screen_width > width && screen_height > height {
        return ((screen_width - width) / 2, (screen_height - height) / 2);
    }

    (80, 80)
}

#[cfg(target_os = "windows")]
fn updater_dark_theme() -> bool {
    // GetSysColor(COLOR_WINDOW) is a legacy GDI colour and does not track the
    // user's light/dark preference. Microsoft documents UISettings.GetColorValue
    // as the Windows UI colour source.
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    UISettings::new()
        .and_then(|settings| settings.GetColorValue(UIColorType::Background))
        .map(|color| u16::from(color.R) + u16::from(color.G) + u16::from(color.B) < 384)
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn draw_updater_panel_cutout(hwnd: windows::Win32::Foundation::HWND) {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{POINT, RECT, SIZE};
    use windows::Win32::Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DT_LEFT, DT_WORDBREAK, DeleteDC,
        DeleteObject, GetDC, ReleaseDC, SelectObject, SetBkMode, TRANSPARENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowRect, ULW_ALPHA, UpdateLayeredWindow,
    };

    let state = UPDATER_PANEL_STATE
        .get()
        .and_then(|state| state.lock().ok().map(|state| state.clone()))
        .unwrap_or_else(|| UpdaterPanelState {
            title: "正在安装更新".to_string(),
            subtitle: "请稍等片刻。".to_string(),
            step: 1,
            failed: false,
            animation_tick: 0,
        });

    unsafe {
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let width = client.right - client.left;
        let height = client.bottom - client.top;
        if width <= 0 || height <= 0 {
            return;
        }

        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: (width * height * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            ..Default::default()
        };

        let screen_hdc = GetDC(None);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bitmap) = CreateDIBSection(None, &bitmap_info, DIB_RGB_COLORS, &mut bits, None, 0)
        else {
            let _ = ReleaseDC(None, screen_hdc);
            return;
        };
        if bits.is_null() {
            let _ = DeleteObject(bitmap.into());
            let _ = ReleaseDC(None, screen_hdc);
            return;
        }

        let buffer_len = (width * height * 4) as usize;
        let buffer = std::slice::from_raw_parts_mut(bits as *mut u8, buffer_len);
        buffer.fill(0);

        let dark_theme = UPDATER_DARK_THEME.get().copied().unwrap_or(false);
        draw_updater_animation_frame(buffer, width, height, &state);
        draw_updater_bottom_copy(buffer, width, height, &state, dark_theme);
        let mem_hdc = CreateCompatibleDC(Some(screen_hdc));
        if mem_hdc.is_invalid() {
            let _ = DeleteObject(bitmap.into());
            let _ = ReleaseDC(None, screen_hdc);
            return;
        }
        let old_bitmap = SelectObject(mem_hdc, bitmap.into());

        let status_line = format!("{} - {}", state.title, state.subtitle);
        let _ = SetBkMode(mem_hdc, TRANSPARENT);
        draw_text_styled(
            mem_hdc,
            &status_line,
            82,
            180,
            486,
            208,
            DT_LEFT | DT_WORDBREAK,
            13,
            500,
            if dark_theme {
                windows::Win32::Foundation::COLORREF(0x00FAF2F5)
            } else {
                windows::Win32::Foundation::COLORREF(0x00342D30)
            },
        );
        restore_gdi_text_alpha(buffer, width, height, 82, 180, 486, 208);

        let mut window_rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut window_rect);
        let destination = POINT {
            x: window_rect.left,
            y: window_rect.top,
        };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_hdc),
            Some(&destination as *const POINT),
            Some(&size as *const SIZE),
            Some(mem_hdc),
            Some(&source as *const POINT),
            windows::Win32::Foundation::COLORREF(0),
            Some(&blend as *const BLENDFUNCTION),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_hdc, old_bitmap);
        let _ = DeleteDC(mem_hdc);
        let _ = DeleteObject(bitmap.into());
        let _ = ReleaseDC(None, screen_hdc);
    }
}

#[cfg(target_os = "windows")]
fn draw_updater_animation_frame(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    state: &UpdaterPanelState,
) {
    let Some(frames) = updater_construction_frames() else {
        return;
    };
    if frames.is_empty() {
        return;
    }

    let frame_index = (state.animation_tick as usize) % frames.len();
    let frame = &frames[frame_index];
    let copy_width = width.min(frame.width).max(0) as usize;
    let copy_height = height.min(frame.height).max(0) as usize;
    let dst_stride = width as usize * 4;
    let src_stride = frame.width as usize * 4;

    for y in 0..copy_height {
        let dst_start = y * dst_stride;
        let src_start = y * src_stride;
        let byte_len = copy_width * 4;
        buffer[dst_start..dst_start + byte_len]
            .copy_from_slice(&frame.bgra[src_start..src_start + byte_len]);
    }
}

#[cfg(target_os = "windows")] #[rustfmt::skip]
fn draw_updater_bottom_copy(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    state: &UpdaterPanelState,
    dark_theme: bool,
) {
    let (shadow, surface, primary, error) = if dark_theme { ((10, 9, 12, 220), (34, 32, 40, 255), (166, 216, 255, 255), (255, 180, 171, 255)) } else { ((62, 58, 68, 180), (249, 247, 252, 255), (0, 92, 122, 255), (186, 26, 26, 255)) };
    fill_bgra_rect(buffer, width, height, 72, 186, 512, 224, shadow); fill_bgra_rect(buffer, width, height, 64, 178, 504, 216, surface); fill_bgra_rect(buffer, width, height, 104, 210, 484, 220, (120, 120, 128, 100));
    let progress_right = 104 + 380 * state.step.min(3) as i32 / 3; if progress_right > 104 { fill_bgra_rect(buffer, width, height, 104, 210, progress_right, 220, if state.failed { error } else { primary }); } fill_bgra_rect(buffer, width, height, 82, 172, 132, 184, if state.failed { error } else { primary });
}

#[cfg(target_os = "windows")]
fn fill_bgra_rect(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    rgba: (u8, u8, u8, u8),
) {
    let left = left.clamp(0, width) as usize;
    let top = top.clamp(0, height) as usize;
    let right = right.clamp(0, width) as usize;
    let bottom = bottom.clamp(0, height) as usize;
    if left >= right || top >= bottom {
        return;
    }

    let (r, g, b, a) = rgba;
    let premultiply = |channel: u8| ((channel as u16 * a as u16 + 127) / 255) as u8;
    let pixel = [premultiply(b), premultiply(g), premultiply(r), a];
    let stride = width as usize * 4;

    for y in top..bottom {
        let row = y * stride;
        for x in left..right {
            let index = row + x * 4;
            buffer[index..index + 4].copy_from_slice(&pixel);
        }
    }
}

#[cfg(target_os = "windows")]
fn restore_gdi_text_alpha(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    let left = left.clamp(0, width) as usize;
    let top = top.clamp(0, height) as usize;
    let right = right.clamp(0, width) as usize;
    let bottom = bottom.clamp(0, height) as usize;
    if left >= right || top >= bottom {
        return;
    }

    let stride = width as usize * 4;
    for y in top..bottom {
        let row = y * stride;
        for x in left..right {
            let pixel = row + x * 4;
            // GDI writes the glyph RGB into a 32-bit DIB but clears its alpha byte.
            if buffer[pixel + 3] == 0
                && (buffer[pixel] != 0 || buffer[pixel + 1] != 0 || buffer[pixel + 2] != 0)
            {
                buffer[pixel + 3] = 255;
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn updater_construction_frames() -> Option<&'static [UpdaterAnimationFrame]> {
    UPDATER_CONSTRUCTION_FRAMES
        .get_or_init(decode_updater_construction_frames)
        .as_deref()
}

#[cfg(target_os = "windows")]
fn decode_updater_construction_frames() -> Option<Vec<UpdaterAnimationFrame>> {
    let sheet = image::load_from_memory(updater_construction_frames_bytes())
        .ok()?
        .to_rgba8();
    let (sheet_width, sheet_height) = sheet.dimensions();
    if sheet_height != 240 || sheet_width < 560 || sheet_width % 560 != 0 {
        return None;
    }

    let frame_width = 560;
    let frame_height = sheet_height;
    let frame_count = sheet_width / frame_width;
    let mut decoded = Vec::with_capacity(frame_count as usize);

    for frame_index in 0..frame_count {
        let mut bgra = Vec::with_capacity((frame_width * frame_height * 4) as usize);
        let left = frame_index * frame_width;
        for y in 0..frame_height {
            for x in 0..frame_width {
                let [r, g, b, a] = sheet.get_pixel(left + x, y).0;
                let premultiply = |channel: u8| ((channel as u16 * a as u16 + 127) / 255) as u8;
                bgra.push(premultiply(b));
                bgra.push(premultiply(g));
                bgra.push(premultiply(r));
                bgra.push(a);
            }
        }

        decoded.push(UpdaterAnimationFrame {
            width: frame_width as i32,
            height: frame_height as i32,
            bgra,
        });
    }

    Some(decoded)
}

#[cfg(target_os = "windows")]
fn updater_construction_frames_bytes() -> &'static [u8] {
    include_bytes!("../assets/updater-construction-frames.png")
}

#[cfg(target_os = "windows")]
unsafe fn draw_text_styled(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
    size: i32,
    weight: i32,
    color: windows::Win32::Foundation::COLORREF,
) {
    use windows::Win32::Graphics::Gdi::{
        ANTIALIASED_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
        DeleteObject, FF_DONTCARE, OUT_DEFAULT_PRECIS, SelectObject,
    };
    use windows::core::PCWSTR;

    let face = to_wide_null(if text.is_ascii() {
        "Segoe UI Variable Text"
    } else {
        "Microsoft YaHei UI"
    });
    let font = unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
            PCWSTR(face.as_ptr()),
        )
    };
    let previous_font = unsafe { SelectObject(hdc, font.into()) };
    let _ = unsafe { windows::Win32::Graphics::Gdi::SetTextColor(hdc, color) };
    unsafe { draw_text(hdc, text, left, top, right, bottom, format) };
    let _ = unsafe { SelectObject(hdc, previous_font) };
    let _ = unsafe { DeleteObject(font.into()) };
}

#[cfg(target_os = "windows")]
unsafe fn draw_text(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
) {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = windows::Win32::Foundation::RECT {
        left,
        top,
        right,
        bottom,
    };
    let _ = unsafe { windows::Win32::Graphics::Gdi::DrawTextW(hdc, &mut wide, &mut rect, format) };
}

#[cfg(target_os = "windows")]
fn wait_for_parent_exit(parent_pid: u32) {
    if parent_pid == 0 || parent_pid == std::process::id() {
        return;
    }

    for _ in 0..45 {
        if !is_pid_running(parent_pid) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(target_os = "windows")]
fn is_pid_running(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let filter = format!("PID eq {}", pid);
    let Ok(output) = Command::new("tasklist")
        .args(["/FI", &filter, "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };

    let text = crate::text_encoding::decode_console_bytes(&output.stdout);
    text.contains(&pid.to_string())
}

#[cfg(target_os = "windows")]
fn stage_installer_for_elevated_run(
    state: &UpdaterHelperState,
) -> Result<tempfile::TempPath, String> {
    let staging_dir = crate::sunshine::install_dir().join("updates");
    fs::create_dir_all(&staging_dir)
        .map_err(|error| format!("create protected updater directory failed: {error}"))?;
    stage_installer_in_directory(state, &staging_dir)
}

#[cfg(target_os = "windows")]
fn stage_installer_in_directory(
    state: &UpdaterHelperState,
    directory: &Path,
) -> Result<tempfile::TempPath, String> {
    let temporary = tempfile::Builder::new()
        .prefix("sunshine-update-")
        .suffix(".exe")
        .tempfile_in(directory)
        .map_err(|error| format!("create protected installer failed: {error}"))?;
    let (mut output, path) = temporary.into_parts();
    let mut input = fs::File::open(&state.installer_path)
        .map_err(|error| format!("open update installer failed: {error}"))?;
    std::io::copy(&mut input, &mut output)
        .map_err(|error| format!("stage installer in protected directory failed: {error}"))?;
    drop(output);
    validate_download_file_identity(
        &path,
        state.expected_size,
        Some(&state.expected_sha256),
        Some("windows-x64-installer"),
    )
    .map_err(|error| format!("protected installer verification failed: {error}"))?;
    Ok(path)
}

#[cfg(target_os = "windows")]
fn run_installer_and_wait(state: &UpdaterHelperState) -> Result<i32, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    if state.extension != "exe" {
        return Err("Only Windows Installer .exe files are supported".to_string());
    }
    let installer_path = stage_installer_for_elevated_run(state)?;

    let status = Command::new(installer_path.as_os_str())
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("start installer failed: {}", e));
    let status = status?;

    Ok(status.code().unwrap_or(-1))
}

#[cfg(target_os = "windows")]
fn write_updater_result(result_path: &str, result: &UpdaterHelperResult) -> Result<(), String> {
    let result_path = Path::new(result_path);
    if let Some(parent) = result_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create result dir failed: {}", e))?;
    }

    let content = serde_json::to_string_pretty(result)
        .map_err(|e| format!("serialize updater result failed: {}", e))?;
    fs::write(result_path, content).map_err(|e| format!("write updater result failed: {}", e))
}

#[cfg(target_os = "windows")]
fn restart_gui_with_update_result(gui_exe_path: &str, result_path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    Command::new(gui_exe_path)
        .args([UPDATE_RESULT_ARG, result_path])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("restart control panel failed: {}", e))?;

    Ok(())
}

/// Tauri命令：获取是否包含预发布版本的偏好
#[tauri::command]
pub fn get_include_prerelease_preference(app: AppHandle) -> bool {
    get_include_prerelease(&app)
}

/// Tauri命令：设置是否包含预发布版本的偏好
#[tauri::command]
pub fn set_include_prerelease_preference(app: AppHandle, include: bool) {
    preferences::set_include_prerelease(&app, include);
}

/// Tauri命令：Mirror酱 CDK 是否已存入操作系统安全存储。只回答布尔值。
#[tauri::command]
pub async fn update_cdk_present() -> bool {
    tauri::async_runtime::spawn_blocking(|| cdk_store::present(mirrorchyan::CREDENTIAL_TARGET))
        .await
        .unwrap_or(false)
}

/// Tauri命令：把 CDK 存入操作系统安全存储（Windows Credential Manager）。
/// CDK 不写文件、不写日志、不跨 IPC 回传。
#[tauri::command]
pub async fn update_cdk_set(cdk: String) -> Result<bool, String> {
    let cdk = cdk_store::validate_cdk(&cdk)?;
    tauri::async_runtime::spawn_blocking(move || {
        cdk_store::set(mirrorchyan::CREDENTIAL_TARGET, &cdk)
    })
    .await
    .map_err(|_| "系统安全存储任务失败".to_string())??;
    Ok(true)
}

/// Tauri命令：从操作系统安全存储删除 CDK；不存在时也成功（幂等）。
#[tauri::command]
pub async fn update_cdk_clear() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| cdk_store::clear(mirrorchyan::CREDENTIAL_TARGET))
        .await
        .map_err(|_| "系统安全存储任务失败".to_string())??;
    Ok(true)
}

/// Tauri命令：检查 Mirror酱 的最新版本。渲染层传入 `currentVersion` 与
/// `channel`，CDK 在这里从安全存储读出并拼进请求 URL；回复是
/// `{ status, json }`：`status: 0, json: null` 表示无回答（渲染层映射为
/// network），`json` 已去掉 `data.url` 与任何 CDK 回显。完整 URL 与错误原文
/// 都包含 CDK，绝不写日志。
#[tauri::command]
pub async fn update_mirrorchyan_check(
    current_version: String,
    channel: String,
) -> serde_json::Value {
    let current_version: String = current_version.chars().take(128).collect();
    let channel = mirrorchyan::normalize_channel(&channel);
    let cdk = read_stored_cdk().await;
    let answer = mirrorchyan::check(&current_version, cdk.as_deref(), channel).await;
    info!("Mirror酱 检查: {}", mirrorchyan::log_summary(&answer));
    serde_json::json!({
        "status": answer.status,
        "json": mirrorchyan::sanitize_body(&answer),
    })
}

/// Tauri命令：保存更新来源（auto|cnb|github|mirror）到 update_preferences.json。
#[tauri::command]
pub fn set_update_source(app: AppHandle, source: String) -> Result<bool, String> {
    preferences::set_update_source(&app, &source)?;
    Ok(true)
}

/// Tauri命令：匿名使用统计里的 update_source 取值来源：最近一次真正用到的来源（mirror / cnb / github），
/// 还没有下载过时是用户的设置（auto / cnb / github / mirror），永远不是空串。
#[tauri::command]
pub fn get_update_source_used(app: AppHandle) -> String {
    preferences::used_update_source(&app)
}

/// Tauri命令：最近一次下载里首选来源（Mirror酱）没有用上的原因（version_lag / cdk_invalid / quota / network / no_cdk）；用上了或没选 Mirror酱 时是空串。
#[tauri::command]
pub fn get_update_source_miss(app: AppHandle) -> String {
    preferences::update_source_miss(&app)
}

/// 读取安全存储中的 CDK；存储不可用或为空都按“无 CDK”处理。
async fn read_stored_cdk() -> Option<String> {
    match tauri::async_runtime::spawn_blocking(|| cdk_store::read(mirrorchyan::CREDENTIAL_TARGET))
        .await
    {
        Ok(Ok(stored)) => stored,
        Ok(Err(error)) => {
            // 错误文本只含错误码，不含 CDK。
            warn!("读取系统安全存储失败: {error}");
            None
        }
        Err(_) => None,
    }
}

/// Tauri 命令：手动检查更新
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let include_prerelease = get_include_prerelease(&app);
    let result = check_for_updates_internal(true, include_prerelease).await;
    if result.is_ok() {
        save_last_check_time(&app);
    }
    result
}

/// Tauri 命令：按首页中用户明确选择的发布通道检查更新。
///
/// 该命令不修改侧栏 Beta 偏好；测试版仍同时比较 stable 和 prerelease，
/// 只保证原生更新器展示的候选与用户选择的通道一致。
#[tauri::command]
pub async fn check_for_updates_for_channel(
    app: AppHandle,
    channel: String,
) -> Result<Option<UpdateInfo>, String> {
    let channel = match channel.as_str() {
        "stable" => UpdateChannel::StableOnly,
        "prerelease" => UpdateChannel::StableAndPrerelease,
        _ => return Err(format!("不支持的更新通道: {channel}")),
    };

    let result = check_for_updates_in_channel(true, channel).await;
    if result.is_ok() {
        save_last_check_time(&app);
    }
    result
}

/// 检查是否需要自动更新
fn should_auto_check(last_check_time: u64) -> bool {
    let current_time = get_current_timestamp();
    current_time.saturating_sub(last_check_time) > UPDATE_CHECK_INTERVAL
}

/// 处理自动检查结果（`manual=false`，`Some` 必定是新版本）
fn handle_auto_check_result<R: Runtime>(
    app: &AppHandle<R>,
    result: Result<Option<UpdateInfo>, String>,
) {
    match result {
        Ok(Some(update_info)) => {
            if !crate::desktop_settings::load_desktop_settings_from_disk().update_notify {
                debug!("Update notification disabled by desktop settings");
                save_last_check_time(app);
                return;
            }
            info!("🎉 发现新版本: {}", update_info.version);
            save_last_check_time(app);

            // 发送事件到前端
            for label in ["main", "desktop"] {
                if let Some(window) = app.get_webview_window(label) {
                    let _ = window.emit("update-available", &update_info);
                }
            }
        }
        Ok(None) => {
            debug!("✅ 已是最新版本");
            save_last_check_time(app);
        }
        Err(e) => {
            error!("❌ 检查更新失败: {}", e);
        }
    }
}

/// 启动时自动检查更新（如果距离上次检查超过4小时）
pub fn check_for_updates_on_startup<R: Runtime + 'static>(app: AppHandle<R>) {
    clean_up_updater_copies_later();
    let last_check_time = preferences::last_check_time(&app);

    if !should_auto_check(last_check_time) {
        debug!("⏰ 距离上次检查时间未超过4小时，跳过自动检查");
        return;
    }

    debug!("⏰ 距离上次检查已超过4小时，自动检查更新...");
    let app_clone = app.clone();
    let include_prerelease = get_include_prerelease(&app);
    tauri::async_runtime::spawn(async move {
        let result = check_for_updates_internal(false, include_prerelease).await;
        handle_auto_check_result(&app_clone, result);
    });
}

// ========== 进程管理 ==========

#[cfg(target_os = "windows")]
fn validate_update_service_image(image: &Path, install_dir: &Path) -> Result<(), String> {
    let root = install_dir
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let image = image.canonicalize().map_err(|error| error.to_string())?;
    if !image.starts_with(&root) {
        return Err("refusing to stop a service process outside the target installation".into());
    }
    Ok(())
}

/// Stop only this installation's service. Its existing shutdown handler owns Core cleanup.
#[cfg(target_os = "windows")]
fn stop_sunshine_for_update() -> Result<(), String> {
    crate::host_facts::expect_stop(900_000);
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Services::*;
    use windows::Win32::System::Threading::*;
    use windows::core::{Owned, PCWSTR, PWSTR, w};

    fn query_status(service: SC_HANDLE) -> Result<SERVICE_STATUS_PROCESS, String> {
        let mut status = SERVICE_STATUS_PROCESS::default();
        let mut needed = 0;
        // The initialized structure supplies an aligned, exactly sized output buffer.
        unsafe {
            let buffer = std::slice::from_raw_parts_mut(
                std::ptr::from_mut(&mut status).cast::<u8>(),
                std::mem::size_of::<SERVICE_STATUS_PROCESS>(),
            );
            QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, Some(buffer), &mut needed)
                .map_err(|error| format!("query update service failed: {error}"))?;
        }
        Ok(status)
    }

    let root = crate::sunshine::install_dir();
    if !crate::desktop_settings::sunshine_service_matches_install(&root)? {
        info!("No service belongs to this installation; leaving other services running");
        return Ok(());
    }
    unsafe {
        let manager = Owned::new(
            OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|error| format!("open service manager failed: {error}"))?,
        );
        let service = Owned::new(
            OpenServiceW(
                *manager,
                w!("SunshineService"),
                SERVICE_QUERY_STATUS | SERVICE_STOP,
            )
            .map_err(|error| format!("open update service failed: {error}"))?,
        );
        let status = query_status(*service)?;
        if status.dwCurrentState == SERVICE_STOPPED {
            return end_leftover_hosts(&root);
        }
        // SCM's PID is only meaningful in stable running/paused states.
        if !matches!(status.dwCurrentState, SERVICE_RUNNING | SERVICE_PAUSED)
            || status.dwProcessId == 0
            || status.dwProcessId == std::process::id()
        {
            return Err(
                "update service is changing state; retry the update when it settles".into(),
            );
        }
        let process = Owned::new(
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                status.dwProcessId,
            )
            .map_err(|error| format!("open update service process failed: {error}"))?,
        );
        let mut image = vec![0u16; 32768];
        let mut length = image.len() as u32;
        QueryFullProcessImageNameW(
            *process,
            PROCESS_NAME_WIN32,
            PWSTR(image.as_mut_ptr()),
            &mut length,
        )
        .map_err(|error| format!("identify update service process failed: {error}"))?;
        validate_update_service_image(
            &PathBuf::from(std::ffi::OsString::from_wide(&image[..length as usize])),
            &root,
        )?;
        let confirmed = query_status(*service)?;
        if confirmed.dwProcessId != status.dwProcessId
            || confirmed.dwCurrentState != status.dwCurrentState
        {
            return Err("update service changed during verification; retry the update".into());
        }
        let mut stopped = SERVICE_STATUS::default();
        ControlService(*service, SERVICE_CONTROL_STOP, &mut stopped)
            .map_err(|error| format!("stop update service failed: {error}"))?;
        match WaitForSingleObject(*process, 45_000) {
            WAIT_OBJECT_0 => end_leftover_hosts(&root),
            WAIT_TIMEOUT => Err("update service did not exit; installation was cancelled".into()),
            _ => Err(format!(
                "wait for update service failed: {}",
                std::io::Error::last_os_error()
            )),
        }
    }
}

/// Start this installation's service again after an update that did not finish. Best effort: a
/// service that is already running, or that is not this installation's, is left alone.
#[cfg(target_os = "windows")]
fn start_sunshine_after_failed_update() {
    use windows::Win32::System::Services::*;
    use windows::core::{Owned, PCWSTR, w};

    let root = crate::sunshine::install_dir();
    if !matches!(
        crate::desktop_settings::sunshine_service_matches_install(&root),
        Ok(true)
    ) {
        return;
    }
    unsafe {
        let Ok(manager) = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) else {
            return;
        };
        let manager = Owned::new(manager);
        let Ok(service) = OpenServiceW(*manager, w!("SunshineService"), SERVICE_START) else {
            return;
        };
        let service = Owned::new(service);
        let _ = StartServiceW(*service, None);
    }
}

/// The service stop reaps the child it launched. A user-mode sunshine.exe, or a child that
/// outlived the service, is still holding the ports. End those before the installer runs.
#[cfg(target_os = "windows")]
fn end_leftover_hosts(root: &std::path::Path) -> Result<(), String> {
    crate::sunshine_service::end_host_processes(root).map_err(|code| {
        format!("sunshine.exe was still running and could not be ended ({code}); installation was cancelled")
    })
}

#[cfg(all(test, target_os = "windows"))]
mod update_service_scope_tests {
    use super::*;

    #[test]
    fn service_image_must_resolve_inside_the_target_installation() {
        let temporary = tempfile::tempdir().unwrap();
        let target = temporary.path().join("目标 Sunshine ' with spaces");
        let other = temporary.path().join("目标 Sunshine ' with spaces-other");
        fs::create_dir_all(target.join("tools")).unwrap();
        fs::create_dir_all(&other).unwrap();
        let image = target.join("tools/sunshinesvc.exe");
        fs::write(&image, b"fixture").unwrap();
        assert!(validate_update_service_image(&image, &target).is_ok());
        let foreign = other.join("sunshinesvc.exe");
        fs::write(&foreign, b"fixture").unwrap();
        assert!(validate_update_service_image(&foreign, &target).is_err());
        assert!(
            validate_update_service_image(
                &target.join("../目标 Sunshine ' with spaces-other/sunshinesvc.exe"),
                &target
            )
            .is_err()
        );
        assert!(validate_update_service_image(&target.join("missing.exe"), &target).is_err());
    }
}

// ========== 下载相关 ==========

fn validate_download_url(url: &str, expected_filename: Option<&str>) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|error| format!("下载地址格式无效: {}", error))?;
    if parsed.scheme() != "https"
        || parsed.port().is_some_and(|port| port != 443)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("下载地址必须是无凭据的 HTTPS 地址".to_string());
    }
    let expected_prefix: &[&str] = match parsed.host_str() {
        Some("github.com") => &["AlkaidLab", "foundation-sunshine", "releases", "download"],
        Some("cnb.cool") => &[
            "AlkaidLab",
            "foundation-sunshine-release",
            "-",
            "releases",
            "download",
        ],
        _ => return Err("下载地址不是允许的 Foundation Sunshine Release 地址".to_string()),
    };
    let segments = parsed
        .path_segments()
        .ok_or_else(|| "下载地址缺少路径".to_string())?
        .collect::<Vec<_>>();
    if segments.len() != expected_prefix.len() + 2
        || !segments
            .iter()
            .take(expected_prefix.len())
            .zip(expected_prefix)
            .all(|(actual, expected)| *actual == *expected)
    {
        return Err("下载地址不是 Foundation Sunshine Release 附件".to_string());
    }
    if let Some(expected_filename) = expected_filename {
        let actual_filename = metadata::decode_segment(segments[expected_prefix.len() + 1])?;
        if actual_filename != expected_filename {
            return Err("下载地址的文件名与 metadata 不一致".to_string());
        }
    }
    Ok(())
}

fn validate_download_filename(filename: &str) -> Result<(), String> {
    if filename.trim().is_empty() {
        return Err("下载文件名不能为空".to_string());
    }
    if filename.contains('/') || filename.contains('\\') {
        return Err("下载文件名不能包含路径分隔符".to_string());
    }

    let mut components = Path::new(filename).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err("下载文件名必须是单个普通文件名".to_string());
    }

    Ok(())
}

fn download_source_name(url: &str) -> &'static str {
    match reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
        .as_deref()
    {
        Some("cnb.cool") => "CNB",
        _ => "GitHub",
    }
}

fn normalize_sha256(value: &str) -> Option<String> {
    let value = value.strip_prefix("sha256:").unwrap_or(value).trim();
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn validate_download_file_identity(
    path: &Path,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
    asset_type: Option<&str>,
) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|error| format!("读取下载文件失败: {error}"))?;
    if !metadata.is_file() {
        return Err("下载结果不是普通文件".to_string());
    }
    if let Some(expected_size) = expected_size.filter(|size| *size > 0)
        && metadata.len() != expected_size
    {
        return Err(format!(
            "文件大小不匹配: expected {expected_size}, got {}",
            metadata.len()
        ));
    }

    if let Some(expected_sha256) = expected_sha256 {
        let expected_sha256 = normalize_sha256(expected_sha256)
            .ok_or_else(|| "expected SHA-256 必须是 64 位十六进制字符串".to_string())?;
        let actual = crate::native_components::install::sha256_file(path, metadata.len())
            .map_err(|error| format!("读取下载文件失败: {error}"))?;
        if actual != expected_sha256 {
            return Err(format!(
                "SHA-256 不匹配: expected {expected_sha256}, got {actual}"
            ));
        }
    }

    let mut header = [0u8; 4];
    use std::io::Read;
    let mut header_file =
        fs::File::open(path).map_err(|error| format!("打开下载文件失败: {error}"))?;
    let count = header_file
        .read(&mut header)
        .map_err(|error| format!("读取文件头失败: {error}"))?;
    let valid_type = match asset_type {
        Some("windows-x64-installer") => count >= 2 && &header[..2] == b"MZ",
        Some("checksum-json") | Some("checksum-text") | None => true,
        Some(other) => return Err(format!("不支持的下载资产类型: {other}")),
    };
    if !valid_type {
        return Err("下载文件格式与资产类型不匹配".to_string());
    }
    Ok(())
}

fn validate_installer_path(
    path: &Path,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
) -> Result<PathBuf, String> {
    let file = path
        .canonicalize()
        .map_err(|error| format!("无法定位安装包: {error}"))?;
    let temp_path = std::env::temp_dir();
    let temp_dir = temp_path
        .canonicalize()
        .map_err(|error| format!("无法定位临时目录: {error}"))?;
    if file.parent() != Some(temp_dir.as_path()) {
        return Err("安装包必须位于临时目录的顶层".to_string());
    }

    let filename = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "安装包文件名无效".to_string())?;
    validate_download_filename(filename)?;
    if !is_supported_installer_filename(filename) {
        return Err("Panel 只支持 Windows Installer .exe 安装包".to_string());
    }
    validate_download_file_identity(
        &file,
        expected_size,
        expected_sha256,
        Some("windows-x64-installer"),
    )?;
    // Keep the validated, normal temp path for the Windows installer. The
    // canonical path can contain a `\\?\` prefix which older installers do
    // not consistently accept, while the canonical checks above already
    // enforce the directory boundary and file identity.
    Ok(temp_path.join(filename))
}

/// 发送下载进度事件到前端
fn emit_download_progress(
    window: &tauri::WebviewWindow,
    progress: u32,
    total: u64,
    downloaded: u64,
    phase: DownloadPhase,
    source: Option<&str>,
) {
    let fallback_reason = download_status()
        .lock()
        .ok()
        .and_then(|status| status.fallback_reason.clone());
    let _ = window.emit(
        "download-progress",
        serde_json::json!({
            "progress": progress,
            "total": total,
            "downloaded": downloaded,
            "phase": phase,
            "source": source,
            "fallbackReason": fallback_reason
        }),
    );
}

fn emit_install_progress(
    app_handle: &AppHandle,
    stage: &str,
    detail: Option<&str>,
    terminal: bool,
) {
    let _ = app_handle.emit(
        "install-progress",
        serde_json::json!({
            "stage": stage,
            "detail": detail.unwrap_or(""),
            "terminal": terminal
        }),
    );
}

fn remove_file_if_exists(
    path: &Path,
    error_code: DownloadErrorCode,
    action: &str,
) -> Result<(), DownloadCommandError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DownloadCommandError::new(
            error_code,
            format!("{}失败: {}", action, error),
        )),
    }
}

async fn finalize_download_file(
    partial_path: &Path,
    file_path: &Path,
) -> Result<(), DownloadCommandError> {
    remove_file_if_exists(
        file_path,
        DownloadErrorCode::FileFinalizationFailed,
        "替换旧安装包",
    )?;
    tokio::fs::rename(partial_path, file_path)
        .await
        .map_err(|error| {
            DownloadCommandError::new(
                DownloadErrorCode::FileFinalizationFailed,
                format!("完成下载文件失败: {}", error),
            )
        })
}

/// 按来源把 Mirror酱 候选（若有）排在 CNB/GitHub 之前，并保持既有的
/// 去重规则（fallback 与主地址相同则不重复）。
fn download_candidates(
    mirror: Option<github_download::DownloadCandidate>,
    url: &str,
    fallback_url: Option<&str>,
) -> Vec<github_download::DownloadCandidate> {
    let mut candidates = Vec::new();
    if let Some(mirror) = mirror {
        candidates.push(mirror);
    }
    candidates.push(github_download::DownloadCandidate {
        name: download_source_name(url),
        url: url.to_string(),
    });
    if let Some(fallback_url) = fallback_url {
        if !candidates
            .iter()
            .any(|candidate| candidate.url == fallback_url)
        {
            candidates.push(github_download::DownloadCandidate {
                name: download_source_name(fallback_url),
                url: fallback_url.to_string(),
            });
        }
    }
    candidates
}

/// 当 CDK 有效且 Mirror酱 给出合规下载地址时返回首选候选。地址带时效令牌，
/// 绝不写日志；文件内容仍按本项目 metadata 的 SHA-256 校验后才安装。
async fn mirror_download_candidate(
    app_handle: &AppHandle,
    target_version: Option<&str>,
) -> (
    Option<github_download::DownloadCandidate>,
    Option<mirrorchyan::Miss>,
) {
    let Some(cdk) = read_stored_cdk().await else {
        return (None, Some(mirrorchyan::Miss::NoCdk));
    };
    let current_version = get_current_sunshine_version()
        .await
        .unwrap_or_else(|error| {
            warn!("获取当前版本失败（Mirror酱 检查按 0.0.0 处理）: {error}");
            "0.0.0".to_string()
        });
    let channel = if get_include_prerelease(app_handle) {
        "beta"
    } else {
        "stable"
    };
    let answer = mirrorchyan::check(&current_version, Some(&cdk), channel).await;
    info!(
        "Mirror酱 检查（下载候选）: {}",
        mirrorchyan::log_summary(&answer)
    );
    // Mirror酱 syncs on a timer: its newest version can be older than the release to install. Its file would fail the SHA-256 of that release, so it is not tried.
    if let Some(miss) = mirrorchyan::miss_of(&answer, target_version) {
        info!(
            "Mirror酱 不用于本次下载（原因: {}；Mirror酱 的版本: {}，要安装的版本: {}）",
            miss.word(),
            mirrorchyan::version_of(&answer).unwrap_or("-"),
            target_version.unwrap_or("-")
        );
        return (None, Some(miss));
    }
    let candidate = mirrorchyan::download_url_of(&answer).map(|url| github_download::DownloadCandidate {
        name: "Mirror酱",
        url,
    });
    (candidate, None)
}

/// 下载更新文件（带真实进度报告）
#[tauri::command]
pub async fn download_update(
    url: String,
    fallback_url: Option<String>,
    filename: String,
    expected_size: Option<u64>,
    expected_sha256: Option<String>,
    asset_type: Option<String>,
    target_version: Option<String>,
    app_handle: AppHandle,
) -> Result<serde_json::Value, DownloadCommandError> {
    validate_download_url(&url, Some(&filename)).map_err(|detail| {
        DownloadCommandError::new(DownloadErrorCode::FilePreparationFailed, detail)
    })?;
    validate_download_filename(&filename).map_err(|detail| {
        DownloadCommandError::new(DownloadErrorCode::FilePreparationFailed, detail)
    })?;
    if asset_type.as_deref() != Some("windows-x64-installer") {
        return Err(DownloadCommandError::new(
            DownloadErrorCode::FilePreparationFailed,
            "Panel 只支持 Windows Installer 安装包，不下载 Portable 压缩包".to_string(),
        ));
    }
    if !is_supported_installer_filename(&filename) {
        return Err(DownloadCommandError::new(
            DownloadErrorCode::FilePreparationFailed,
            "Panel 只支持 Windows Installer .exe 安装包".to_string(),
        ));
    }
    let expected_sha256 = expected_sha256
        .as_deref()
        .and_then(normalize_sha256)
        .ok_or_else(|| {
            DownloadCommandError::new(
                DownloadErrorCode::FilePreparationFailed,
                "更新 metadata 缺少有效 SHA-256".to_string(),
            )
        })?;
    if let Some(fallback_url) = fallback_url.as_deref() {
        validate_download_url(fallback_url, Some(&filename)).map_err(|detail| {
            DownloadCommandError::new(DownloadErrorCode::FilePreparationFailed, detail)
        })?;
    }

    info!("📥 开始下载更新: {}", filename);

    cleanup_old_installers();

    let download_dir = std::env::temp_dir();
    let file_path = download_dir.join(&filename);
    let partial_path = download_dir.join(format!("{}.part", filename));
    let window = app_handle.get_webview_window("main");

    // 安装包已完整下载（例如用户在上次确认框点了取消）时直接复用
    if let Some(cached_path) = find_cached_installer_verified(
        &filename,
        expected_size,
        Some(&expected_sha256),
        asset_type.as_deref(),
    ) {
        info!("♻️ 已存在完整安装包，跳过下载: {}", filename);
        write_update_cache_marker(
            &filename,
            fs::metadata(&cached_path).map(|m| m.len()).unwrap_or(0),
        );
        return Ok(serde_json::json!({
            "success": true,
            "file_path": cached_path.to_string_lossy().to_string(),
            "cached": true,
        }));
    }

    remove_file_if_exists(
        &partial_path,
        DownloadErrorCode::FilePreparationFailed,
        "清理旧的临时下载文件",
    )?;

    set_download_status(|status| {
        *status = UpdateDownloadStatus {
            active: true,
            phase: "connecting".to_string(),
            filename: filename.clone(),
            ..Default::default()
        };
    });
    crate::tray::set_transient_tooltip(
        &app_handle,
        Some(crate::tray::download_progress_tooltip(0)),
    );

    // 来源为 mirror 且 CDK 有效时，Mirror酱 的下载地址排在最前；
    // 任何失败（无 CDK、CDK 错误、离线、地址不合规）都静默回退 CNB → GitHub。
    let mirror_chosen = preferences::update_source(&app_handle) == "mirror";
    let (mirror_candidate, mut mirror_miss) = if mirror_chosen {
        mirror_download_candidate(&app_handle, target_version.as_deref()).await
    } else {
        (None, None)
    };
    if let Some(miss) = mirror_miss {
        set_download_status(|status| status.fallback_reason = Some(miss.word().to_string()));
    }
    let candidates = download_candidates(mirror_candidate, &url, fallback_url.as_deref());

    let mut last_progress_percent = 0;
    let mut last_tooltip_percent = 0u32;
    let mut last_tooltip_at = std::time::Instant::now();
    let outcome = github_download::download_to_file_from_candidates(
        DownloadRequest {
            url: &url,
            destination: &partial_path,
            expected_size: expected_size.filter(|size| *size > 0),
            expected_sha256: Some(&expected_sha256),
            max_size: expected_size.filter(|size| *size > 0),
            user_agent: "Sunshine-Control-Panel updater",
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            overall_timeout: Some(Duration::from_secs(30 * 60)),
        },
        &candidates,
        |event| {
            match event.phase {
                DownloadAttemptPhase::Connecting => {
                    last_progress_percent = 0;
                    set_download_status(|status| {
                        status.phase = "connecting".to_string();
                        status.progress = 0;
                        status.total = event.total;
                    });
                    if let Some(win) = window.as_ref() {
                        emit_download_progress(
                            win,
                            0,
                            event.total,
                            0,
                            DownloadPhase::Connecting,
                            None,
                        );
                    }
                }
                DownloadAttemptPhase::Retrying => {
                    last_progress_percent = 0;
                    set_download_status(|status| {
                        status.phase = "retrying".to_string();
                        status.progress = 0;
                        status.total = event.total;
                    });
                    if let Some(win) = window.as_ref() {
                        emit_download_progress(
                            win,
                            0,
                            event.total,
                            0,
                            DownloadPhase::Retrying,
                            None,
                        );
                    }
                }
                DownloadAttemptPhase::Downloading => {
                    let progress = if event.total == 0 {
                        0
                    } else {
                        (event.downloaded.saturating_mul(100) / event.total).min(99) as u32
                    };
                    set_download_status(|status| {
                        status.phase = "downloading".to_string();
                        status.progress = progress;
                        status.downloaded = event.downloaded;
                        status.total = event.total;
                        status.source = Some(event.source.to_string());
                    });
                    if progress != last_tooltip_percent
                        && (progress >= last_tooltip_percent + 1
                            || last_tooltip_at.elapsed() >= Duration::from_secs(1))
                    {
                        last_tooltip_percent = progress;
                        last_tooltip_at = std::time::Instant::now();
                        crate::tray::set_transient_tooltip(
                            &app_handle,
                            Some(crate::tray::download_progress_tooltip(progress)),
                        );
                    }
                    if event.total == 0 || progress > last_progress_percent || event.downloaded == 0
                    {
                        last_progress_percent = progress;
                        if let Some(win) = window.as_ref() {
                            emit_download_progress(
                                win,
                                progress,
                                event.total,
                                event.downloaded,
                                DownloadPhase::Downloading,
                                Some(event.source),
                            );
                        }
                    }
                }
            }
        },
    )
    .await
    .map_err(|error| {
        set_download_status(|status| {
            status.active = false;
            status.phase = "error".to_string();
            status.error = Some(error.to_string());
        });
        crate::tray::set_transient_tooltip(
            &app_handle,
            Some(crate::tray::download_failed_tooltip().to_string()),
        );
        clear_tooltip_later(app_handle.clone(), Duration::from_secs(20));
        let code = if error.is_setup() {
            DownloadErrorCode::SetupFailed
        } else {
            DownloadErrorCode::SourcesExhausted
        };
        DownloadCommandError::new(code, error.to_string())
    })?;

    if let Err(error) = validate_download_file_identity(
        &partial_path,
        expected_size,
        Some(&expected_sha256),
        asset_type.as_deref(),
    ) {
        let _ = tokio::fs::remove_file(&partial_path).await;
        set_download_status(|status| {
            status.active = false;
            status.phase = "error".to_string();
            status.error = Some(error.clone());
        });
        crate::tray::set_transient_tooltip(
            &app_handle,
            Some(crate::tray::download_failed_tooltip().to_string()),
        );
        clear_tooltip_later(app_handle.clone(), Duration::from_secs(20));
        return Err(DownloadCommandError::new(
            DownloadErrorCode::SourcesExhausted,
            error,
        ));
    }

    if let Some(ref win) = window {
        emit_download_progress(
            win,
            99,
            outcome.total,
            outcome.downloaded,
            DownloadPhase::Verifying,
            Some(outcome.source),
        );
    }
    if let Err(error) = finalize_download_file(&partial_path, &file_path).await {
        let _ = tokio::fs::remove_file(&partial_path).await;
        set_download_status(|status| {
            status.active = false;
            status.phase = "error".to_string();
            status.error = Some(error.detail.clone());
        });
        crate::tray::set_transient_tooltip(
            &app_handle,
            Some(crate::tray::download_failed_tooltip().to_string()),
        );
        clear_tooltip_later(app_handle.clone(), Duration::from_secs(20));
        return Err(error);
    }

    // 记录缓存标记：即使本次安装被取消，重启或下次检查时也可直接复用
    write_update_cache_marker(
        &filename,
        fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(outcome.downloaded),
    );

    preferences::record_used_update_source(&app_handle, outcome.source);
    // Mirror酱 was tried and the file still came from elsewhere: its download failed.
    if mirror_chosen && mirror_miss.is_none() && outcome.source != "Mirror酱" {
        mirror_miss = Some(mirrorchyan::Miss::Network);
        info!("Mirror酱 的下载没有成功，已改用 {}", outcome.source);
        set_download_status(|status| status.fallback_reason = Some(mirrorchyan::Miss::Network.word().to_string()));
    }
    preferences::record_update_source_miss(&app_handle, mirror_miss.map(mirrorchyan::Miss::word));
    info!(
        "✅ 下载完成: {} bytes，来源: {}",
        outcome.downloaded, outcome.source
    );
    set_download_status(|status| {
        status.active = false;
        status.phase = "complete".to_string();
        status.progress = 100;
        status.downloaded = outcome.downloaded;
        status.total = outcome.total;
        status.source = Some(outcome.source.to_string());
        status.path = Some(file_path.to_string_lossy().to_string());
        status.error = None;
    });
    crate::tray::set_transient_tooltip(
        &app_handle,
        Some(crate::tray::download_done_tooltip().to_string()),
    );
    clear_tooltip_later(app_handle.clone(), Duration::from_secs(10));
    if let Some(win) = window {
        emit_download_progress(
            &win,
            100,
            outcome.total,
            outcome.downloaded,
            DownloadPhase::Complete,
            Some(outcome.source),
        );
    }
    Ok(serde_json::json!({
        "success": true,
        "file_path": file_path.to_string_lossy().to_string(),
        "source": outcome.source,
        "fallback_reason": mirror_miss.map(mirrorchyan::Miss::word)
    }))
}

/// 查询更新安装包是否已在 temp 目录中完整下载（未命中返回 `None`）。
#[tauri::command]
pub async fn check_cached_update(
    filename: String,
    expected_size: Option<u64>,
    expected_sha256: Option<String>,
    asset_type: Option<String>,
) -> Option<String> {
    tokio::task::spawn_blocking(move || {
        if asset_type.as_deref() != Some("windows-x64-installer") {
            return None;
        }
        if !is_supported_installer_filename(&filename) {
            return None;
        }
        find_cached_installer_verified(
            &filename,
            expected_size,
            expected_sha256.as_deref(),
            asset_type.as_deref(),
        )
        .map(|path| path.to_string_lossy().to_string())
    })
    .await
    .unwrap_or_else(|error| {
        warn!("Cached installer verification failed: {error}");
        None
    })
}

#[cfg(test)]
mod download_source_tests {
    use super::*;

    #[test]
    fn mirror_candidate_is_first_and_the_cnb_github_order_is_kept() {
        let cnb = "https://cnb.cool/AlkaidLab/foundation-sunshine-release/-/releases/download/v1/setup.exe";
        let github = "https://github.com/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe";
        let mirror = github_download::DownloadCandidate {
            name: "Mirror酱",
            url: "https://mirrorchyan.com/resources/download/token".to_string(),
        };

        let with_mirror = download_candidates(Some(mirror), cnb, Some(github));
        let names: Vec<_> = with_mirror.iter().map(|candidate| candidate.name).collect();
        assert_eq!(names, ["Mirror酱", "CNB", "GitHub"]);

        let without_mirror = download_candidates(None, cnb, Some(github));
        let names: Vec<_> = without_mirror.iter().map(|candidate| candidate.name).collect();
        assert_eq!(names, ["CNB", "GitHub"]);
    }

    #[test]
    fn duplicate_fallback_url_is_not_added_twice() {
        let cnb = "https://cnb.cool/AlkaidLab/foundation-sunshine-release/-/releases/download/v1/setup.exe";
        let candidates = download_candidates(None, cnb, Some(cnb));
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn download_url_requires_an_allowlisted_foundation_release_host() {
        assert!(
            validate_download_url(
                "https://github.com/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe",
                None,
            )
            .is_ok()
        );
        assert!(validate_download_url(
            "https://cnb.cool/AlkaidLab/foundation-sunshine-release/-/releases/download/v1/setup.exe",
            None,
        )
        .is_ok());

        for invalid_url in [
            "http://github.com/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe",
            "https://github.com.example.invalid/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe",
            "https://github.com/AlkaidLab/other/releases/download/v1/setup.exe",
            "https://user:secret@github.com/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe",
            "not a url",
        ] {
            assert!(
                validate_download_url(invalid_url, None).is_err(),
                "unexpectedly accepted {invalid_url}"
            );
        }

        assert!(
            validate_download_url(
                "https://github.com/AlkaidLab/foundation-sunshine/releases/download/v1/setup.exe",
                Some("setup.exe"),
            )
            .is_ok()
        );
        assert!(
            validate_download_url(
                "https://github.com/AlkaidLab/foundation-sunshine/releases/download/v1/other.exe",
                Some("setup.exe"),
            )
            .is_err()
        );
    }

    #[test]
    fn installer_path_is_limited_to_a_valid_temp_file() {
        let temp_dir = std::env::temp_dir();
        let filename = format!("sunshine-installer-path-test-{}.exe", std::process::id());
        let file = temp_dir.join(&filename);
        let nested_dir = temp_dir.join(format!(
            "sunshine-installer-path-test-{}",
            std::process::id()
        ));
        let nested_file = nested_dir.join(&filename);
        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir_all(&nested_dir);

        fs::write(&file, b"MZtest").unwrap();
        fs::create_dir(&nested_dir).unwrap();
        fs::write(&nested_file, b"MZtest").unwrap();

        assert_eq!(validate_installer_path(&file, None, None).unwrap(), file);
        assert!(validate_installer_path(&nested_file, None, None).is_err());

        let expected_sha256 = format!("{:x}", sha2::Sha256::digest(b"MZtest"));
        assert_eq!(
            validate_installer_path(&file, Some(6), Some(&expected_sha256)).unwrap(),
            file
        );
        assert!(validate_installer_path(&file, Some(6), Some(&"0".repeat(64))).is_err());

        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir_all(&nested_dir);
    }

    #[test]
    fn test_channel_falls_back_when_newer_candidate_has_no_compatible_asset() {
        let asset_type = preferred_asset_type().to_string();
        let latest = ReleaseCandidate {
            version: "v2026.925.152547.杂鱼".to_string(),
            prerelease: false,
            github_release_id: Some(10),
            published_at: None,
            release_notes: String::new(),
            release_page: String::new(),
            assets: vec![ReleaseAsset {
                asset_type,
                name: "Sunshine-update.exe".to_string(),
                size: 1,
                sha256: "a".repeat(64),
                url: String::new(),
                fallback_url: None,
            }],
        };
        let pre_latest = ReleaseCandidate {
            version: "v2026.1001.112543.杂鱼".to_string(),
            prerelease: true,
            github_release_id: Some(11),
            ..latest.clone()
        };
        let mut pre_without_asset = pre_latest;
        pre_without_asset.assets.clear();
        let catalog = ReleaseCatalog {
            latest: Some(latest),
            pre_latest: Some(pre_without_asset),
            prerelease_unavailable: None,
        };

        let selected =
            select_compatible_candidate(&catalog, UpdateChannel::StableAndPrerelease).unwrap();
        assert!(!selected.prerelease);
    }

    #[test]
    fn download_filename_must_be_a_single_normal_component() {
        for valid_filename in ["Sunshine-Windows-1.2.3.exe", "archive..exe"] {
            assert!(
                validate_download_filename(valid_filename).is_ok(),
                "unexpectedly rejected {valid_filename}"
            );
        }

        for invalid_filename in [
            "",
            "   ",
            ".",
            "..",
            "../setup.exe",
            "..\\setup.exe",
            "folder/setup.exe",
            "folder\\setup.exe",
        ] {
            assert!(
                validate_download_filename(invalid_filename).is_err(),
                "unexpectedly accepted {invalid_filename}"
            );
        }
    }

    #[test]
    fn download_errors_expose_stable_language_neutral_codes() {
        let error = DownloadCommandError::new(
            DownloadErrorCode::SourcesExhausted,
            "diagnostic details stay out of the localized UI",
        );
        let serialized = serde_json::to_value(error).unwrap();

        assert_eq!(serialized["code"], "sources_exhausted");
        assert_eq!(
            serialized["detail"],
            "diagnostic details stay out of the localized UI"
        );
    }

    #[test]
    fn an_unreadable_installed_version_is_an_error_not_a_reason_to_offer_everything() {
        assert_eq!(known_current_version(Ok("2026.1112.025657".to_string())).unwrap(), "2026.1112.025657");
        for bad in [Err("host did not answer".to_string()), Ok(String::new()), Ok("  ".to_string())] {
            let error = known_current_version(bad).unwrap_err();
            assert!(error.starts_with(CURRENT_VERSION_UNKNOWN_ERROR), "{error}");
        }
        // the old fallback: 0.0.0 is older than every release, so every candidate looked like an update
        let release = ReleaseCandidate {
            version: "v2026.1112.25657.杂鱼".to_string(),
            prerelease: false,
            github_release_id: None,
            published_at: None,
            release_notes: String::new(),
            release_page: String::new(),
            assets: Vec::new(),
        };
        assert!(!candidate_is_newer_than("Unknown", &release));
    }
}

#[cfg(test)]
mod update_cache_tests {
    use super::*;

    #[test]
    fn cached_installer_reuse_and_marker_roundtrip() {
        let filename = format!("sunshine-cache-test-{}.exe", std::process::id());
        let file_path = std::env::temp_dir().join(&filename);
        let _ = fs::remove_file(&file_path);
        let _ = fs::remove_file(update_cache_marker_path());

        // 文件不存在时不应命中
        assert!(find_cached_installer(&filename, Some(123)).is_none());

        fs::write(&file_path, b"payload-payload").unwrap();
        let size = fs::metadata(&file_path).unwrap().len();

        // 大小一致才复用
        assert!(find_cached_installer(&filename, Some(size)).is_some());
        assert!(find_cached_installer(&filename, Some(size + 1)).is_none());

        // 同名且同大小但内容变化时，SHA-256 不匹配，不能复用旧缓存。
        fs::write(&file_path, b"MZold").unwrap();
        let content_size = 5;
        let old_sha256 = format!("{:x}", sha2::Sha256::digest(b"MZold"));
        let new_sha256 = format!("{:x}", sha2::Sha256::digest(b"MZnew"));
        assert!(
            find_cached_installer_verified(
                &filename,
                Some(content_size),
                Some(&old_sha256),
                Some("windows-x64-installer"),
            )
            .is_some()
        );
        fs::write(&file_path, b"MZnew").unwrap();
        assert!(
            find_cached_installer_verified(
                &filename,
                Some(content_size),
                Some(&old_sha256),
                Some("windows-x64-installer"),
            )
            .is_none()
        );
        assert!(!file_path.exists(), "SHA-256 不匹配的缓存应被删除");
        fs::write(&file_path, b"MZnew").unwrap();
        assert!(
            find_cached_installer_verified(
                &filename,
                Some(content_size),
                Some(&new_sha256),
                Some("windows-x64-installer"),
            )
            .is_some()
        );
        fs::write(&file_path, b"payload-payload").unwrap();

        // 期望大小缺省（或为 0）时回退到缓存标记
        write_update_cache_marker(&filename, size);
        assert!(find_cached_installer(&filename, None).is_some());
        assert!(find_cached_installer(&filename, Some(0)).is_some());

        // 非法文件名始终不命中
        assert!(find_cached_installer("../escape.exe", Some(size)).is_none());

        write_update_cache_marker("Sunshine-1.2.3.exe", 42);
        let marker = read_update_cache_marker().unwrap();
        assert_eq!(marker.filename, "Sunshine-1.2.3.exe");
        assert_eq!(marker.size, 42);

        // 损坏的标记文件按未命中处理
        fs::write(update_cache_marker_path(), "not json").unwrap();
        assert!(read_update_cache_marker().is_none());
        assert!(find_cached_installer(&filename, None).is_none());

        // 标记大小回退路径同样受保留期约束；调用方传入的期望值不受影响
        let stale = CachedInstallerMarker {
            filename: filename.clone(),
            size,
            downloaded_at: get_current_timestamp() - UPDATE_CACHE_MAX_AGE_SECS - 60,
        };
        fs::write(
            update_cache_marker_path(),
            serde_json::to_string(&stale).unwrap(),
        )
        .unwrap();
        assert!(find_cached_installer(&filename, None).is_none());
        assert!(find_cached_installer(&filename, Some(size)).is_some());

        // 清理保护判定：过期标记 → 安装包与标记一并清除
        let cleanup_dir =
            std::env::temp_dir().join(format!("sunshine-cache-test-dir-{}", std::process::id()));
        fs::create_dir_all(&cleanup_dir).unwrap();
        let cached_installer = cleanup_dir.join(&filename);
        fs::write(&cached_installer, b"payload").unwrap();

        assert!(protected_installer_for_cleanup(&cleanup_dir).is_none());
        assert!(!cached_installer.exists(), "过期安装包应被删除");
        assert!(read_update_cache_marker().is_none(), "过期标记应被删除");

        // 有效标记 → 返回受保护的文件名
        fs::write(&cached_installer, b"payload").unwrap();
        write_update_cache_marker(&filename, size);
        assert_eq!(
            protected_installer_for_cleanup(&cleanup_dir).as_deref(),
            Some(filename.as_str())
        );

        // 非法文件名的标记 → 标记被清除且不保护任何路径
        write_update_cache_marker("../escape.exe", size);
        assert!(protected_installer_for_cleanup(&cleanup_dir).is_none());
        assert!(read_update_cache_marker().is_none());

        // 标记指向的文件丢失 → 标记被清除
        write_update_cache_marker(&filename, size);
        let _ = fs::remove_file(&cached_installer);
        assert!(protected_installer_for_cleanup(&cleanup_dir).is_none());
        assert!(read_update_cache_marker().is_none());

        // 时钟回拨（downloaded_at 晚于当前时间）按过期处理：
        // 标记大小不参与回退，清理也不持续保护
        fs::write(&cached_installer, b"payload").unwrap();
        let future = CachedInstallerMarker {
            filename: filename.clone(),
            size,
            downloaded_at: get_current_timestamp() + 3600,
        };
        fs::write(
            update_cache_marker_path(),
            serde_json::to_string(&future).unwrap(),
        )
        .unwrap();
        assert!(find_cached_installer(&filename, None).is_none());
        // 调用方期望值路径不读标记，不受未来时间戳影响
        assert!(find_cached_installer(&filename, Some(size)).is_some());
        assert!(protected_installer_for_cleanup(&cleanup_dir).is_none());
        assert!(!cached_installer.exists());
        assert!(read_update_cache_marker().is_none());

        let _ = fs::remove_dir_all(&cleanup_dir);
        let _ = fs::remove_file(&file_path);
        let _ = fs::remove_file(update_cache_marker_path());
    }
}

// ========== 安装相关 ==========

#[cfg(all(test, target_os = "windows"))]
mod updater_staging_tests {
    use super::*;

    #[test]
    fn unlaunched_handoff_is_cleaned_but_launched_handoff_survives() {
        let (_, state_path, _, guard) =
            prepare_updater_helper("unused.exe", "exe", None, &"a".repeat(64), None, None).unwrap();
        let directory = guard.path().to_path_buf();
        assert!(state_path.is_file());
        drop(guard);
        assert!(!directory.exists());

        let (_, state_path, _, guard) =
            prepare_updater_helper("unused.exe", "exe", None, &"a".repeat(64), None, None).unwrap();
        let directory = guard.keep();
        assert!(state_path.is_file());
        assert_eq!(
            updater_cleanup_dir_for_result(&directory.join("result.json")),
            Some(directory.canonicalize().unwrap())
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn staged_executable_is_owned_until_exit_and_failed_verification_cleans_up() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("安装包 ' with spaces.exe");
        let destination = root.path().join("目标目录 ' with spaces");
        fs::create_dir(&destination).unwrap();
        fs::write(&source, b"MZtest").unwrap();
        let state = UpdaterHelperState {
            installer_path: source.to_string_lossy().into_owned(),
            extension: "exe".into(),
            expected_size: Some(6),
            expected_sha256: format!("{:x}", sha2::Sha256::digest(b"MZtest")),
            target_version: None,
            gui_exe_path: String::new(),
            result_path: String::new(),
            parent_pid: std::process::id(),
            dark_theme: None,
        };
        let staged = stage_installer_in_directory(&state, &destination).unwrap();
        assert_eq!(staged.extension().unwrap(), "exe");
        assert_eq!(fs::read(&staged).unwrap(), b"MZtest");
        let path = staged.to_path_buf();
        drop(staged);
        assert!(!path.exists());

        fs::write(&source, b"MZevil").unwrap();
        assert!(stage_installer_in_directory(&state, &destination).is_err());
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        fs::remove_file(&source).unwrap();
        assert!(stage_installer_in_directory(&state, &destination).is_err());
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    }
}

#[cfg(target_os = "windows")]
fn prepare_updater_helper(
    file_path: &str,
    extension: &str,
    expected_size: Option<u64>,
    expected_sha256: &str,
    target_version: Option<String>,
    dark_theme: Option<bool>,
) -> Result<(PathBuf, PathBuf, UpdaterHelperState, tempfile::TempDir), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("get current exe failed: {}", e))?;
    let work_dir = tempfile::Builder::new()
        .prefix("sunshine-updater-")
        .tempdir()
        .map_err(|error| format!("create updater directory failed: {error}"))?;

    // The helper is the installed GUI executable itself. Keeping the elevated
    // entry point outside the user-writable temp directory prevents a medium
    // integrity process from replacing the helper before UAC approval.
    let helper_exe = current_exe.clone();

    let state_path = work_dir.path().join("state.json");
    let result_path = work_dir.path().join("result.json");
    let state = UpdaterHelperState {
        installer_path: file_path.to_string(),
        extension: extension.to_string(),
        expected_size,
        expected_sha256: expected_sha256.to_string(),
        target_version,
        gui_exe_path: current_exe.to_string_lossy().to_string(),
        result_path: result_path.to_string_lossy().to_string(),
        parent_pid: std::process::id(),
        dark_theme,
    };

    let state_content = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("serialize updater state failed: {}", e))?;
    fs::write(&state_path, state_content)
        .map_err(|e| format!("write updater state failed: {}", e))?;

    Ok((helper_exe, state_path, state, work_dir))
}

#[cfg(target_os = "windows")]
fn launch_updater_helper(
    helper_exe: &Path,
    state_path: &Path,
    state: &UpdaterHelperState,
) -> Result<(), String> {
    let expected_size = state
        .expected_size
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    crate::elevation::launch_elevated(
        helper_exe.as_os_str(),
        &[
            UPDATER_HELPER_ARG,
            &state_path.to_string_lossy(),
            &state.installer_path,
            &expected_size,
            &state.expected_sha256,
            &state.gui_exe_path,
            &state.result_path,
        ],
        // Keep the progress helper visible after UAC approval.
        windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL.0,
    )
    .map(|_| ())
    .map_err(|error| format!("launch updater helper elevated failed: {error}"))
}

/// Repair an incomplete install: download the installer of the newest release of the person's
/// channel and run it over the installed copy, whatever version is installed. The same path as an
/// update (download, helper copy, installer), so the Control Panel leaves while it runs and comes
/// back afterwards.
pub(crate) async fn repair_installation(app: AppHandle) -> Result<(), String> {
    let channel = if get_include_prerelease(&app) {
        UpdateChannel::StableAndPrerelease
    } else {
        UpdateChannel::StableOnly
    };
    let catalog = resolve_catalog(channel).await?;
    let candidate = select_compatible_candidate(&catalog, channel)
        .ok_or_else(|| format!("未找到包含 {} 更新资产的发布版本", preferred_asset_type()))?;
    let info = create_update_info(&candidate);
    let (Some(url), Some(name), Some(sha256), Some(kind)) = (
        info.download_url.clone(),
        info.download_name.clone(),
        info.download_sha256.clone(),
        info.download_type.clone(),
    ) else {
        return Err("the release has no installer to repair with".to_string());
    };
    let downloaded = download_update(
        url,
        info.download_fallback_url.clone(),
        name,
        info.download_size,
        Some(sha256.clone()),
        Some(kind),
        Some(info.version.clone()),
        app.clone(),
    )
    .await
    .map_err(|error| error.detail)?;
    let file_path = downloaded
        .get("file_path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "the download did not report a file".to_string())?
        .to_string();
    install_update(
        file_path,
        Some(info.version),
        info.download_size,
        Some(sha256),
        None,
        app,
    )
    .await
}

#[tauri::command]
pub async fn repair_installation_command(app: AppHandle) -> Result<(), String> {
    repair_installation(app).await
}

/// 安装更新文件
#[tauri::command]
pub async fn install_update(
    file_path: String,
    target_version: Option<String>,
    expected_size: Option<u64>,
    expected_sha256: Option<String>,
    dark_theme: Option<bool>,
    app_handle: AppHandle,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        info!("🔧 开始安装更新: {}", file_path);

        // 先启动更新助手；如果用户取消 UAC，串流仍然保持运行。
        emit_install_progress(&app_handle, "preparing", None, false);
        let expected_sha256 = match expected_sha256.as_deref().and_then(normalize_sha256) {
            Some(value) => value,
            None => {
                let error = "安装前缺少有效的更新 SHA-256".to_string();
                emit_install_progress(&app_handle, "failed", Some(&error), true);
                return Err(error);
            }
        };
        let path = match validate_installer_path(
            Path::new(&file_path),
            expected_size,
            Some(&expected_sha256),
        ) {
            Ok(path) => path,
            Err(error) => {
                emit_install_progress(&app_handle, "failed", Some(&error), true);
                return Err(error);
            }
        };
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        emit_install_progress(&app_handle, "building-command", None, false);
        let (helper_exe, state_path, state, work_dir) = match prepare_updater_helper(
            &path.to_string_lossy(),
            &extension,
            expected_size,
            &expected_sha256,
            target_version,
            dark_theme,
        ) {
            Ok(paths) => paths,
            Err(e) => {
                emit_install_progress(&app_handle, "failed", Some(&e), true);
                return Err(e);
            }
        };

        emit_install_progress(&app_handle, "launching-installer", None, false);
        if let Err(e) = launch_updater_helper(&helper_exe, &state_path, &state) {
            emit_install_progress(&app_handle, "failed", Some(&e), true);
            return Err(e);
        }
        // The helper and restarted GUI now own cleanup. A failed launch above
        // drops the guard and removes the unpublished state automatically.
        let _ = work_dir.keep();
        emit_install_progress(&app_handle, "installer-started", None, false);

        // The panel leaves from here on (the delayed exit below, or the installer closing this process by
        // name): WebView2 process endings from now on are part of the handoff, not failures to report.
        crate::exit_state::begin("update handoff");

        info!("✅ 安装程序已静默启动，正在安装...");

        // 延迟后退出当前GUI进程
        let app_clone = app_handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            emit_install_progress(&app_clone, "app-exiting", None, false);
            tokio::time::sleep(Duration::from_secs(1)).await;
            // Deliberate handoff: the installer starts the next GUI. Exit code 0
            // would make the host treat this as a killed agent and launch another
            // copy while the installer is replacing files.
            app_clone.exit(crate::gui_agent_exit::GUI_AGENT_DELIBERATE_QUIT_EXIT_CODE);
        });

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("此功能仅支持Windows".to_string())
    }
}

fn update_cache_marker_path() -> PathBuf {
    std::env::temp_dir().join(UPDATE_CACHE_MARKER_FILE)
}

fn read_update_cache_marker() -> Option<CachedInstallerMarker> {
    let raw = fs::read_to_string(update_cache_marker_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_update_cache_marker(filename: &str, size: u64) {
    let marker = CachedInstallerMarker {
        filename: filename.to_string(),
        size,
        downloaded_at: get_current_timestamp(),
    };
    let raw = match serde_json::to_string(&marker) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("⚠️ 序列化更新缓存标记失败: {}", e);
            return;
        }
    };
    if let Err(e) = fs::write(update_cache_marker_path(), raw) {
        warn!("⚠️ 写入更新缓存标记失败: {}", e);
    }
}

/// 决定本次清理应保护的安装包，返回其文件名。
///
/// 标记文件位于 temp 目录、内容不完全受我们控制，因此文件名必须先通过
/// `validate_download_filename` 才能据此拼路径或删除文件；标记非法、
/// 过期或指向的文件已丢失时解除保护并清掉标记本身。
fn protected_installer_for_cleanup(temp_dir: &Path) -> Option<String> {
    let marker = read_update_cache_marker()?;

    if validate_download_filename(&marker.filename).is_err() {
        let _ = fs::remove_file(update_cache_marker_path());
        return None;
    }

    let cached_path = temp_dir.join(&marker.filename);
    // 时钟回拨（downloaded_at 晚于当前时间）按过期处理，不无限期保护
    let expired = get_current_timestamp()
        .checked_sub(marker.downloaded_at)
        .is_none_or(|age| age > UPDATE_CACHE_MAX_AGE_SECS);
    if expired || !cached_path.exists() {
        let _ = fs::remove_file(&cached_path);
        let _ = fs::remove_file(update_cache_marker_path());
        return None;
    }

    Some(marker.filename)
}

/// 返回已完整下载且大小一致的安装包路径，命中时无需重新下载。
///
/// 大小以调用方传入的期望值为准——它来自当前这次更新检查，天然新鲜；
/// 缺省时回退到缓存标记中记录的大小，此时与清理逻辑同样受保留期约束。
/// 无法确定期望大小时保守地视为未命中。
fn find_cached_installer(filename: &str, expected_size: Option<u64>) -> Option<PathBuf> {
    if validate_download_filename(filename).is_err() {
        return None;
    }

    let expected = expected_size.filter(|size| *size > 0).or_else(|| {
        // 标记大小仅作回退，过期后不再据此复用
        let marker = read_update_cache_marker()?;
        let fresh = get_current_timestamp()
            .checked_sub(marker.downloaded_at)
            .is_some_and(|age| age <= UPDATE_CACHE_MAX_AGE_SECS);
        (fresh && marker.filename == filename).then_some(marker.size)
    })?;
    let file_path = std::env::temp_dir().join(filename);
    let actual = fs::metadata(&file_path).ok()?.len();

    (actual == expected).then_some(file_path)
}

fn find_cached_installer_verified(
    filename: &str,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
    asset_type: Option<&str>,
) -> Option<PathBuf> {
    // A cached file must be checked against the current release identity. A
    // size-only match is not sufficient because two releases may reuse a
    // filename and have the same byte length.
    let expected_sha256 = normalize_sha256(expected_sha256?)?;
    let path = find_cached_installer(filename, expected_size)?;
    if validate_download_file_identity(&path, expected_size, Some(&expected_sha256), asset_type)
        .is_ok()
    {
        return Some(path);
    }
    let _ = fs::remove_file(&path);
    None
}

/// 清理临时目录中的旧安装包
fn cleanup_old_installers() {
    let temp_dir = std::env::temp_dir();

    // 用户取消安装后保留已下载的安装包，避免下次被迫重新下载；
    // 标记非法、过期或文件已丢失时解除保护。
    let protected = protected_installer_for_cleanup(&temp_dir);

    info!("🧹 检查并清理临时目录中的旧安装包...");

    let entries = match std::fs::read_dir(&temp_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("⚠️ 无法读取临时目录: {}", e);
            return;
        }
    };

    let cleaned_count = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension()?.to_str()?.to_lowercase();

            if !matches!(ext.as_str(), "msi" | "exe") {
                return None;
            }

            let file_name = path.file_name()?.to_str()?;

            // 已下载待安装的安装包不清理
            if protected.as_deref() == Some(file_name) {
                return None;
            }

            let file_name_lower = file_name.to_lowercase();

            // 检查是否包含 sunshine 相关关键词
            let is_sunshine_installer =
                file_name_lower.contains("sunshine") || file_name_lower.starts_with("sunshine-");

            if !is_sunshine_installer {
                return None;
            }

            match std::fs::remove_file(&path) {
                Ok(_) => {
                    info!("✅ 已删除旧安装包: {}", file_name);
                    Some(())
                }
                Err(e) => {
                    debug!("⚠️ 无法删除 {}: {} (可能正在使用中)", file_name, e);
                    None
                }
            }
        })
        .count();

    if cleaned_count > 0 {
        info!("✅ 清理完成，共删除 {} 个旧安装包", cleaned_count);
    } else {
        debug!("✅ 未发现需要清理的旧安装包");
    }
}

/// 初始化更新偏好状态。
///
/// 必须在任何窗口或异步更新任务启动前调用，不能依赖主 WebView 是否在启动时创建。
pub fn init_update_preferences(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    preferences::init(app)
}

/// 启动清理和自动更新检查；偏好状态由 `init_update_preferences` 独立初始化。
pub fn start_update_checker<R: Runtime + 'static>(
    app: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    if UPDATE_CHECKER_STARTED.set(()).is_err() {
        debug!("更新检查器已启动，跳过重复初始化");
        return Ok(());
    }

    // 在启动时清理旧的安装包（在检查更新之前）
    cleanup_old_installers();

    // 延迟自动检查更新：主要是等待主窗口/前端就绪（偏好已在后端启动时加载）
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        check_for_updates_on_startup(app_clone);
    });

    Ok(())
}

/// 主窗口按需创建并完成事件监听后启动自动检查。
#[tauri::command]
pub fn start_update_checker_when_ui_ready(app: tauri::AppHandle) -> Result<(), String> {
    start_update_checker(&app).map_err(|error| error.to_string())
}

// Windows path syntax: a backslash separates folders only on Windows.
#[cfg(all(test, target_os = "windows"))]
mod updater_copy_name_tests {
    use super::*;

    #[test]
    fn only_the_helper_copy_in_the_updates_folder_counts_as_a_copy() {
        let copy = Path::new(r"C:\Program Files\Sunshine\updates\updater-4242\sunshine-updater.exe");
        assert!(is_updater_copy_path(copy));
        assert!(!is_updater_copy_path(Path::new(
            r"C:\Program Files\Sunshine\assets\gui\sunshine-gui.exe"
        )));
        assert!(!is_updater_copy_path(Path::new(
            r"C:\Program Files\Sunshine\updates\sunshine-update-abc.exe"
        )));
        assert!(!is_updater_copy_path(Path::new(
            r"C:\Program Files\Sunshine\other\updater-1\sunshine-updater.exe"
        )));
    }

    #[test]
    fn a_copy_never_has_the_name_the_installer_ends_by_name() {
        // The installer ends sunshine-gui.exe by name; the copy has to outlive that.
        assert_ne!(UPDATER_COPY_EXE.to_ascii_lowercase(), "sunshine-gui.exe");
    }
}
