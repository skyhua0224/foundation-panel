//! The user's Windows proxy for Panel downloads.
//!
//! `reqwest` reads only the proxy environment variables unless the
//! `system-proxy` feature is enabled, and that feature stops at the static
//! ProxyServer value of the WinINet settings. Proxy clients configured through
//! the Settings app usually publish a PAC address (AutoConfigURL) or automatic
//! detection instead, so this module asks WinHTTP, the Windows component that
//! implements both resolution paths, for the proxy of each download address.
//!
//! Sources: docs/authority/source-index.tsv entries
//! panel-ms-winhttp-ieproxy-20261109, panel-ms-winhttp-getproxyforurl-20261109,
//! panel-ms-winhttp-autoproxy-20261109, panel-ms-wininet-internet-options-20261109,
//! panel-ms-wininet-vs-winhttp-20261109 and panel-reqwest-system-proxy-20261109.

/// The variables `reqwest` already follows. When one is set the download keeps
/// the client's own environment matcher, so `NO_PROXY` and the usual precedence
/// stay exactly as the user configured them.
const ENV_PROXY_KEYS: &[&str] = &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"];

pub(crate) fn environment_proxy_configured() -> bool {
    ENV_PROXY_KEYS.iter().any(|key| {
        std::env::var_os(key)
            .map(|value| !value.to_string_lossy().trim().is_empty())
            .unwrap_or(false)
    })
}

/// Whether the WinINet bypass list excludes this host. Entries are separated by
/// semicolons; `*` and `<local>` are the two special values documented by
/// Microsoft, and a leading dot (or `*.`) matches the domain and its
/// subdomains.
pub(crate) fn bypass_matches(bypass: &str, host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    for entry in bypass.split(&[';', ','][..]) {
        let entry = entry.trim().to_ascii_lowercase();
        if entry.is_empty() {
            continue;
        }
        if entry == "*" {
            return true;
        }
        if entry == "<local>" {
            if !host.contains('.') || host.parse::<std::net::Ipv4Addr>().is_ok() {
                return true;
            }
            continue;
        }
        let entry = entry
            .trim_start_matches("*.")
            .trim_start_matches('.')
            .trim_end_matches('/');
        if entry.is_empty() {
            continue;
        }
        if host == entry || host.ends_with(&format!(".{entry}")) {
            return true;
        }
    }
    false
}

/// Pick the entry of a WinINet `ProxyServer` value that applies to `scheme`.
/// The value is either `host:port` for every protocol or a list such as
/// `http=host:port;https=host:port;socks=host:port`.
fn proxy_entry<'a>(settings: &'a str, scheme: &str) -> Option<&'a str> {
    let wanted = if scheme.eq_ignore_ascii_case("https") {
        "https"
    } else {
        "http"
    };
    let mut bare = None;
    let mut named = None;
    for entry in settings.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        match entry.split_once('=') {
            Some((name, value)) => {
                if name.trim().eq_ignore_ascii_case(wanted) {
                    named = Some(value.trim());
                }
            }
            None => bare = Some(entry),
        }
    }
    named.or(bare)
}

/// Turn one WinINet proxy entry into a URL `reqwest` accepts. A SOCKS-only
/// entry is not used: the download client is built without the `socks`
/// transport, and connecting directly is preferable to failing every attempt.
pub(crate) fn proxy_url(entry: &str) -> Option<String> {
    let entry = entry.trim();
    if entry.is_empty() || entry.eq_ignore_ascii_case("direct") {
        return None;
    }
    let lower = entry.to_ascii_lowercase();
    if lower.starts_with("socks") {
        return None;
    }
    let host_port = entry
        .strip_prefix("http://")
        .or_else(|| entry.strip_prefix("https://"))
        .unwrap_or(entry)
        .trim_end_matches('/');
    if host_port.is_empty() || host_port.contains(['/', '?', '#', ' ']) {
        return None;
    }
    Some(format!("http://{host_port}"))
}

/// Resolve the proxy for one download address. `Some` means the request must
/// use that proxy; `None` leaves the client on its default direct path (or on
/// the environment matcher, when the caller did not replace it).
pub(crate) async fn resolve_for_url(url: &str) -> Option<String> {
    if environment_proxy_configured() {
        return None;
    }
    #[cfg(target_os = "windows")]
    {
        let url = url.to_string();
        // WinHTTP may download a PAC file or run DHCP/DNS discovery: never do
        // that on a runtime worker that also receives download progress.
        let joined = tokio::task::spawn_blocking(move || system_proxy_for(&url));
        match tokio::time::timeout(std::time::Duration::from_secs(8), joined).await {
            Ok(Ok(proxy)) => proxy,
            _ => None,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
        None
    }
}

/// Resolve the proxy from the current user's WinINet configuration through
/// WinHTTP. Called on a blocking thread.
#[cfg(target_os = "windows")]
fn system_proxy_for(url: &str) -> Option<String> {
    use windows::Win32::Networking::WinHttp::{
        WINHTTP_CURRENT_USER_IE_PROXY_CONFIG, WinHttpGetIEProxyConfigForCurrentUser,
    };

    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let scheme = parsed.scheme().to_string();

    unsafe {
        let mut config = WINHTTP_CURRENT_USER_IE_PROXY_CONFIG::default();
        if WinHttpGetIEProxyConfigForCurrentUser(&mut config).is_err() {
            return None;
        }
        let auto_config = take_winhttp_string(config.lpszAutoConfigUrl);
        let proxy = take_winhttp_string(config.lpszProxy);
        let bypass = take_winhttp_string(config.lpszProxyBypass);
        let auto_detect = config.fAutoDetect.as_bool();

        if auto_config.is_some() || auto_detect {
            if let Some(answer) = resolve_with_winhttp(url, auto_config.as_deref(), auto_detect) {
                return answer.filter(|_| !bypass_matches(bypass.as_deref().unwrap_or(""), &host));
            }
        }

        let settings = proxy?;
        if bypass_matches(bypass.as_deref().unwrap_or(""), &host) {
            return None;
        }
        proxy_url(proxy_entry(&settings, &scheme)?)
    }
}

/// Own a WinHTTP-allocated string and release it with `GlobalFree`, as the
/// WinHTTP documentation requires for the buffers of these calls.
#[cfg(target_os = "windows")]
unsafe fn take_winhttp_string(value: windows::core::PWSTR) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let text = unsafe { value.to_string() }.ok();
    let _ = unsafe {
        windows::Win32::Foundation::GlobalFree(Some(windows::Win32::Foundation::HGLOBAL(
            value.as_ptr() as *mut _,
        )))
    };
    text
}

