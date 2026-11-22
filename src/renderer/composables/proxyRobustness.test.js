import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// Requirement: the panel talks to a host on the same computer, and the host config API, the pages and their chunks must not fail or wait on it in normal conditions. The
// causes found are pinned here, in the source of the proxy, because the Rust tests run on Windows only.
const rs = fs.readFileSync(new URL('../../../src-tauri/src/proxy_server.rs', import.meta.url), 'utf8')
const vigem = fs.readFileSync(new URL('../../../src-tauri/src/vigem.rs', import.meta.url), 'utf8')
const app = fs.readFileSync(new URL('../../../src-tauri/src/app.rs', import.meta.url), 'utf8')

test('one slow request does not turn the next three seconds of requests into 503s', () => {
  // connection_failure_response marks the host away only for a refused connection
  assert.match(rs, /if host_is_gone\(error\) \{\s*mark_unavailable\(\);\s*\}/)
  assert.match(rs, /const FAST_FAIL_COOLDOWN_SECS: u64 = 1;/)
  // the error text of a failed request is the whole chain: reqwest says only "error sending request" and keeps "timed out" in its source
  assert.match(rs, /let error_str = error_chain\(&\*e\);/)
})

test('the pairing long poll is carried by a client whose read timeout outlasts its wait', () => {
  assert.match(rs, /const LONG_POLL_CLIENT_TIMEOUT_SECS: u64 = 70;/)
  assert.match(rs, /else if long_poll_timeout\(method, url\)\.is_some\(\) \{\s*get_long_poll_http_client\(\)/)
  assert.match(rs, /const DEFAULT_PROXY_TIMEOUT_SECS: u64 = 15;/)
  assert.match(rs, /const PROXY_CONNECT_TIMEOUT_MS: u64 = 3000;/)
})

test('the host\'s hashed files are kept, told to the WebView as immutable, and fetched once in the background when the host first answers', () => {
  assert.match(rs, /let cacheable_asset = method == axum::http::Method::GET && is_hashed_asset\(&path\);\s*if cacheable_asset \{\s*if let Some\(response\) = asset_cache_get\(&path\)/)
  assert.match(rs, /public, max-age=31536000, immutable/)
  assert.match(rs, /tauri::async_runtime::spawn\(prewarm_assets\(\)\)/)
})

test('the readiness handshake: the probe waits for the host inside one call and asks again every 100 ms, so a start-up race ends in milliseconds', () => {
  const probe = rs.slice(rs.indexOf('pub async fn probe_sunshine_host'), rs.indexOf('async fn proxy_health_handler'))
  assert.match(probe, /loop \{/); assert.match(probe, /tokio::time::sleep\(std::time::Duration::from_millis\(100\)\)/); assert.match(probe, /return Err\(text\)/)
})

test('the driver probe runs on a blocking thread, once for the pages that ask together, and the probes with a cache are run at the start', () => {
  assert.match(vigem, /static VIGEM_STATUS: crate::probe_cache::ProbeCache<VigemStatus>/)
  assert.match(vigem, /tokio::task::spawn_blocking\(check_vigem\)/)
  assert.match(vigem, /let _status_changes = VIGEM_STATUS\.invalidating\(\);/)
  assert.match(app, /crate::vdd::get_vdd_status\(\)\.await;[\s\S]*crate::vigem::get_vigem_status\(\)\.await;/)
})

test('everything that contacts the host at the start waits for it through the same readiness wait (proxy_server::host_ready)', () => {
  const src = (f) => fs.readFileSync(new URL(`../../../src-tauri/src/${f}`, import.meta.url), 'utf8')
  assert.match(rs, /pub\(crate\) async fn wait_for_host\(timeout: std::time::Duration\) -> Result<u16, String>/)
  assert.match(rs, /pub async fn host_ready\(max: std::time::Duration\) -> bool/); assert.match(rs, /pub fn host_not_ready\(\)/)
  assert.match(rs, /pub async fn probe_sunshine_host\(timeout_ms: Option<u64>\) -> Result<u16, String> \{[\s\S]{0,260}wait_for_host\(timeout\)\.await/, 'the window\'s probe is the same wait')
  // the DLSS NR status (the shortcuts), tried again until the host answers
  assert.match(src('nr_overlay.rs'), /for attempt in 0\.\.8u32 \{\s*let _ = crate::proxy_server::host_ready\(std::time::Duration::from_secs\(20\)\)\.await;/)
  assert.match(src('hdr_enhanced.rs'), /let _ = crate::proxy_server::host_ready\(std::time::Duration::from_secs\(8\)\)\.await;/)
  // the clipboard event stream: ready first, and a 401 is "not ready yet", asked again at once
  const clip = src('clipboard.rs')
  assert.match(clip, /let _ = crate::proxy_server::host_ready\(std::time::Duration::from_secs\(30\)\)\.await;\s*let url = match get_sunshine_url\(\)/)
  assert.match(clip, /resp\.status\(\) == reqwest::StatusCode::UNAUTHORIZED[\s\S]{0,400}crate::proxy_server::host_not_ready\(\)/)
  // the tray state and its event stream
  assert.match(src('tray/events.rs'), /let _ = crate::proxy_server::host_ready\(Duration::from_secs\(30\)\)\.await;\s*let supports_events = handle_state_result\(\s*sunshine::get_tray_state\(\)\.await/)
  // a loopback connect is accepted or refused at once: 500 ms made a starved host a failed call
  assert.match(src('sunshine.rs'), /\.connect_timeout\(std::time::Duration::from_millis\(3000\)\)\s*\.read_timeout\(std::time::Duration::from_secs\(10\)\)/)
})
