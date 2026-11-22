use crate::ctx_popup;
use crate::proxy_server;
use crate::sunshine;
use crate::toolbar;
use crate::tray;
use crate::update;
use crate::windows;
use log::{debug, error, info, warn};
use tauri::{App, AppHandle, Emitter, Manager};

pub(crate) const TOOLBAR_SHORTCUT: &str = "CmdOrCtrl+Shift+Alt+T";

// Tauri owns these handles for this application instance. Drop also cancels
// tasks if setup returns early, before ownership is transferred to app state.
struct BackgroundTasks(Vec<tauri::async_runtime::JoinHandle<()>>);

impl BackgroundTasks {
    fn abort(&self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

impl Drop for BackgroundTasks {
    fn drop(&mut self) {
        self.abort();
    }
}

/// 应用程序初始化设置
pub fn setup_application(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let show_toolbar = args.iter().any(|arg| arg == "--toolbar" || arg == "-t");
    let show_desktop = args.iter().any(|arg| arg == "--desktop" || arg == "-d");
    // The Sunshine host runs `--close-desktop` when a stream of its Big Screen entry ends. With no panel running there is nothing to close:
    // do not start one (and do not block the undo command of the host, which waits for this process).
    if args.iter().any(|arg| arg == "--close-desktop") {
        info!("🖥️ --close-desktop：没有正在运行的面板，无需关闭桌面 UI");
        app.handle().exit(0);
        return Ok(());
    }
    // Recover an interrupted user-session transaction even if WebView2 is
    // unavailable. The guard also covers a later setup error.
    #[cfg(target_os = "windows")]
    let touch_keyboard_guard = (!crate::test_isolation::active()).then(crate::touch_keyboard::start);
    let agent_only = args.iter().any(|arg| arg == "--hidden");
    let explicit_minimized = args.iter().any(|arg| arg == "--minimized");
    // `--start-guide` is the installer's "open the start guide" button. It shows the main window
    // even when the panel is set to start minimized. `--hidden` stays an agent with no window.
    // panel-tauri-single-instance: a second launch is delivered in handle_single_instance.
    let start_guide = !agent_only && crate::start_guide::requests_start_guide(&args);
    if start_guide {
        crate::start_guide::queue_start_guide();
    }
    let desktop_settings = crate::desktop_settings::load_desktop_settings_from_disk();
    let send_to_client_paths = crate::file_transfer::parse_send_to_client_args(&args);
    let is_send_to_client = !send_to_client_paths.is_empty();
    let quick_share_folder_paths = crate::file_mapping::parse_quick_share_folder_args(&args);
    let is_quick_share_folder = !quick_share_folder_paths.is_empty();
    let url_contains_pin = args
        .iter()
        .find(|arg| arg.starts_with("--url="))
        .map_or(false, |arg| arg.contains("/pin"));

    let app_handle = app.handle().clone();
    if !crate::webview_runtime::ensure_available() {
        info!("WebView2 Runtime is unavailable; native fallback was shown");
        // process::exit skips destructors; wait for the recovery worker before
        // leaving this early-exit path.
        #[cfg(target_os = "windows")]
        drop(touch_keyboard_guard);
        std::process::exit(crate::webview_runtime::WEBVIEW2_MISSING_EXIT_CODE);
    }
    if !crate::test_isolation::active() {
        windows::register_agent_restart();
        crate::desktop_settings::apply_startup_settings(&app_handle, &desktop_settings);
    }
    // 更新偏好属于应用级状态，必须在 `--hidden` 的纯托盘代理模式下也立即注册。
    // 自动联网检查仍在后面按窗口启动模式决定，避免把状态初始化和网络行为耦合。
    update::init_update_preferences(&app_handle)?;
    if !crate::test_isolation::active() {
        crate::install_integrity::start(app_handle.clone());
        crate::clipboard::auto_start();
        #[cfg(target_os = "windows")]
        crate::text_context::start(desktop_settings.text_context_enabled);
    }
    let mut tasks = BackgroundTasks(vec![windows::lifecycle::start_monitor(app_handle.clone())]);
    #[cfg(target_os = "windows")]
    if !crate::test_isolation::active() {
        crate::vdd::start_hardware_cursor_migration();
    }
    // a measurement instance (SUNSHINE_GUI_TEST_OFFSCREEN) shows its window, off screen, whatever the person's setting: a page of a hidden window sleeps
    let start_minimized = (explicit_minimized || desktop_settings.start_minimized)
        && !windows::test_offscreen()
        && !start_guide;

    // Agent-only startup keeps tray and session services alive without
    // allocating a hidden WebView. UI windows are created on demand.
    if agent_only {
        info!("Starting Sunshine user agent without WebView windows");
    } else if show_desktop {
        info!("🖥️ 检测到 --desktop 参数，启动桌面 UI 模式");
        if start_minimized {
            windows::create_desktop_window_hidden(&app_handle)?;
        } else {
            windows::create_desktop_window(&app_handle)?;
        }
        windows::create_main_window_hidden(&app_handle)?;
    } else if !show_toolbar && !url_contains_pin && !is_send_to_client && !is_quick_share_folder {
        if start_minimized {
            windows::create_main_window_hidden(&app_handle)?;
        } else {
            windows::create_main_window(&app_handle)?;
        }
    }

    if start_guide {
        info!("--start-guide: opening the start guide at its first step");
        if let Some(window) = app_handle.get_webview_window("main") {
            windows::show_and_activate_window(&window);
            let _ = window.emit("start-guide-pending", ());
        } else {
            windows::create_main_window(&app_handle)?;
        }
    }

    if !agent_only {
        // The pages ask for these probes the moment they mount (0.05 to 1.7 s each on the Windows test VM); started now, they are running or kept by then.
        tauri::async_runtime::spawn(async {
            let _ = tokio::join!(
                crate::vdd::get_vdd_status(),
                crate::vmouse::get_vmouse_status(),
                crate::sunshine::get_sunshine_version(),
                crate::system::get_sunshine_start_time(),
            );
        });
    }
    // the tray's menus open in the menu window (ctx_popup.rs): the icon's callbacks reach the app through this handle, and the window is made hidden, as for the main window's context menus
    ctx_popup::remember_app(&app_handle);
    tray::create_system_tray(&app_handle)?;
    // The menu window is made at every start. A tray-only start (agent_only) has no main window to wait for, and the window is the only surface of the tray's menu: without it the icon stayed on the
    // native menu until someone happened to open the main window (the log: 20 minutes to 5 hours of "Starting Sunshine user agent without WebView windows" before "menu page ready").
    if agent_only {
        ctx_popup::schedule_create_after(&app_handle, std::time::Duration::ZERO);
    } else {
        ctx_popup::schedule_create(&app_handle);
        // the main window's first view normally makes it (panel_content_ready); if that is late, a click on the icon in the first seconds must still find the Material menu coming
        ctx_popup::schedule_create_after(&app_handle, std::time::Duration::from_secs(6));
    }
    crate::mem_adaptive::start(&app_handle);
    crate::launch_splash::start_for_measurement(&app_handle);
    crate::power::refresh_ecoqos_state(&app_handle);
    if !crate::test_isolation::active() {
        #[cfg(target_os = "windows")]
        if let Err(e) = crate::shell_context_menu::install_file_transfer_menu() {
            warn!("⚠️  安装文件传输右键菜单失败: {}", e);
        }
        #[cfg(target_os = "windows")]
        if desktop_settings.file_mapping_menu_enabled {
            if let Err(e) = crate::shell_context_menu::install_file_mapping_menu() {
                warn!("install file mapping context menu failed: {}", e);
            }
        }
        register_global_shortcuts(app, desktop_settings.toolbar_shortcut_enabled)?;
        crate::nr_overlay::initialize(&app_handle);
    }
    setup_menu_event_handler(app);

    if is_send_to_client {
        crate::file_transfer::dispatch_cli_send(send_to_client_paths);
    }
    if is_quick_share_folder {
        crate::file_mapping::dispatch_cli_quick_share(quick_share_folder_paths);
    }

    // 延迟任务
    tasks.0.push(tauri::async_runtime::spawn(async move {
        update::emit_update_result_if_requested(&app_handle);

        // PIN 配对窗口
        if url_contains_pin {
            info!("🔐 将在应用启动后打开 PIN 配对窗口");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = windows::open_pin_window(&app_handle) {
                error!("❌ 创建 PIN 配对窗口失败: {}", e);
            }
        }

        // 工具栏窗口（非桌面模式下）
        if show_toolbar && !show_desktop {
            info!("🔧 将在应用启动后打开工具栏");
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            if let Err(e) = toolbar::create_toolbar_window_internal(&app_handle) {
                error!("❌ 创建工具栏失败: {}", e);
            }
        }
    }));
    // The status probes the pages ask for (the ones with a cache: each starts a process or a WMI query, 0.05 to 1.9 s on the Windows test VM, 3.4 s at the start) are run once now, one after the other in the
    // background, so the first entry of a page finds their answer in the cache (probe_cache.rs) instead of waiting for them. Not in an agent-only start: nobody looks at a page there.
    if !agent_only {
        tasks.0.push(tauri::async_runtime::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            let _ = crate::sunshine::get_sunshine_build_info().await;
            let _ = crate::vdd::get_vdd_status().await;
            let _ = crate::vigem::get_vigem_status().await;
        }));
    }
    #[cfg(target_os = "windows")]
    if let Some(guard) = touch_keyboard_guard {
        app.manage(guard);
    }
    app.manage(tasks);

