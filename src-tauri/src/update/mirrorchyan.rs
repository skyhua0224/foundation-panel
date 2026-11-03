//! Mirror酱（MirrorChyan）最新版本接口的客户端，原生侧。
//!
//! 请求与答案的形状由渲染层的纯客户端（foundation-ui/update 的 mirrorchyan.js）
//! 决定，服务端行为以 MirrorChyan 官方集成文档为准（docs/authority/source-index.tsv
//! 的 panel-mirrorchyan-readme-20261103 与 panel-mirrorchyan-errorcode-20261103）：
//! `GET https://mirrorchyan.com/api/resources/{rid}/latest`，查询参数依次为
//! current_version、cdk（有才带）、os、arch、channel、user_agent；答案为
//! `{code, msg, data}`，code 0 成功，`data.url` 仅在 CDK 有效且有新版本时出现。
//!
//! CDK 只在调用方（本模块的使用者）从操作系统安全存储读出后拼进 URL：
//! 完整 URL 与 reqwest 的错误文本都包含 CDK，因此本模块绝不把两者写进日志，
//! 错误只归类为 Connect/Timeout/Body/Other。

use serde_json::Value;
use std::time::Duration;

/// 服务资源标识：本项目的 Mirror酱 资源。
pub(crate) const RESOURCE_ID: &str = "foundation-sunshine";
/// `user_agent` 查询参数（Mirror酱 的来源统计标识），同时用作 HTTP 头。
pub(crate) const USER_AGENT_TAG: &str = "foundation-sunshine-panel";
/// 凭据在操作系统安全存储中的目标名（Windows Credential Manager 的 TargetName）。
pub(crate) const CREDENTIAL_TARGET: &str = "foundation-sunshine-panel/mirrorchyan-cdk";

const API_ORIGIN: &str = "https://mirrorchyan.com";
const API_HOST: &str = "mirrorchyan.com";
/// 检查更新是一次手动/低频操作：整个请求 8 秒超时（设计文档 docs/update-source-design.md）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// 答案是小型 JSON：超过 1 MiB 视为异常响应，按无有效答案处理。
const MAX_BODY_BYTES: usize = 1024 * 1024;
/// 下载地址只信任官方文档示例中的主机（README：`https://mirrorchyan.com/resources/download/...`）。
const DOWNLOAD_HOSTS: [&str; 2] = [API_HOST, "www.mirrorchyan.com"];

/// 渲染层只区分 stable 与 beta；其余值一律按 stable（与 mirrorchyan.js 一致）。
pub(crate) fn normalize_channel(channel: &str) -> &'static str {
    if channel == "beta" { "beta" } else { "stable" }
}

/// 请求 URL。参数顺序与编码（application/x-www-form-urlencoded，空格为 `+`）
/// 与 mirrorchyan.js 的 `mirrorchyanUrl` 完全一致；`cdk` 只在存在时出现。
/// 仅用于发起请求与测试断言，绝不能写日志。
fn latest_url(current_version: &str, cdk: Option<&str>, channel: &str) -> String {
    let mut url = reqwest::Url::parse(&format!("{API_ORIGIN}/api/resources/{RESOURCE_ID}/latest"))
        .expect("constant MirrorChyan endpoint parses");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("current_version", current_version);
        if let Some(cdk) = cdk {
            query.append_pair("cdk", cdk);
        }
        query.append_pair("os", "windows");
        query.append_pair("arch", "amd64");
        query.append_pair("channel", normalize_channel(channel));
        query.append_pair("user_agent", USER_AGENT_TAG);
    }
    url.into()
}

/// 原生侧看到的答案：`status` 为 HTTP 状态码，0 表示没有得到回答
/// （离线、超时、连接失败）；`body` 为解析后的 JSON，无法解析时为 `None`。
/// 该结构是内部形状；跨 IPC 返回前必须经过 [`sanitize_body`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) body: Option<Value>,
}

