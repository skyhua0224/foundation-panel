use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

/// 更新来源的可选值：自动（CNB 优先）、仅 CNB、仅 GitHub、Mirror酱（CDK 加速下载）。
pub(crate) const UPDATE_SOURCES: [&str; 4] = ["auto", "cnb", "github", "mirror"];

fn default_update_source() -> String {
    "auto".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct UpdatePreferences {
    last_check_time: u64,
    include_prerelease: bool,
    /// 渲染层 foundation-ui/update 的 `SOURCE_KEY` 同款取值；旧文件缺省为 auto。
    #[serde(default = "default_update_source")]
    update_source: String,
    /// 最近一次真正用到的来源（mirror / cnb / github，来自完成的下载）；空表示还没有下载过。
    #[serde(default)]
    used_update_source: String,
    /// 最近一次下载里首选来源（Mirror酱）没有用上的原因，见 `mirrorchyan::Miss::word`；空表示用上了或没选 Mirror酱。
    #[serde(default)]
    update_source_miss: String,
}

fn preferences_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let app_data_dir = crate::test_isolation::app_data_dir(app)
        .map_err(|e| format!("获取应用数据目录失败: {e}"))?;

    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    }

    Ok(app_data_dir.join("update_preferences.json"))
}

fn load<R: Runtime>(app: &AppHandle<R>) -> UpdatePreferences {
    let path = match preferences_path(app) {
        Ok(path) => path,
        Err(error) => {
            warn!("⚠️ 获取更新偏好设置路径失败: {error}，使用默认偏好");
            return UpdatePreferences::default();
        }
    };

    if !path.exists() {
        return UpdatePreferences::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<UpdatePreferences>(&content) {
            Ok(preferences) => {
                debug!(
                    "📨 已加载更新偏好: include_prerelease={}, last_check_time={}",
                    preferences.include_prerelease, preferences.last_check_time
                );
                preferences
            }
            Err(error) => {
                warn!("⚠️ 解析更新偏好设置失败: {error}，使用默认偏好");
                UpdatePreferences::default()
            }
        },
        Err(error) => {
            warn!("⚠️ 读取更新偏好设置失败: {error}，使用默认偏好");
            UpdatePreferences::default()
        }
    }
}

