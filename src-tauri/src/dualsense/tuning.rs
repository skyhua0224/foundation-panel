//! Client for the host's DualSense tuning profile (`/api/dualsense/tuning`).
//!
//! The host publishes one descriptor per setting (range, default, group, tier,
//! whether the running build implements it) together with the current value.
//! The Panel renders that table and sends back only the values the person
//! changed, so it keeps no second key list.
//!
//! Failures are reported with a stable code so the interface can show a plain
//! sentence: `DS5-HOST-001` host unreachable, `-002` host without tuning API
//! (an older build), `-003` not authorized, `-004` other schema, `-005`
//! unreadable answer, `DS5-CFG-003` save not possible.

use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub(crate) const SCHEMA: &str = "ALK-DS5T/1";
const TUNING_PATH: &str = "/api/dualsense/tuning";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_KEYS: usize = 512;
const MAX_VALUES_PER_SAVE: usize = 256;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TuningKey {
    pub id: u32,
    pub key: String,
    /// Developer-style English label; the interface carries its own wording.
    pub label: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub group: u32,
    pub control: String,
    pub tier: String,
    pub scale: String,
    pub expert: bool,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: Value,
    pub value: Value,
    pub stored: bool,
    pub implemented: bool,
    pub active: bool,
    pub unit: Option<String>,
    pub enum_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TuningBackend {
    pub native_path_compiled: bool,
    pub native_path_active: bool,
    pub active_controllers: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct TuningSnapshot {
    pub schema: String,
    pub revision: u64,
    pub keys: Vec<TuningKey>,
    pub implemented_count: u32,
    pub native_path_compiled: bool,
    pub backend: TuningBackend,
    pub platform: String,
    pub load_error: String,
    pub load_warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct SavedKey {
    pub id: u32,
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct TuningSaveResult {
    /// False when the host refused the whole request.
    pub accepted: bool,
    pub code: u32,
    /// The host's English diagnostic; the interface shows its own sentence.
    pub detail: String,
    pub applied: Vec<SavedKey>,
    pub unavailable: Vec<SavedKey>,
    pub snapshot: Option<TuningSnapshot>,
}

fn coded(code: &str, message: &str) -> String {
    format!("{code}: {message}")
}

/// Rejects answers that are not a sane ALK-DS5T/1 table.
fn validate_snapshot(snapshot: TuningSnapshot) -> Result<TuningSnapshot, String> {
    if snapshot.schema != SCHEMA {
        return Err(coded(
            "DS5-HOST-004",
            &format!("unsupported tuning schema '{}'", snapshot.schema),
        ));
    }
    if snapshot.keys.is_empty() || snapshot.keys.len() > MAX_KEYS {
        return Err(coded("DS5-HOST-005", "the tuning table is empty or too large"));
    }
    let mut ids = std::collections::HashSet::new();
    for key in &snapshot.keys {
        if key.id == 0 || key.key.is_empty() || !ids.insert(key.id) {
            return Err(coded("DS5-HOST-005", "the tuning table has an invalid entry"));
        }
    }
    Ok(snapshot)
}

/// Parses a response body that must be a tuning table.
pub(crate) fn parse_snapshot(bytes: &[u8]) -> Result<TuningSnapshot, String> {
    let snapshot: TuningSnapshot = serde_json::from_slice(bytes)
        .map_err(|_| coded("DS5-HOST-005", "the host sent an unexpected response"))?;
    validate_snapshot(snapshot)
}

/// Maps a non-success status to the failure the interface explains.
fn status_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => coded("DS5-HOST-003", "the host did not authorize the request"),
        // An older host answers unknown routes with 404 (or 405 for a verb).
        404 | 405 | 501 => coded("DS5-HOST-002", "the host has no DualSense tuning"),
        code => coded("DS5-HOST-005", &format!("the host responded with HTTP status {code}")),
    }
}

async fn read_body(mut response: reqwest::Response) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(coded("DS5-HOST-005", "the tuning answer is too large"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| coded("DS5-HOST-001", "the host stopped answering"))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(coded("DS5-HOST-005", "the tuning answer is too large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Reads the tuning table from `base_url`.
pub(crate) async fn fetch_snapshot(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<TuningSnapshot, String> {
    let response = client
        .get(format!("{}{TUNING_PATH}", base_url.trim_end_matches('/')))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            warn!("DualSense tuning read failed: {}", error.without_url());
            coded("DS5-HOST-001", "the host is not reachable")
        })?;
    let status = response.status();
    let bytes = read_body(response).await?;
    if !status.is_success() {
        return Err(status_error(status));
    }
    parse_snapshot(&bytes)
}

/// Builds the request body; refuses empty or oversized saves.
pub(crate) fn save_body(values: Map<String, Value>) -> Result<Value, String> {
    if values.is_empty() || values.len() > MAX_VALUES_PER_SAVE {
        return Err(coded("DS5-CFG-003", "there is nothing to save"));
    }
    if values.values().any(Value::is_null) {
        return Err(coded("DS5-CFG-003", "a value is missing"));
    }
    Ok(serde_json::json!({ "schema": SCHEMA, "values": values }))
}

fn saved_keys(body: &Value, field: &str) -> Vec<SavedKey> {
    body.get(field)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// Interprets the host's answer to a save: 200, or a refusal that still carries the table.
pub(crate) fn parse_save_response(status: u16, bytes: &[u8]) -> Result<TuningSaveResult, String> {
    let body: Value = serde_json::from_slice(bytes).map_err(|_| match status {
        401 | 403 => coded("DS5-HOST-003", "the host did not authorize the request"),
        404 | 405 | 501 => coded("DS5-HOST-002", "the host has no DualSense tuning"),
        _ => coded("DS5-HOST-005", "the host sent an unexpected response"),
    })?;
    if matches!(status, 401 | 403 | 404 | 405 | 501) {
        return Err(status_error(
            reqwest::StatusCode::from_u16(status).unwrap_or(reqwest::StatusCode::BAD_REQUEST),
        ));
    }
    let snapshot = body
        .get("keys")
        .and_then(|_| serde_json::from_value::<TuningSnapshot>(body.clone()).ok())
        .and_then(|snapshot| validate_snapshot(snapshot).ok());
    let accepted = (200..300).contains(&status) && body.get("status") == Some(&Value::Bool(true));
    Ok(TuningSaveResult {
        accepted,
        code: body.get("code").and_then(Value::as_u64).unwrap_or(0) as u32,
        detail: body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect(),
        applied: saved_keys(&body, "applied"),
        unavailable: saved_keys(&body, "unavailable"),
        snapshot,
    })
}

/// Sends changed values to the host.
pub(crate) async fn save_values(
    client: &reqwest::Client,
    base_url: &str,
    values: Map<String, Value>,
) -> Result<TuningSaveResult, String> {
    let body = save_body(values)?;
    let response = client
        .post(format!("{}{TUNING_PATH}", base_url.trim_end_matches('/')))
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            warn!("DualSense tuning save failed: {}", error.without_url());
            coded("DS5-HOST-001", "the host is not reachable")
        })?;
    let status = response.status().as_u16();
    let bytes = read_body(response).await?;
    parse_save_response(status, &bytes)
}

async fn local_base() -> Result<String, String> {
    crate::sunshine::get_sunshine_url()
        .await
        .map_err(|_| coded("DS5-HOST-001", "the host address is unknown"))
}

#[tauri::command]
pub async fn dualsense_get_tuning() -> Result<TuningSnapshot, String> {
    let client = crate::sunshine::create_https_client()?;
    fetch_snapshot(&client, &local_base().await?).await
}

#[tauri::command]
pub async fn dualsense_set_tuning(values: Map<String, Value>) -> Result<TuningSaveResult, String> {
    let client = crate::sunshine::create_https_client()?;
    save_values(&client, &local_base().await?, values).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "schema": "ALK-DS5T/1", "status": true, "revision": 3,
            "implemented_count": 2, "native_path_compiled": true,
            "backend": { "native_path_compiled": true, "native_path_active": false, "active_controllers": 0, "persistence": "config-directory" },
            "platform": "windows",
            "keys": [
                { "id": 3, "key": "ds5.haptics.hd.gain", "label": "x", "type": "f32", "group": 1, "control": "slider",
                  "tier": "common", "scale": "linear", "expert": false, "min": 0.0, "max": 2.0, "step": 0.01,
                  "default": 1.0, "value": 1.2, "stored": true, "implemented": true, "active": false,
                  "unit": null, "enum_names": [], "future_field": 1 },
                { "id": 81, "key": "ds5.led.lightbar.mode", "type": "enum", "group": 8, "control": "segmented", "tier": "common",
                  "default": "followGame", "value": "followGame", "implemented": false, "enum_names": ["followGame", "fixed"] }
            ]
        })
    }

    #[test]
    fn parses_the_host_table_and_ignores_unknown_fields() {
        let snapshot = parse_snapshot(sample().to_string().as_bytes()).unwrap();
        assert_eq!(snapshot.revision, 3);
        assert_eq!(snapshot.keys.len(), 2);
        assert!(snapshot.keys[0].implemented && !snapshot.keys[1].implemented);
        assert_eq!(snapshot.keys[0].value, json!(1.2));
        assert_eq!(snapshot.keys[1].enum_names, ["followGame", "fixed"]);
        assert!(snapshot.backend.native_path_compiled);
    }

    #[test]
    fn rejects_other_schemas_and_broken_tables() {
        let mut other = sample();
        other["schema"] = json!("ALK-DS5T/2");
        assert!(parse_snapshot(other.to_string().as_bytes()).unwrap_err().starts_with("DS5-HOST-004"));
        let mut duplicate = sample();
        duplicate["keys"][1]["id"] = json!(3);
        assert!(parse_snapshot(duplicate.to_string().as_bytes()).unwrap_err().starts_with("DS5-HOST-005"));
        let mut empty = sample();
        empty["keys"] = json!([]);
        assert!(parse_snapshot(empty.to_string().as_bytes()).unwrap_err().starts_with("DS5-HOST-005"));
        assert!(parse_snapshot(b"<html>").unwrap_err().starts_with("DS5-HOST-005"));
    }

    #[test]
    fn save_body_needs_values_and_names_the_schema() {
        assert!(save_body(Map::new()).is_err());
        let mut values = Map::new();
        values.insert("ds5.haptics.hd.gain".into(), json!(1.4));
        assert_eq!(
            save_body(values).unwrap(),
            json!({ "schema": "ALK-DS5T/1", "values": { "ds5.haptics.hd.gain": 1.4 } })
        );
        let mut missing = Map::new();
        missing.insert("ds5.haptics.hd.gain".into(), Value::Null);
        assert!(save_body(missing).is_err());
    }

    #[test]
    fn partial_save_reports_applied_and_unavailable_keys() {
        let mut body = sample();
        body["partial"] = json!(true);
        body["code"] = json!(1);
        body["applied"] = json!([{ "id": 3, "key": "ds5.haptics.hd.gain", "value": 1.4, "clamped": false }]);
        body["unavailable"] = json!([{ "id": 81, "key": "ds5.led.lightbar.mode", "code": 5, "reason": "x" }]);
        let result = parse_save_response(200, body.to_string().as_bytes()).unwrap();
        assert!(result.accepted);
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.unavailable[0].id, 81);
        assert!(result.snapshot.is_some());
    }

    #[test]
    fn refused_save_keeps_the_table_and_is_not_accepted() {
        let mut body = sample();
        body["status"] = json!(false);
        body["code"] = json!(5);
        body["error"] = json!("none of the submitted keys are implemented by the running path");
        let result = parse_save_response(409, body.to_string().as_bytes()).unwrap();
        assert!(!result.accepted);
        assert_eq!(result.code, 5);
        assert!(result.snapshot.is_some());
        let invalid = parse_save_response(
            400,
            json!({ "schema": "ALK-DS5T/1", "status": false, "code": 3, "error": "range" })
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        assert!(!invalid.accepted && invalid.snapshot.is_none() && invalid.code == 3);
    }

    #[test]
    fn missing_route_and_authorization_are_distinct_failures() {
        for status in [404, 405, 501] {
            assert!(parse_save_response(status, b"Not found").unwrap_err().starts_with("DS5-HOST-002"));
            assert!(parse_save_response(status, b"{}").unwrap_err().starts_with("DS5-HOST-002"));
        }
        for status in [401, 403] {
            assert!(parse_save_response(status, b"{}").unwrap_err().starts_with("DS5-HOST-003"));
        }
        assert!(parse_save_response(500, b"oops").unwrap_err().starts_with("DS5-HOST-005"));
    }

    async fn serve(router: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn an_older_host_without_the_route_is_reported_as_unsupported() {
        let base = serve(axum::Router::new()).await;
        let error = fetch_snapshot(&reqwest::Client::new(), &base).await.unwrap_err();
        assert!(error.starts_with("DS5-HOST-002"), "{error}");
        let mut values = Map::new();
        values.insert("ds5.haptics.hd.gain".into(), json!(1.0));
        let error = save_values(&reqwest::Client::new(), &base, values).await.unwrap_err();
        assert!(error.starts_with("DS5-HOST-002"), "{error}");
    }

    #[tokio::test]
    async fn a_current_host_round_trips_a_read_and_a_save() {
        use axum::{Json, routing::get};
        let router = axum::Router::new().route(
            TUNING_PATH,
            get(|| async { Json(sample()) }).post(|Json(body): Json<Value>| async move {
                assert_eq!(body["schema"], "ALK-DS5T/1");
                let mut answer = sample();
                answer["applied"] = json!([{ "id": 3, "key": "ds5.haptics.hd.gain" }]);
                answer["unavailable"] = json!([]);
                answer["partial"] = json!(false);
                answer["code"] = json!(0);
                Json(answer)
            }),
        );
        let base = serve(router).await;
        let client = reqwest::Client::new();
        assert_eq!(fetch_snapshot(&client, &base).await.unwrap().revision, 3);
        let mut values = Map::new();
        values.insert("ds5.haptics.hd.gain".into(), json!(1.4));
        let result = save_values(&client, &base, values).await.unwrap();
        assert!(result.accepted && result.applied.len() == 1);
    }

    #[tokio::test]
    async fn an_unreachable_host_is_reported_as_such() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let error = fetch_snapshot(&reqwest::Client::new(), &format!("http://127.0.0.1:{port}"))
            .await
            .unwrap_err();
        assert!(error.starts_with("DS5-HOST-001"), "{error}");
    }
}
