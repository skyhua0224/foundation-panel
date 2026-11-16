// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bat_runner;
mod build_identity;
mod component_inventory;
mod client_fingerprint_rules;
mod clipboard;
mod commands;
mod crash;
mod controller_hub;
mod controller_input;
mod controllermeta;
mod ctx_popup;
mod desktop_settings;
mod dev_tools;
mod device_config;
mod doctor_local;
mod dualsense;
#[cfg(target_os = "windows")]
mod elevation;
mod file_mapping;
mod file_transfer;
mod fs_utils;
mod gui_agent_exit;
mod game_session;
mod github_download;
mod host_facts;
mod host_facts_math;
mod host_version;
mod hwinfo;
mod logger;
mod menu_place;
mod moonlight_web;
mod native_tools;
mod power;
mod device_probe;
mod display_mode;
mod dpi_scale;
mod dpi_scale_core;
mod probe_cache;
mod proxy_server;
mod rtss;
mod rtss_detect;
mod rtss_frame_rates;
mod perf;
mod hdr_enhanced;
mod native_components;
mod launch_splash;
mod nr_overlay;
mod panel_page;
#[cfg(target_os = "windows")]
mod shell_context_menu;
mod stall_watch;
mod start_guide;
mod sunshine;
mod version_compare;
mod sunshine_service;
mod system;
mod system_load;
mod system_load_core;
mod text_encoding;
#[cfg(target_os = "windows")]
mod text_context;
mod toolbar;
mod tray;
mod tray_config;
mod tray_prefs;
mod update;
mod usbip;
mod utils;
mod vdd;
mod vdd_calibration;
#[cfg(target_os = "windows")]
mod vdd_ioctl;
mod vigem;
mod vmouse;
#[cfg(target_os = "windows")]
mod win_clipboard;
mod windows;
mod windows_proxy;
mod webview_runtime;

use log::info;
use std::sync::OnceLock;

static ASYNC_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn configure_async_runtime() {
    let worker_threads = std::thread::available_parallelism()
        .map(|count| count.get().min(4))
        .unwrap_or(2);
    let runtime = ASYNC_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .thread_name("sunshine-gui-worker")
            // 2 MiB is tokio's default: the TLS and HTTP calls of the agent run deep, and two stack overflows of the first release came from worker threads
            .thread_stack_size(8 * 1024 * 1024)
            .enable_all()
            .build()
            .expect("failed to create the Sunshine GUI async runtime")
    });
    tauri::async_runtime::set(runtime.handle().clone());
}