impl Answer {
    /// 无回答（网络层失败）。渲染层把它映射为 `network`。
    fn no_answer() -> Self {
        Self {
            status: 0,
            body: None,
        }
    }
}

/// 传输层失败的分类。不携带文本：reqwest 的错误信息里含完整 URL（也就含 CDK）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchError {
    Connect,
    Timeout,
    Body,
    Other,
}

/// 用注入的传输执行一次检查。`fetch` 收到完整 URL（含 CDK，调用方不得记录），
/// 返回 `(HTTP 状态码, 解析后的 JSON 主体)`；任何失败都归为无回答。
async fn check_with<F, Fut>(
    fetch: F,
    current_version: &str,
    cdk: Option<&str>,
    channel: &str,
) -> Answer
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(u16, Option<Value>), FetchError>>,
{
    let url = latest_url(current_version, cdk, channel);
    match fetch(url).await {
        Ok((status, body)) => Answer { status, body },
        Err(_) => Answer::no_answer(),
    }
}

/// 真实 HTTP 传输：仅 HTTPS、8 秒总超时、只在 mirrorchyan.com 内部重定向、
/// 不带凭据与 Cookie，主体超过 1 MiB 即放弃。
async fn fetch_via_https(url: String) -> Result<(u16, Option<Value>), FetchError> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            // 仅允许 mirrorchyan.com 同主机的重定向，至多 3 跳；跨主机一律失败。
            let same_host = attempt
                .previous()
                .last()
                .and_then(|previous| previous.host_str().map(str::to_ascii_lowercase))
                .is_some_and(|previous_host| {
                    attempt
                        .url()
                        .host_str()
                        .is_some_and(|host| host.eq_ignore_ascii_case(&previous_host))
                });
            if !same_host || attempt.previous().len() >= 3 {
                attempt.error("redirect outside the MirrorChyan origin refused")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| FetchError::Other)?;

    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT_TAG)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                FetchError::Timeout
            } else if error.is_connect() {
                FetchError::Connect
            } else {
                FetchError::Other
            }
        })?;
    let status = response.status().as_u16();

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| FetchError::Body)?;
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(FetchError::Body);
        }
        body.extend_from_slice(&chunk);
    }
    let json = serde_json::from_slice::<Value>(&body)
        .ok()
        .filter(|value| value.is_object());
    Ok((status, json))
}

/// 执行一次真实检查。CDK 由调用方从安全存储读出；本函数与其调用方都不得
/// 记录 URL 或错误原文。
pub(crate) async fn check(current_version: &str, cdk: Option<&str>, channel: &str) -> Answer {
    check_with(fetch_via_https, current_version, cdk, channel).await
}

/// 答案的业务码；无有效主体时为 `None`。
fn code_of(answer: &Answer) -> Option<i64> {
    answer.body.as_ref()?.get("code")?.as_i64()
}

/// 仅用于日志的安全摘要：HTTP 状态与业务码，绝无 URL、CDK 或 msg。
pub(crate) fn log_summary(answer: &Answer) -> String {
    format!(
        "status={}, code={}",
        answer.status,
        code_of(answer)
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".into())
    )
}

/// 下载路径的判定：业务码为 0 且 `data.url` 是 HTTPS 地址时，该地址才可用。
/// 其余情况（无 CDK、CDK 错误、无新版本、网络失败）一律回退 CNB/GitHub。
/// Mirror酱 发来的 sha256 不参与安装决策：文件仍按本项目 release metadata 的
/// SHA-256 校验（docs/update-source-design.md）。
pub(crate) fn download_url_of(answer: &Answer) -> Option<String> {
    if code_of(answer) != Some(0) {
        return None;
    }
    let url = answer.body.as_ref()?.pointer("/data/url")?.as_str()?;
    if validate_download_url(url).is_err() {
        return None;
    }
    Some(url.to_string())
}

