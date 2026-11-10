import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

const shell = fs.readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')

test('the panel shows the 「Sunshine 意外停止」 surface when the host stops without being asked, and starts the automatic report', () => {
  assert.match(shell, /onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/)
  assert.match(shell, /startAutoReport\(\{ context: errorContext \}\)/)
  assert.match(shell, /setErrorContext\(errorContext\)/)
  assert.match(shell, /<MErrorHost[^>]*:restart="\(\) => hostConn\.restart\('service'\)"/, 'the restart of a host that does not answer is the way out')
})

test('the connection reads the facts of the operating system, so that a silence of the host is not announced as a stop', () => {
  assert.match(shell, /facts: \(\) => tools\.getHostFacts\(\)/)
  assert.match(shell, /facts: \(\) => tools\.getHostFacts\(\),\s*\n\s*onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/, 'the facts come before the stop surface in the options')
  const adapter = fs.readFileSync(new URL('../tauri-adapter.js', import.meta.url), 'utf8')
  assert.match(adapter, /getHostFacts: \(\) => invoke\('get_sunshine_host_facts'\)/)
})

test('a host that runs and does not answer opens 「Sunshine 没有响应」 and the dialog closes when it answers again', () => {
  assert.match(shell, /onHung: \(\{ at, seconds \}\) => reportHostHung\(\{ at, seconds \}\)/)
  assert.match(shell, /onHungEnd: \(\) => dismissHostHung\(\)/)
  assert.match(shell, /reportHostStop, reportHostHung, dismissHostHung,/, 'both come from the shared package')
  assert.match(shell, /onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/, 'a stop is still opened only on the facts')
})

test('the facts carry the process id of the service wrapper, so that a restart of the service is told from a crash', () => {
  const rust = fs.readFileSync(new URL('../../../src-tauri/src/host_facts.rs', import.meta.url), 'utf8')
  assert.match(rust, /pub service_pid: u32/)
  assert.match(rust, /status\.dwProcessId/)
  assert.match(rust, /let \(service, service_pid\) = service_state\(\)/)
})

test('the status page is drawn once, over the whole content area: the overlay holds it for every page, the detail pane holds nothing while the host cannot be used', async () => {
  const fs = await import('node:fs')
  const src = fs.readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')
  assert.match(src, /<template #detail=\"\{ entry \}\">[\s\S]*?<template v-if=\"offlineShown && !isWebPage\" \/>/)
  assert.match(src, /<template #overlay>[\s\S]*<MHostOffline v-if=\"offlineShown\"/)
  assert.equal((src.match(/<MHostOffline/g) || []).length, 1)
  assert.match(src, /:overlay=\"isWebPage \|\| offlineShown\" :status=\"offlineShown\"/)
})

test('the offline tips get the real port of the Web UI from the URL the panel talks to', async () => {
  const fs = await import('node:fs')
  const src = fs.readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')
  assert.equal((src.match(/:port=\"hostPort\"/g) || []).length, 1)
  assert.match(src, /new URL\(String\(await sunshine\.getUrl\(\)\)\)\.port/)
})

test('the pairing code of the connect dialog is an SVG, not a PNG made by a canvas (38-47 ms in one task at the dialog\'s opening)', async () => {
  const fs = await import('node:fs')
  for (const f of ['SidebarMenu.vue', 'PanelHome.vue']) {
    const src = fs.readFileSync(new URL(`./${f}`, import.meta.url), 'utf8')
    assert.ok(!src.includes('QRCode.toDataURL'), f)
    assert.match(src, /render: \(url\) => qrSvgDataUrl\(QRCode\.create\(url, \{ errorCorrectionLevel: 'M' \}\)\.modules\)/, f)
  }
})

test('the host is probed by the panel\'s own process, not by a fetch that can queue behind the window\'s own first requests', () => {
  assert.match(shell, /const status = await hostData\.probe\(hostConn\.state\.status === 'connecting' \? 6000 : 2500\)/)
  assert.match(shell, /return status >= 100 && status < 600/, 'a valid 503 response is host reachability, not a dead service')
  assert.doesNotMatch(shell, /status !== 502|status !== 503/, 'the old single-status offline classification is gone')
  assert.doesNotMatch(shell, /const probeHost = async \(\) => \{\s*\n\s*const base/, 'the old fetch probe')
  const adapter = fs.readFileSync(new URL('../tauri-adapter.js', import.meta.url), 'utf8')
  assert.match(adapter, /probeHost: \(timeoutMs = 2500\) => invoke\('probe_sunshine_host', \{ timeoutMs \}\)/)
  const rust = fs.readFileSync(new URL('../../../src-tauri/src/proxy_server.rs', import.meta.url), 'utf8')
  assert.match(rust, /pub async fn probe_sunshine_host\(timeout_ms: Option<u64>\) -> Result<u16, String>/)
  assert.match(fs.readFileSync(new URL('../../../src-tauri/src/main.rs', import.meta.url), 'utf8'), /proxy_server::probe_sunshine_host,/)
})