    Ok(())
}

pub fn shutdown_application(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    if !crate::test_isolation::active() {
        crate::touch_keyboard::stop();
    }
    if let Some(tasks) = app.try_state::<BackgroundTasks>() {
        tasks.abort();
    }
    windows::lifecycle::shutdown();
    if !crate::test_isolation::active() {
        #[cfg(target_os = "windows")]
        crate::text_context::stop();
        crate::clipboard::stop();
        #[cfg(target_os = "windows")]
        crate::tray::cleanup_prevent_sleep();
        crate::moonlight_web::cleanup();
        crate::windows::unregister_agent_restart();
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn explicit_exit_and_failed_setup_both_cancel_background_tasks() {
        for explicit in [true, false] {
            let (mut jobs, mut cancellations) = (Vec::new(), Vec::new());
            for _ in 0..2 {
                let (done, cancelled) = tokio::sync::oneshot::channel::<()>();
                jobs.push(tauri::async_runtime::JoinHandle::Tokio(tokio::spawn(
                    async move {
                        std::future::pending::<()>().await;
                        drop(done);
                    },
                )));
                cancellations.push(cancelled);
            }
            let owner = BackgroundTasks(jobs);
            if explicit {
                owner.abort();
            } else {
                drop(owner);
            }
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                for cancelled in cancellations {
                    assert!(cancelled.await.is_err());
                }
            })
            .await
            .unwrap();
        }
    }
}