fn persist<R: Runtime>(app: &AppHandle<R>, preferences: &UpdatePreferences) {
    let path = match preferences_path(app) {
        Ok(path) => path,
        Err(error) => {
            warn!("⚠️ 获取更新偏好设置路径失败，无法保存: {error}");
            return;
        }
    };

    match serde_json::to_string_pretty(preferences) {
        Ok(json) => {
            if let Err(error) = fs::write(&path, json) {
                warn!("⚠️ 保存更新偏好设置失败: {error}");
            } else {
                debug!(
                    "💾 已保存更新偏好: include_prerelease={}, last_check_time={}",
                    preferences.include_prerelease, preferences.last_check_time
                );
            }
        }
        Err(error) => warn!("⚠️ 序列化更新偏好设置失败: {error}"),
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 确保更新偏好始终存在，包括 `--hidden` 的纯托盘代理启动。
fn ensure<R: Runtime>(app: &AppHandle<R>) -> bool {
    if app.try_state::<Arc<Mutex<UpdatePreferences>>>().is_some() {
        return true;
    }

    let _ = app.manage(Arc::new(Mutex::new(load(app))));
    let initialized = app.try_state::<Arc<Mutex<UpdatePreferences>>>().is_some();
    if !initialized {
        error!("❌ 更新偏好状态初始化失败");
    }
    initialized
}

pub(crate) fn init(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if ensure(app) {
        Ok(())
    } else {
        Err(std::io::Error::other("failed to initialize update preferences").into())
    }
}

pub(crate) fn last_check_time<R: Runtime>(app: &AppHandle<R>) -> u64 {
    ensure(app);
    app.try_state::<Arc<Mutex<UpdatePreferences>>>()
        .map(|preferences| preferences.lock().unwrap().last_check_time)
        .unwrap_or(0)
}

pub(crate) fn save_last_check_time<R: Runtime>(app: &AppHandle<R>) {
    ensure(app);
    if let Some(preferences) = app.try_state::<Arc<Mutex<UpdatePreferences>>>() {
        let snapshot = {
            let mut preferences = preferences.lock().unwrap();
            preferences.last_check_time = current_timestamp();
            preferences.clone()
        };
        persist(app, &snapshot);
    }
}

pub(crate) fn include_prerelease<R: Runtime>(app: &AppHandle<R>) -> bool {
    ensure(app);
    app.try_state::<Arc<Mutex<UpdatePreferences>>>()
        .map(|preferences| preferences.lock().unwrap().include_prerelease)
        .unwrap_or(false)
}

pub(crate) fn set_include_prerelease<R: Runtime>(app: &AppHandle<R>, include: bool) {
    ensure(app);
    if let Some(preferences) = app.try_state::<Arc<Mutex<UpdatePreferences>>>() {
        let snapshot = {
            let mut preferences = preferences.lock().unwrap();
            preferences.include_prerelease = include;
            preferences.clone()
        };
        info!("📝 更新偏好设置: 包含预发布版本 = {include}");
        persist(app, &snapshot);
    }
}

/// 校验并归一化来源值：只有四个已知值，未知值拒绝（不静默回退，
/// 渲染层自己已做 normalizeSource，这里防 IPC 直调写脏数据）。
pub(crate) fn normalize_update_source(value: &str) -> Result<&'static str, String> {
    UPDATE_SOURCES
        .into_iter()
        .find(|known| *known == value)
        .ok_or_else(|| format!("不支持的更新来源: {value}"))
}

pub(crate) fn update_source<R: Runtime>(app: &AppHandle<R>) -> &'static str {
    ensure(app);
    let stored = app
        .try_state::<Arc<Mutex<UpdatePreferences>>>()
        .map(|preferences| preferences.lock().unwrap().update_source.clone())
        .unwrap_or_default();
    UPDATE_SOURCES
        .into_iter()
        .find(|known| *known == stored)
        .unwrap_or("auto")
}

/// 下载候选的显示名（Mirror酱 / CNB / GitHub）对应的来源词；其他名字不记录。
pub(crate) fn used_source_word(candidate_name: &str) -> Option<&'static str> {
    match candidate_name {
        "Mirror酱" => Some("mirror"),
        "CNB" => Some("cnb"),
        "GitHub" => Some("github"),
        _ => None,
    }
}

/// 来源统计该报的词：真正用过的来源优先，没有时是用户的设置（默认 auto）。
pub(crate) fn source_to_report(used: &str, setting: &str) -> String {
    UPDATE_SOURCES
        .into_iter()
        .find(|known| *known == used)
        .or_else(|| UPDATE_SOURCES.into_iter().find(|known| *known == setting))
        .unwrap_or("auto")
        .to_string()
}

pub(crate) fn used_update_source<R: Runtime>(app: &AppHandle<R>) -> String {
    ensure(app);
    let used = app
        .try_state::<Arc<Mutex<UpdatePreferences>>>()
        .map(|preferences| preferences.lock().unwrap().used_update_source.clone())
        .unwrap_or_default();
    source_to_report(&used, update_source(app))
}

/// 记下一次完成的下载实际用到的来源；只在变化时写盘。
pub(crate) fn record_used_update_source<R: Runtime>(app: &AppHandle<R>, candidate_name: &str) {
    let Some(word) = used_source_word(candidate_name) else {
        return;
    };
    ensure(app);
    if let Some(preferences) = app.try_state::<Arc<Mutex<UpdatePreferences>>>() {
        let snapshot = {
            let mut preferences = preferences.lock().unwrap();
            if preferences.used_update_source == word {
                return;
            }
            preferences.used_update_source = word.to_string();
            preferences.clone()
        };
        persist(app, &snapshot);
    }
}