#[cfg(target_os = "windows")]
fn configure_loopback_proxy_bypass() {
    // This runs before any helper path or async worker can create an HTTP
    // client, so every GUI-owned loopback request gets the same direct route.
    unsafe {
        // A caller-provided value (for example --remote-debugging-port to trace
        // a separately launched instance) stays in front of ours.
        let mut browser_arguments = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .into_iter()
            .collect::<Vec<_>>();
        browser_arguments.extend([
            // WebView2 uses Chromium's bypass syntax. Keep external requests on the
            // user's configured proxy while forcing every loopback form to direct.
            "--proxy-bypass-list=localhost;*.localhost;127.0.0.0/8;[::1]".to_owned(),
            // UI 音效：手柄输入不属于「用户手势」，不解锁 AudioContext；
            // 应用自身的导航反馈音不是网页自动播放媒体
            "--autoplay-policy=no-user-gesture-required".to_owned(),
        ]);
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            browser_arguments.join(" "),
        );

        // reqwest follows NO_PROXY/no_proxy. Preserve the user's existing
        // exclusions and add the same loopback scope used by WebView2.
        for key in ["NO_PROXY", "no_proxy"] {
            let mut value = std::env::var(key).unwrap_or_default();
            for entry in ["localhost", ".localhost", "127.0.0.0/8", "::1"] {
                if !value.split(',').any(|current| current.trim() == entry) {
                    if !value.is_empty() {
                        value.push(',');
                    }
                    value.push_str(entry);
                }
            }
            std::env::set_var(key, value);
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    configure_loopback_proxy_bypass();

    #[cfg(target_os = "windows")]
    if let Some(exit_code) =
        native_components::providers::nvidia_dlssnr::try_handle_elevated_command()
    {
        std::process::exit(exit_code);
    }
    if let Some(exit_code) =
        native_components::providers::nvidia_rtx_hdr::try_handle_elevated_command()
    {
        std::process::exit(exit_code);
    }

    // Elevated SunshineService helper: one fixed verb, allowlisted operations.
    #[cfg(target_os = "windows")]
    if let Some(exit_code) = sunshine_service::try_handle_elevated_command() {
        std::process::exit(exit_code);
    }

    #[cfg(target_os = "windows")]
    utils::wait_for_elevated_restart_handoff();

    #[cfg(target_os = "windows")]
    if let Some(exit_code) = usbip::try_handle_elevated_command() {
        std::process::exit(exit_code);
    }

    #[cfg(target_os = "windows")]
    if let Some(exit_code) = vdd::try_handle_elevated_ioctl_command() {
        std::process::exit(exit_code);
    }

    if desktop_settings::try_remove_auto_start_from_args() {
        return;
    }
    if update::try_run_updater_helper_from_args() {
        return;
    }
    if sunshine::try_run_core_compatibility_check_from_args() {
        return;
    }

    // Panics, native crashes and WebView2 failures leave a report (crash/mod.rs); the upload waits for the start to be over.
    crash::install(env!("CARGO_PKG_VERSION"), sunshine::config_dir());

    // The agent mostly waits on local I/O. Bounding the shared runtime avoids
    // allocating one worker per logical CPU on high-core-count hosts. Proxy
    // environment variables must be finalized before worker threads start.
    configure_async_runtime();

    let builder = tauri::Builder::default();

    let builder = if std::env::var_os("SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE").is_some() {
        builder
    } else {
        builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            app::handle_single_instance(app, args);
        }))
    };

    let application = builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 初始化日志系统（需要在 setup 中获取 app handle）
            logger::init_logger();
            client_fingerprint_rules::start();
            stall_watch::start(app.handle().clone());
            info!("🚀 Sunshine Control Panel 启动中...");

            app::setup_application(app)
        })
        .on_window_event(|window, event| {
            windows::handle_window_event(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::toggle_dark_mode,
            toolbar::handle_toolbar_menu_action,
            toolbar::save_toolbar_position,
            toolbar::is_primary_mouse_button_pressed,
            toolbar::set_toolbar_window_expanded,
            display_mode::list_display_modes,
            display_mode::set_display_mode,
            system::get_current_dpi,
            system::get_dpi_scale_info,
            system::set_desktop_dpi,
            native_tools::open_native_tool,
            commands::open_tool_window,
            commands::open_panel_page,
            commands::open_desktop,
            game_session::launch_game,
            game_session::get_running_game,
            game_session::stop_running_game,
            game_session::get_game_stats,
            game_session::focus_running_game,
            toolbar::create_toolbar_window,
            commands::fetch_speech_phrases,
            commands::fetch_remote_bytes,
            commands::ai_api_proxy,
            commands::capture_screenshot,
            desktop_settings::get_desktop_settings,
            desktop_settings::save_desktop_settings,
            desktop_settings::load_toolbar_shortcut_status,
            desktop_settings::set_toolbar_shortcut_enabled,
            vdd::get_vdd_status,
            vdd::install_vdd_driver,
            doctor_local::doctor_local,
            vdd::set_vdd_keep_enabled,
            vdd::set_vdd_headless_create_enabled,
            vdd::get_vdd_tools_dir_path,
            vdd::get_vdd_edid_file_path,
            vdd::load_vdd_settings,
            vdd::save_vdd_settings,
            vdd::exec_vdd_cmd,
            vdd::upload_edid_file,
            vdd::read_edid_file,
            vdd::delete_edid_file,
            vdd::get_vdd_trace_status,
            vdd::start_vdd_trace,
            vdd::stop_vdd_trace,
            vdd::open_vdd_trace_folder,
            vdd_calibration::launch_windows_hdr_calibration,
            system::get_gpus,
            system::get_monitors,
            system::get_system_info,
            system::get_process_memory_info,
            system_load::system_load,
            system::get_sunshine_start_time,
            host_facts::get_sunshine_host_facts,
            sunshine::get_sunshine_install_path,
            sunshine::get_sunshine_version,
            sunshine::get_sunshine_build_info,
            sunshine::parse_sunshine_config,
            sunshine::get_sunshine_url,
            sunshine::get_command_line_url,
            sunshine::get_sunshine_locale,
            sunshine::set_sunshine_locale,
            sunshine::get_active_sessions,
            sunshine::stop_all_sessions,
            sunshine::change_bitrate,
            sunshine::toggle_sunshine_mode,
            sunshine::is_sunshine_running_in_user_mode,
            sunshine::restart_sunshine_in_user_mode,
            sunshine::restart_sunshine_service,
            sunshine_service::set_sunshine_service_start_type,
            sunshine_service::get_sunshine_service_status,
            sunshine_service::control_sunshine_service,
            proxy_server::get_proxy_url_command,
            proxy_server::get_proxy_health_check,
            proxy_server::probe_sunshine_host,
            proxy_server::refresh_sunshine_target,
            proxy_server::wait_for_proxy_ready,
            native_components::native_component_get_status,
            native_components::hdr_enhanced_select_backend,
            native_components::list_native_components,
            native_components::native_component_import,
            native_components::native_component_remove,
            native_components::native_component_recover,
            utils::open_external_url,
            utils::open_device_manager,
            utils::open_local_path,
            utils::open_sparxie,
            utils::restart_graphics_driver,
            utils::restart_as_admin,
            utils::is_running_as_admin,
            vdd::uninstall_vdd_driver,
            vmouse::get_vmouse_status,
            vmouse::install_vmouse_driver,
            vmouse::uninstall_vmouse_driver,
            vmouse::set_vmouse_config,
            controller_hub::get_controller_hub_config,
            controller_hub::save_controller_hub_config,
            controller_hub::get_virtual_microphone_status,
            controller_hub::get_audio_devices,
            controller_hub::start_vb_cable_job,
            controller_hub::test_virtual_microphone,
            controller_hub::save_microphone_backend,
            vigem::get_vigem_status,
            component_inventory::get_component_inventory,
            vigem::install_vigem_driver,
            vigem::uninstall_vigem_driver,
            dualsense::dualsense_get_status,
            dualsense::dualsense_get_tuning,
            dualsense::dualsense_set_tuning,
            usbip::usbip_get_status,
            usbip::usbip_get_forwarding_config,
            usbip::usbip_save_forwarding_config,
            usbip::usbip_install_transport,
            usbip::usbip_cleanup_transport,
            usbip::usbip_list_remote,
            usbip::usbip_attach,
            usbip::usbip_detach,
            fs_utils::get_icc_file_list,
            fs_utils::read_directory,
            fs_utils::read_image_as_data_url,
            fs_utils::copy_image_to_assets,
            fs_utils::cleanup_unused_covers,
            fs_utils::resolve_lnk_target,
            fs_utils::scan_directory_for_apps,
            fs_utils::scan_game_libraries,
            fs_utils::search_steam_covers,
            fs_utils::upload_steam_cover,
            fs_utils::save_text_file,
            fs_utils::save_binary_file,
            update::check_for_updates,
            update::check_for_updates_for_channel,
            update::start_update_checker_when_ui_ready,
            update::get_include_prerelease_preference,
            update::set_include_prerelease_preference,
            update::update_cdk_present,
            update::update_cdk_set,
            update::update_cdk_clear,
            update::update_mirrorchyan_check,
            update::set_update_source,
            update::get_update_source_used,
            update::download_update,
            update::download_update_status,
            update::check_cached_update,
            update::install_update,
            logger::get_all_logs,
            logger::clear_logs,
            logger::export_logs,
            moonlight_web::moonlight_web_get_status,
            moonlight_web::moonlight_web_start,
            moonlight_web::moonlight_web_stop,
            moonlight_web::moonlight_web_get_config,
            moonlight_web::moonlight_web_save_config,
            moonlight_web::moonlight_web_check_release,
            moonlight_web::moonlight_web_download,
            moonlight_web::moonlight_web_get_install_path,
            moonlight_web::moonlight_web_generate_cert,
            controllermeta::controllermeta_get_status,
            controllermeta::controllermeta_check_release,
            controllermeta::controllermeta_download,
            controllermeta::controllermeta_launch,
            controllermeta::controllermeta_get_install_path,
            controllermeta::controllermeta_uninstall,
            clipboard::clipboard_sync_status,
            file_mapping::quick_share_folder,
            file_mapping::list_file_mappings,
            file_mapping::delete_file_mapping,
            file_mapping::update_file_mapping,
            file_mapping::install_file_mapping_menu,
            file_mapping::uninstall_file_mapping_menu,
            file_transfer::send_file_to_client,
            windows::webview_heartbeat,
            logger::frontend_slow,
            windows::panel_content_ready,
            ctx_popup::ctx_popup_loaded,
            ctx_popup::ctx_popup_open,
            ctx_popup::ctx_popup_ready,
            ctx_popup::ctx_popup_pick,
            ctx_popup::ctx_popup_cancel,
            ctx_popup::ctx_popup_selftest,
            windows::panel_set_surface,
            windows::panel_dpi_settled,
            windows::panel_activity,
            windows::panel_first_paint,
            windows::test_synthetic_dpi,
            dev_tools::dev_get_state,
            dev_tools::dev_set_cover_mode,
            dev_tools::dev_set_cold_cloak,
            dev_tools::dev_simulate_dpi,
            dev_tools::dev_open_devtools,
            dev_tools::dev_capture_window,
            dev_tools::dev_clear_webview_cache,
            dev_tools::dev_reset_desktop_settings,
            dev_tools::dev_environment,
            windows::resize_tool_window,
            hdr_enhanced::nr_live_status,
            hdr_enhanced::nr_live_set_enabled,
            hdr_enhanced::nr_live_remember,
            launch_splash::launch_splash_frame,
            launch_splash::launch_splash_prepared,
            launch_splash::launch_splash_painted,
            nr_overlay::nr_overlay_capture_shortcut,
            nr_overlay::nr_overlay_settings,
            nr_overlay::nr_overlay_save_settings,
            nr_overlay::nr_overlay_select_session,
            nr_overlay::nr_overlay_set_visible,
            nr_overlay::nr_overlay_is_visible,
            rtss::get_rtss_status,
            rtss::rtss_set_osd,
            rtss::rtss_clear_osd,
            rtss::rtss_set_framerate_limit,
            rtss::rtss_get_framerate_limit,
            rtss::rtss_toggle_limiter,
            rtss::rtss_get_limiter_status,
            rtss::rtss_toggle_overlay,
            rtss::rtss_download_cli,
            rtss::rtss_get_available_metrics,
            rtss::rtss_start_monitoring,
            rtss::rtss_stop_monitoring,
            rtss::rtss_get_monitoring_status,
            rtss::rtss_get_osd_properties,
            rtss::rtss_set_osd_property,
            hwinfo::hwinfo_get_sensors,
            hwinfo::hwinfo_get_readings,
            hwinfo::hwinfo_check_available,
            tray::set_locale_preferences,
            tray::tray_prefs_get,
            tray::tray_prefs_set,
            tray::tray_popup_act,
            tray::tray_popup_set,
            tray::tray_layout_set,
            tray::tray_popup_layout,
            ctx_popup::tray_popup_theme_set,
            tray::get_tray_locale,
            tray::main_panel_loading,
            tray::main_panel_ready,
            panel_page::take_pending_panel_page,
            start_guide::take_pending_start_guide,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    application.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        // the application goes down: the menu window that is destroyed with the rest is not made again
        tauri::RunEvent::ExitRequested { code: Some(_), .. } => {
            ctx_popup::note_exiting();
            windows::hide_all_windows(app_handle);
        }
        tauri::RunEvent::Exit => {
            ctx_popup::note_exiting();
            windows::hide_all_windows(app_handle);
            native_tools::shutdown_all();
            app::shutdown_application(app_handle);
        }
        _ => {}
    });
}