/// 注册全局快捷键
fn register_global_shortcuts(
    app: &mut App,
    toolbar_shortcut_enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !toolbar_shortcut_enabled {
        info!("Desktop pet shortcut is disabled; skipping registration");
        return Ok(());
    }

    let registered = sync_toolbar_shortcut(app.handle(), true).map_err(std::io::Error::other)?;
    if !registered {
        warn!(
            "Desktop pet shortcut is currently unavailable; the application will continue normally"
        );
    }

    Ok(())
}

/// 返回桌宠快捷键是否已由当前 GUI 注册。
pub fn toolbar_shortcut_is_registered(app_handle: &AppHandle) -> bool {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    app_handle.global_shortcut().is_registered(TOOLBAR_SHORTCUT)
}

/// 按用户设置注册或注销桌宠快捷键。
///
/// 启用时若组合键被其他程序占用，会保留用户的启用意图并返回 `Ok(false)`，
/// 让后续打开设置页或重启 GUI 时可以再次尝试注册。
pub fn sync_toolbar_shortcut(app_handle: &AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let shortcut_manager = app_handle.global_shortcut();
    if !enabled {
        if !shortcut_manager.is_registered(TOOLBAR_SHORTCUT) {
            return Ok(false);
        }

        shortcut_manager
            .unregister(TOOLBAR_SHORTCUT)
            .map_err(|error| format!("failed to unregister toolbar shortcut: {error}"))?;
        info!("Desktop pet shortcut unregistered: CTRL+SHIFT+ALT+T");
        return Ok(false);
    }

    if shortcut_manager.is_registered(TOOLBAR_SHORTCUT) {
        return Ok(true);
    }

    let app_handle = app_handle.clone();
    match shortcut_manager.on_shortcut(TOOLBAR_SHORTCUT, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            debug!("Global shortcut triggered: CTRL+SHIFT+ALT+T");
            // The shortcut opens the toolbar, whose actions lead to the main panel.
            // Begin warming on the existing Pressed event (panel-memory-tauri-shortcut, Usage).
            if app_handle.get_webview_window("toolbar").is_none() {
                crate::mem_adaptive::prewarm_main(&app_handle, "toolbar shortcut pressed");
            }
            toggle_toolbar_window(&app_handle);
        }
    }) {
        Ok(_) => {
            info!("Desktop pet shortcut registered: CTRL+SHIFT+ALT+T");
            Ok(true)
        }
        Err(error) => {
            warn!(
                "Desktop pet shortcut registration failed; it may be used by another application: {error}"
            );
            Ok(false)
        }
    }
}

