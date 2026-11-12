use crate::device_config::{DEVICE_CONFIG_LOCK, read_device_api_response};
use crate::sunshine;
use log::{debug, info};
use serde::{Deserialize, Serialize};

/// 控制器中心相关的 sunshine.conf 配置（仿真模式 / DS4 行为 / DSU 体感服务器）
#[derive(Debug, Serialize, Clone)]
pub struct ControllerHubConfig {
    pub gamepad: String,
    pub motion_as_ds4: bool,
    pub touchpad_as_ds4: bool,
    pub ds4_back_as_touchpad_click: bool,
    pub enable_dsu_server: bool,
    pub dsu_server_port: u16,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct MicrophoneStatus {
    pub success: bool,
    pub configured_backend: String,
    pub active_backend: String,
    pub fallback_reason: String,
    pub component_available: bool,
    pub online: bool,
    pub device_created: bool,
    pub host_streaming: bool,
    pub generation: u32,
    pub state: String,
    pub buffered_bytes: u32,
    pub underruns: u32,
    pub dropped_frames: u32,
    pub submit_errors: u32,
    pub last_error: i32,
    pub error_code: String,
    /// Peak of the client microphone input since the previous status read (0..1).
    pub input_peak: f64,
    /// Milliseconds since the last client microphone frame; -1 when none arrived.
    pub input_active_ms: i64,
    /// Total client microphone frames received since the host process started.
    pub input_frames: u64,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct MicrophoneTestResult {
    pub success: bool,
    pub error_code: String,
    pub backend: String,
    /// "tone" after a test sound was played; "live" when a stream owns the route.
    pub mode: String,
    pub input_peak: f64,
    pub input_active_ms: i64,
    /// Tone diagnostics: seconds heard, judged windows and the tone amplitude found.
    pub heard_seconds: f64,
    pub windows_total: i32,
    pub windows_good: i32,
    pub heard_amplitude: f64,
}

async fn device_api_url(path: &str) -> Result<String, String> {
    let base_url = sunshine::get_sunshine_url().await?;
    Ok(format!("{}{}", base_url.trim_end_matches('/'), path))
}

#[tauri::command]
pub async fn get_virtual_microphone_status() -> Result<MicrophoneStatus, String> {
    let endpoint = device_api_url("/api/microphone/status").await?;
    let response = sunshine::create_https_client()?
        .get(endpoint)
        .send()
        .await
        .map_err(|error| format!("Unable to query virtual microphone status: {error}"))?;
    read_device_api_response(response, "virtual microphone status").await
}

/// The audio endpoints of the host (GET /api/audio/devices): render and capture lists with the known virtual devices
/// flagged, and the VB-CABLE state. Passed through as the host sends it; the pages read the fields they need.
#[tauri::command]
pub async fn get_audio_devices() -> Result<serde_json::Value, String> {
    let endpoint = device_api_url("/api/audio/devices").await?;
    let response = sunshine::create_https_client()?
        .get(endpoint)
        .send()
        .await
        .map_err(|error| format!("Unable to query audio devices: {error}"))?;
    read_device_api_response(response, "audio devices").await
}

/// Ask the host to install VB-CABLE ("install": download from VB-Audio, check the signature, silent setup) or to enable its
/// disabled device ("enable": pnputil). The page has asked the person first; the host starts nothing without `confirm`.
/// The progress is `vb_cable.job` of GET /api/audio/devices.
#[tauri::command]
pub async fn start_vb_cable_job(action: String) -> Result<serde_json::Value, String> {
    if action != "install" && action != "enable" {
        return Err(format!("Invalid VB-CABLE action: {action}"));
    }
    let endpoint = device_api_url(&format!("/api/audio/vb-cable/{action}")).await?;
    let response = sunshine::create_https_client()?
        .post(endpoint)
        .json(&serde_json::json!({ "confirm": true }))
        .send()
        .await
        .map_err(|error| format!("Unable to start the VB-CABLE job: {error}"))?;
    read_device_api_response(response, "VB-CABLE job").await
}

#[tauri::command]
pub async fn test_virtual_microphone() -> Result<MicrophoneTestResult, String> {
    let endpoint = device_api_url("/api/microphone/test").await?;
    let response = sunshine::create_https_client()?
        .post(endpoint)
        .send()
        .await
        .map_err(|error| format!("Unable to test the virtual microphone: {error}"))?;
    read_device_api_response(response, "virtual microphone test").await
}

const MICROPHONE_BACKENDS: [&str; 4] = ["auto", "vb_cable", "usbip_experimental", "disabled"];

/// 保存麦克风重定向后端（写入 sunshine.conf 的 microphone_redirect_backend）。
#[tauri::command]
pub async fn save_microphone_backend(backend: String) -> Result<(), String> {
    if !MICROPHONE_BACKENDS.contains(&backend.as_str()) {
        return Err(format!("Invalid microphone backend: {backend}"));
    }
    let _config_guard = DEVICE_CONFIG_LOCK.lock().await;
    let mut config = crate::vdd::read_full_sunshine_config().await?;
    config.insert(
        "microphone_redirect_backend".to_string(),
        serde_json::json!(backend),
    );
    sunshine::post_sunshine_config(&config).await?;
    info!("麦克风重定向后端已保存: {backend}");
    Ok(())
}

const GAMEPAD_MODES: [&str; 4] = ["auto", "x360", "ds4", "ds5"];

/// 对齐 Sunshine 核心 to_bool 的接受列表（大小写不敏感）
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_lowercase().as_str() {
        "true" | "yes" | "enable" | "enabled" | "on" | "1" => Some(true),
        "false" | "no" | "disable" | "disabled" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// 读取控制器中心配置，缺失键回退到核心默认值
#[tauri::command]
pub async fn get_controller_hub_config() -> Result<ControllerHubConfig, String> {
    let endpoint = device_api_url("/api/gamepad/config").await?;
    let response = sunshine::create_https_client()?
        .get(endpoint)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Unable to query controller configuration: {}",
                error.without_url()
            )
        })?;
    let response: serde_json::Value =
        read_device_api_response(response, "controller configuration").await?;
    if response.get("status").and_then(serde_json::Value::as_str) != Some("true") {
        return Err("Sunshine could not read the controller configuration".to_string());
    }