/// Run WinHTTP's own PAC/auto-detect resolution. The outer `Option` says
/// whether WinHTTP answered at all; the inner one says whether the answer was
/// a proxy.
#[cfg(target_os = "windows")]
fn resolve_with_winhttp(
    url: &str,
    auto_config: Option<&str>,
    auto_detect: bool,
) -> Option<Option<String>> {
    use windows::Win32::Networking::WinHttp::{
        WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_AUTO_DETECT_TYPE_DHCP,
        WINHTTP_AUTO_DETECT_TYPE_DNS_A, WINHTTP_AUTOPROXY_AUTO_DETECT,
        WINHTTP_AUTOPROXY_CONFIG_URL, WINHTTP_AUTOPROXY_OPTIONS, WINHTTP_PROXY_INFO,
        WinHttpCloseHandle, WinHttpGetProxyForUrl, WinHttpOpen,
    };
    use windows::core::PCWSTR;

    let mut flags = 0;
    if auto_config.is_some() {
        flags |= WINHTTP_AUTOPROXY_CONFIG_URL;
    }
    if auto_detect {
        flags |= WINHTTP_AUTOPROXY_AUTO_DETECT;
    }
    if flags == 0 {
        return None;
    }

    let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let pac_wide: Vec<u16> = auto_config
        .map(|value| value.encode_utf16().chain(std::iter::once(0)).collect())
        .unwrap_or_default();

    unsafe {
        let session = WinHttpOpen(
            windows::core::w!("Sunshine Panel update download"),
            WINHTTP_ACCESS_TYPE_NO_PROXY,
            PCWSTR_NULL,
            PCWSTR_NULL,
            0,
        );
        if session.is_null() {
            return None;
        }
        let mut options = WINHTTP_AUTOPROXY_OPTIONS {
            dwFlags: flags,
            dwAutoDetectFlags: if auto_detect {
                WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A
            } else {
                0
            },
            lpszAutoConfigUrl: if pac_wide.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(pac_wide.as_ptr())
            },
            fAutoLogonIfChallenged: true.into(),
            ..Default::default()
        };
        let mut info = WINHTTP_PROXY_INFO::default();
        let result =
            WinHttpGetProxyForUrl(session, PCWSTR(url_wide.as_ptr()), &mut options, &mut info);
        let _ = WinHttpCloseHandle(session);
        if result.is_err() {
            return None;
        }
        let access = info.dwAccessType;
        let proxy = take_winhttp_string(info.lpszProxy);
        if !info.lpszProxyBypass.is_null() {
            let _ = take_winhttp_string(info.lpszProxyBypass);
        }
        if access == windows::Win32::Networking::WinHttp::WINHTTP_ACCESS_TYPE_NO_PROXY {
            return Some(None);
        }
        Some(proxy.and_then(|value| proxy_url(proxy_entry(&value, "https")?)))
    }
}

#[cfg(target_os = "windows")]
const PCWSTR_NULL: windows::core::PCWSTR = windows::core::PCWSTR::null();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_proxy_applies_to_https() {
        assert_eq!(
            proxy_entry("127.0.0.1:7890", "https"),
            Some("127.0.0.1:7890")
        );
        assert_eq!(
            proxy_url("127.0.0.1:7890").as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn per_scheme_proxy_picks_the_https_entry() {
        let settings = "http=127.0.0.1:7891;https=127.0.0.1:7890";
        assert_eq!(proxy_entry(settings, "https"), Some("127.0.0.1:7890"));
        assert_eq!(proxy_entry(settings, "http"), Some("127.0.0.1:7891"));
        assert_eq!(proxy_entry("socks=127.0.0.1:7890", "https"), None);
        assert_eq!(proxy_url("socks=127.0.0.1:7890"), None);
    }

    #[test]
    fn bypass_list_follows_wininet_syntax() {
        assert!(bypass_matches("<local>", "nas"));
        assert!(bypass_matches("<local>", "127.0.0.1"));
        assert!(!bypass_matches("<local>", "github.com"));
        assert!(bypass_matches(
            "*.example.com;github.com",
            "api.example.com"
        ));
        assert!(bypass_matches("*.example.com;github.com", "github.com"));
        assert!(!bypass_matches("*.example.com", "example.org"));
        assert!(bypass_matches("*", "anything.invalid"));
    }

    #[test]
    fn a_normalized_proxy_url_has_no_path_or_credentials() {
        assert_eq!(
            proxy_url("  http://127.0.0.1:7890/ ").as_deref(),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(proxy_url("127.0.0.1:7890/pac"), None);
        assert_eq!(proxy_url("direct"), None);
    }
}