/// 下载地址的白名单校验：HTTPS、无端口/凭据/fragment，主机仅限官方文档示例的
/// mirrorchyan.com（含 www）。`data.url` 带时效令牌，通过与否都不写日志。
pub(crate) fn validate_download_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "下载地址格式无效".to_string())?;
    let host_ok = parsed
        .host_str()
        .is_some_and(|host| DOWNLOAD_HOSTS.contains(&host.to_ascii_lowercase().as_str()));
    if parsed.scheme() != "https"
        || parsed.port().is_some_and(|port| port != 443)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || !host_ok
    {
        return Err("下载地址不是允许的 Mirror酱 地址".to_string());
    }
    Ok(())
}

/// IPC 回复用的主体：去掉 `data.url`（带时效令牌）与任何可能的 CDK 回显，
/// 其余字段原样保留，渲染层的 mirrorchyan.js 按纯客户端规则解析。
pub(crate) fn sanitize_body(answer: &Answer) -> Option<Value> {
    let mut body = answer.body.clone()?;
    if let Some(data) = body.get_mut("data").and_then(Value::as_object_mut) {
        data.remove("url");
        data.remove("cdk");
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer(status: u16, body: Value) -> Answer {
        Answer {
            status,
            body: Some(body),
        }
    }

    #[test]
    fn latest_url_matches_the_renderer_client_with_cdk() {
        let url = latest_url("v0.0.1", Some("AB-12"), "beta");
        assert_eq!(
            url,
            "https://mirrorchyan.com/api/resources/foundation-sunshine/latest\
?current_version=v0.0.1&cdk=AB-12&os=windows&arch=amd64&channel=beta&user_agent=foundation-sunshine-panel"
        );
    }

    #[test]
    fn latest_url_omits_cdk_and_normalizes_channel_without_it() {
        let url = latest_url("", None, "nightly");
        assert_eq!(
            url,
            "https://mirrorchyan.com/api/resources/foundation-sunshine/latest\
?current_version=&os=windows&arch=amd64&channel=stable&user_agent=foundation-sunshine-panel"
        );
    }

    #[test]
    fn latest_url_form_encodes_like_urlsearchparams() {
        // application/x-www-form-urlencoded：空格为 `+`，`+` 本身为 %2B，
        // 与 mirrorchyan.js 的 URLSearchParams 逐字节一致。
        let url = latest_url("v1 2+3", None, "stable");
        assert!(
            url.contains("current_version=v1+2%2B3&os=windows"),
            "unexpected URL: {url}"
        );
    }

    #[test]
    fn code_zero_with_https_url_yields_the_download_address() {
        let answer = answer(
            200,
            json!({"code": 0, "msg": "success", "data": {"version_name": "v1.1.0", "url": "https://mirrorchyan.com/resources/download/abc"}}),
        );
        assert_eq!(
            download_url_of(&answer).as_deref(),
            Some("https://mirrorchyan.com/resources/download/abc")
        );
    }

    #[test]
    fn every_documented_business_code_has_no_download_address() {
        // 官方错误码表（ErrorCode.md）与渲染层 ERROR_KINDS 的全部业务码。
        for code in [
            1001, 7001, 7002, 7003, 7004, 7005, 8001, 8002, 8003, 8004, 8009, 1,
        ] {
            let answer = answer(403, json!({"code": code, "msg": "x"}));
            assert_eq!(download_url_of(&answer), None, "code {code}");
        }
        // 负数：服务端自身错误。
        let answer = answer(500, json!({"code": -1, "msg": "x"}));
        assert_eq!(download_url_of(&answer), None);
    }

    #[test]
    fn code_zero_without_url_or_with_foreign_url_has_no_download_address() {
        let no_url = answer(
            200,
            json!({"code": 0, "msg": "success", "data": {"version_name": "v1.1.0"}}),
        );
        assert_eq!(download_url_of(&no_url), None);
        let foreign = answer(
            200,
            json!({"code": 0, "msg": "success", "data": {"url": "https://example.invalid/file.exe"}}),
        );
        assert_eq!(download_url_of(&foreign), None);
        let plain_http = answer(
            200,
            json!({"code": 0, "msg": "success", "data": {"url": "http://mirrorchyan.com/resources/download/abc"}}),
        );
        assert_eq!(download_url_of(&plain_http), None);
    }

    #[test]
    fn download_url_allows_only_the_documented_host_without_credentials() {
        assert!(validate_download_url("https://mirrorchyan.com/resources/download/x").is_ok());
        assert!(validate_download_url("https://www.mirrorchyan.com/resources/download/x").is_ok());
        assert!(validate_download_url("http://mirrorchyan.com/resources/download/x").is_err());
        assert!(validate_download_url("https://user:secret@mirrorchyan.com/d").is_err());
        assert!(validate_download_url("https://mirrorchyan.com:8443/d").is_err());
        assert!(validate_download_url("https://cdn.example.invalid/d").is_err());
    }

    #[tokio::test]
    async fn network_failure_is_no_answer() {
        let answer = check_with(
            |_url| async { Err::<(u16, Option<Value>), _>(FetchError::Timeout) },
            "v1",
            Some("cdk"),
            "stable",
        )
        .await;
        assert_eq!(answer, Answer::no_answer());
        assert_eq!(download_url_of(&answer), None);
    }

    #[tokio::test]
    async fn transport_failure_and_unparseable_body_map_like_the_renderer_expects() {
        // 连接被拒：无回答（渲染层 -> network）。
        let connect = check_with(
            |_url| async { Err::<(u16, Option<Value>), _>(FetchError::Connect) },
            "",
            None,
            "stable",
        )
        .await;
        assert_eq!(connect, Answer::no_answer());

        // 有 HTTP 状态但主体不是 JSON：状态保留、body 为空（渲染层 -> malformed）。
        let malformed = check_with(
            |_url| async { Ok::<(u16, Option<Value>), FetchError>((502, None)) },
            "",
            None,
            "stable",
        )
        .await;
        assert_eq!(
            malformed,
            Answer {
                status: 502,
                body: None
            }
        );
    }

    #[tokio::test]
    async fn check_with_passes_the_exact_url_to_the_transport() {
        let seen = check_with(
            |url| async move {
                assert_eq!(
                    url,
                    "https://mirrorchyan.com/api/resources/foundation-sunshine/latest\
?current_version=v2&cdk=K&os=windows&arch=amd64&channel=stable&user_agent=foundation-sunshine-panel"
                );
                Ok::<(u16, Option<Value>), FetchError>((
                    200,
                    Some(json!({"code": 0, "msg": "success", "data": {"version_name": "v3"}})),
                ))
            },
            "v2",
            Some("K"),
            "stable",
        )
        .await;
        assert_eq!(code_of(&seen), Some(0));
    }

    #[test]
    fn sanitize_body_removes_the_download_address_and_keeps_the_rest() {
        let answer = answer(
            200,
            json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "version_name": "v1.1.0",
                    "url": "https://mirrorchyan.com/resources/download/token",
                    "release_note": "notes",
                    "cdk_expired_time": 1790000000
                }
            }),
        );
        let sanitized = sanitize_body(&answer).expect("body present");
        assert!(sanitized.pointer("/data/url").is_none());
        assert!(sanitized.pointer("/data/cdk").is_none());
        assert_eq!(
            sanitized
                .pointer("/data/version_name")
                .and_then(Value::as_str),
            Some("v1.1.0")
        );
        assert_eq!(
            sanitized
                .pointer("/data/release_note")
                .and_then(Value::as_str),
            Some("notes")
        );
        assert_eq!(
            sanitized
                .pointer("/data/cdk_expired_time")
                .and_then(Value::as_i64),
            Some(1790000000)
        );
    }

    #[test]
    fn log_summary_contains_status_and_code_only() {
        let answer = answer(
            200,
            json!({"code": 0, "msg": "secret-ish message", "data": {"url": "https://mirrorchyan.com/x?cdk=SECRETCDK"}}),
        );
        let summary = log_summary(&answer);
        assert_eq!(summary, "status=200, code=0");
        assert!(!summary.contains("SECRETCDK"));
    }
}