    let get_str = |key: &str| -> Option<String> {
        response.get(key).and_then(|v| v.as_str()).map(String::from)
    };

    let gamepad = get_str("gamepad")
        .filter(|v| GAMEPAD_MODES.contains(&v.as_str()))
        .unwrap_or_else(|| "auto".to_string());

    let get_bool = |key: &str, default: bool| -> bool {
        get_str(key).and_then(|v| parse_bool(&v)).unwrap_or(default)
    };

    let port = get_str("dsu_server_port")
        .and_then(|v| v.trim().parse::<u16>().ok())
        .filter(|p| (1024..=65535).contains(p))
        .unwrap_or(26760);

    Ok(ControllerHubConfig {
        gamepad,
        motion_as_ds4: get_bool("motion_as_ds4", true),
        touchpad_as_ds4: get_bool("touchpad_as_ds4", true),
        ds4_back_as_touchpad_click: get_bool("ds4_back_as_touchpad_click", true),
        enable_dsu_server: get_bool("enable_dsu_server", false),
        dsu_server_port: port,
    })
}

/// 保存控制器中心配置；仅向 Core 提交发生变化的字段。
#[tauri::command]
pub async fn save_controller_hub_config(
    gamepad: Option<String>,
    motion_as_ds4: Option<bool>,
    touchpad_as_ds4: Option<bool>,
    ds4_back_as_touchpad_click: Option<bool>,
    enable_dsu_server: Option<bool>,
    dsu_server_port: Option<u16>,
) -> Result<String, String> {
    if let Some(mode) = gamepad.as_deref()
        && !GAMEPAD_MODES.contains(&mode)
    {
        return Err(format!("无效的仿真模式: {mode}（可选: auto/x360/ds4/ds5）"));
    }
    if let Some(port) = dsu_server_port
        && !(1024..=65535).contains(&port)
    {
        return Err(format!("DSU 端口必须在 1024-65535 之间: {port}"));
    }

    // Serialize this Panel process's writes. Core merges the patch under its
    // own config-file lock, so unrelated settings are never written back.
    let _config_guard = DEVICE_CONFIG_LOCK.lock().await;
    let mut patch = serde_json::Map::new();
    let mut changed: Vec<&str> = Vec::new();
    if let Some(mode) = gamepad.as_deref() {
        patch.insert("gamepad".to_string(), serde_json::json!(mode));
        changed.push("gamepad");
    }
    for (key, value) in [
        ("motion_as_ds4", motion_as_ds4),
        ("touchpad_as_ds4", touchpad_as_ds4),
        ("ds4_back_as_touchpad_click", ds4_back_as_touchpad_click),
        ("enable_dsu_server", enable_dsu_server),
    ] {
        if let Some(v) = value {
            patch.insert(
                key.to_string(),
                serde_json::json!(if v { "true" } else { "false" }),
            );
            changed.push(key);
        }
    }
    if let Some(port) = dsu_server_port {
        patch.insert(
            "dsu_server_port".to_string(),
            serde_json::json!(port.to_string()),
        );
        changed.push("dsu_server_port");
    }

    if changed.is_empty() {
        return Ok("没有需要保存的更改".to_string());
    }

    debug!("📝 控制器中心更新配置项: {:?}", changed);
    let endpoint = device_api_url("/api/gamepad/config").await?;
    let response = sunshine::create_https_client()?
        .post(endpoint)
        .json(&patch)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Unable to save controller configuration: {}",
                error.without_url()
            )
        })?;
    let response: serde_json::Value =
        read_device_api_response(response, "controller configuration save").await?;
    if response.get("status").and_then(serde_json::Value::as_str) != Some("true") {
        return Err("Sunshine could not save the controller configuration".to_string());
    }
    info!("✅ 控制器中心配置已保存: {:?}", changed);
    Ok("saved".to_string())
}

#[cfg(test)]
mod tests {
    use super::MICROPHONE_BACKENDS;

    #[test]
    fn microphone_backends_are_the_four_host_values() {
        assert_eq!(
            MICROPHONE_BACKENDS,
            ["auto", "vb_cable", "usbip_experimental", "disabled"]
        );
    }
}