pub(crate) fn update_source_miss<R: Runtime>(app: &AppHandle<R>) -> String {
    ensure(app);
    app.try_state::<Arc<Mutex<UpdatePreferences>>>()
        .map(|preferences| preferences.lock().unwrap().update_source_miss.clone())
        .unwrap_or_default()
}

/// 记下这次下载里首选来源没有用上的原因（`None` 清除）；只在变化时写盘。
pub(crate) fn record_update_source_miss<R: Runtime>(app: &AppHandle<R>, miss: Option<&str>) {
    let word = miss.unwrap_or("");
    ensure(app);
    if let Some(preferences) = app.try_state::<Arc<Mutex<UpdatePreferences>>>() {
        let snapshot = {
            let mut preferences = preferences.lock().unwrap();
            if preferences.update_source_miss == word {
                return;
            }
            preferences.update_source_miss = word.to_string();
            preferences.clone()
        };
        persist(app, &snapshot);
    }
}

pub(crate) fn set_update_source<R: Runtime>(
    app: &AppHandle<R>,
    source: &str,
) -> Result<(), String> {
    let source = normalize_update_source(source)?;
    ensure(app);
    if let Some(preferences) = app.try_state::<Arc<Mutex<UpdatePreferences>>>() {
        let snapshot = {
            let mut preferences = preferences.lock().unwrap();
            preferences.update_source = source.to_string();
            preferences.clone()
        };
        info!("更新偏好设置: 更新来源 = {source}");
        persist(app, &snapshot);
        Ok(())
    } else {
        Err("更新偏好状态初始化失败".to_string())
    }
}

#[cfg(test)]
mod update_source_tests {
    use super::*;

    #[test]
    fn only_the_four_known_sources_are_accepted() {
        assert_eq!(normalize_update_source("auto").unwrap(), "auto");
        assert_eq!(normalize_update_source("cnb").unwrap(), "cnb");
        assert_eq!(normalize_update_source("github").unwrap(), "github");
        assert_eq!(normalize_update_source("mirror").unwrap(), "mirror");
        assert!(normalize_update_source("p2p").is_err());
        assert!(normalize_update_source("").is_err());
        assert!(normalize_update_source("AUTO").is_err());
    }

    #[test]
    fn legacy_preferences_file_defaults_to_auto_and_round_trips_mirror() {
        let legacy = serde_json::json!({"last_check_time": 5, "include_prerelease": true});
        let parsed: UpdatePreferences = serde_json::from_value(legacy).expect("legacy file loads");
        assert_eq!(parsed.update_source, "auto");

        let chosen: UpdatePreferences = serde_json::from_value(serde_json::json!({
            "last_check_time": 5,
            "include_prerelease": false,
            "update_source": "mirror"
        }))
        .expect("chosen source loads");
        assert_eq!(chosen.update_source, "mirror");
        let serialized = serde_json::to_value(&chosen).expect("serializes");
        assert_eq!(
            serialized
                .get("update_source")
                .and_then(|value| value.as_str()),
            Some("mirror")
        );
    }

    #[test]
    fn the_download_names_map_to_source_words() {
        assert_eq!(used_source_word("Mirror酱"), Some("mirror"));
        assert_eq!(used_source_word("CNB"), Some("cnb"));
        assert_eq!(used_source_word("GitHub"), Some("github"));
        assert_eq!(used_source_word(""), None);
        assert_eq!(used_source_word("something else"), None);
    }

    #[test]
    fn the_reported_source_is_the_used_one_then_the_setting_then_auto_never_empty() {
        assert_eq!(source_to_report("mirror", "github"), "mirror");
        assert_eq!(source_to_report("", "github"), "github");
        assert_eq!(source_to_report("", ""), "auto");
        assert_eq!(source_to_report("bogus", "bogus"), "auto");
    }

    #[test]
    fn an_old_preferences_file_has_no_used_source() {
        let parsed: UpdatePreferences =
            serde_json::from_str(r#"{ "last_check_time": 1, "include_prerelease": false }"#).unwrap();
        assert_eq!(parsed.used_update_source, "");
        assert_eq!(parsed.update_source_miss, "");
    }
}