/// 切换工具栏窗口显示/隐藏
fn toggle_toolbar_window(app_handle: &AppHandle) {
    if let Some(toolbar_window) = app_handle.get_webview_window("toolbar") {
        debug!("🔧 工具栏已存在，关闭");
        let _ = toolbar_window.close();
    } else {
        debug!("🔧 工具栏不存在，创建");
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = toolbar::create_toolbar_window_internal(&app_clone) {
                error!("❌ 快捷键创建工具栏失败: {}", e);
            }
        });
    }
}

/// 设置全局菜单事件处理器
fn setup_menu_event_handler(app: &mut App) {
    let app_handle = app.handle().clone();
    app.handle().on_menu_event(move |_app, event| {
        let event_id = event.id().as_ref();
        if event_id.starts_with("toolbar_") {
            debug!("🔧 全局菜单事件: {:?}", event.id());
            toolbar::handle_toolbar_menu_event(&app_handle, event_id);
        }
    });
}

/// 处理单实例逻辑
pub fn handle_single_instance(app: &AppHandle, args: Vec<String>) {
    info!("🔔 检测到第二个实例启动，激活现有窗口");
    info!("   启动参数: {:?}", args);

    let send_to_client_paths = crate::file_transfer::parse_send_to_client_args(&args);
    if !send_to_client_paths.is_empty() {
        info!("📤 检测到文件传输右键菜单请求");
        crate::file_transfer::dispatch_cli_send(send_to_client_paths);
        return;
    }

    // 诊断：列出当前所有窗口
    let quick_share_folder_paths = crate::file_mapping::parse_quick_share_folder_args(&args);
    if !quick_share_folder_paths.is_empty() {
        info!("file mapping quick share request detected");
        crate::file_mapping::dispatch_cli_quick_share(quick_share_folder_paths);
        return;
    }

    if args.iter().any(|arg| arg == "--hidden") {
        info!("User agent is already running; ignoring duplicate hidden startup");
        return;
    }

    if crate::start_guide::requests_start_guide(&args) {
        info!("--start-guide: forwarding to the running panel");
        crate::start_guide::queue_start_guide();
        crate::mem_adaptive::prewarm_main(app, "start-guide message");
        windows::activate_main_window(app, None);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("start-guide-pending", ());
        }
        return;
    }

    let windows: Vec<_> = app.webview_windows().keys().cloned().collect();
    info!("📋 当前存在的窗口: {:?}", windows);

    // 主机的「大屏」应用在串流结束时用它关掉大屏（`--desktop` 的反向）
    if args.iter().any(|arg| arg == "--close-desktop") {
        let state = if windows::close_desktop_window(app) {
            "已关闭"
        } else {
            "本来就没打开"
        };
        info!("🖥️ 检测到 --close-desktop 参数，桌面 UI {}", state);
        return;
    }

    // 检查是否要打开桌面 UI
    if args.iter().any(|arg| arg == "--desktop" || arg == "-d") {
        info!("🖥️ 检测到 --desktop 参数，打开桌面 UI");
        if let Err(e) = windows::open_desktop_window(app) {
            error!("❌ 打开桌面 UI 失败: {}", e);
        }
        return;
    }

    // 检查是否要打开工具栏
    if args.iter().any(|arg| arg == "--toolbar" || arg == "-t") {
        info!("🔧 检测到 --toolbar 参数，打开工具栏");
        toggle_toolbar_window(app);
        return;
    }

    // 提取 URL 参数并激活主窗口
    let target_url = args
        .iter()
        .find(|arg| arg.starts_with("--url="))
        .map(|arg| arg.trim_start_matches("--url=").to_string());

    if let Some(url) = &target_url {
        info!("📍 检测到 URL 参数: {}", url);

        // 检测 URL 中是否包含 /pin 路径
        match sunshine::set_runtime_sunshine_url(url) {
            Ok(base_url) => {
                proxy_server::set_sunshine_target(base_url);
                proxy_server::reset_fast_fail();
            }
            Err(e) => warn!("Invalid Sunshine URL from second instance: {}", e),
        }

        if url.contains("/pin") {
            info!("🔐 检测到 /pin 路径，打开 PIN 配对窗口");
            if let Err(e) = windows::open_pin_window(app) {
                error!("❌ 打开 PIN 窗口失败: {}", e);
            }
            return;
        }
    }

    // A second-instance message is the earliest signal for this activation
    // (panel-memory-tauri-instance, Usage).
    crate::mem_adaptive::prewarm_main(app, "second-instance message");
    windows::activate_main_window(app, target_url);
}
