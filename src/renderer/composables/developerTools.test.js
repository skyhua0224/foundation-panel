import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { ONBOARDING_KEYS, PALETTE_KEY, OVERVIEW_PREFIX, DEV_FLAGS_KEY, createDeveloperTools, clearPalette, logLine, paletteCount, percentOf, resetOnboarding, resetOverview, resetStorage } from './developerTools.js'

const memoryStorage = (seed = {}) => {
  const data = new Map(Object.entries(seed))
  return { get length() { return data.size }, key: (i) => [...data.keys()][i] ?? null, getItem: (k) => (data.has(k) ? data.get(k) : null), setItem: (k, v) => data.set(k, String(v)), removeItem: (k) => data.delete(k), keys: () => [...data.keys()] }
}
const fakeInvoke = (answers = {}) => { const calls = []; const invoke = async (name, args) => { calls.push([name, args]); const a = answers[name]; if (a instanceof Error) throw a; return typeof a === 'function' ? a(args) : a }; invoke.calls = calls; return invoke }

test('the palette cache counts its pictures and goes away without touching the other keys', () => {
  const s = memoryStorage({ [PALETTE_KEY]: JSON.stringify({ a: 1, b: 2 }), other: 'x' })
  assert.equal(paletteCount(s), 2)
  clearPalette(s)
  assert.deepEqual(s.keys(), ['other'])
  assert.equal(paletteCount(memoryStorage({ [PALETTE_KEY]: '{broken' })), 0)
})
test('the first-run guide, the tour and the dismissed recommendations are reset, nothing else', () => {
  const local = memoryStorage({ 'foundation.coach.panel': '1', 'foundation.guide.panel': '1', 'foundation-ui.clientrec.pixel': 'x', 'sunshine-theme': 'dark' })
  const session = memoryStorage({ 'foundation.guide.dismissed': '1' })
  resetOnboarding(local, session)
  assert.deepEqual(local.keys(), ['sunshine-theme'])
  assert.deepEqual(session.keys(), [])
  for (const key of ONBOARDING_KEYS) assert.ok(key.startsWith('foundation.'))
})
test('the overview layout is reset on its own', () => {
  const s = memoryStorage({ [`${OVERVIEW_PREFIX}panel-home`]: '{}', 'sunshine-locale': 'zh' })
  resetOverview(s)
  assert.deepEqual(s.keys(), ['sunshine-locale'])
})
test('a full reset keeps the developer mode and the overlay choices, so the page stays where it is', () => {
  const s = memoryStorage({ 'foundation-ui.dev': '1', [DEV_FLAGS_KEY]: '{"hud":true}', 'sunshine-theme': 'dark', 'sunshine-desktop-settings': '{}' })
  resetStorage(s)
  assert.deepEqual(s.keys().sort(), [DEV_FLAGS_KEY, 'foundation-ui.dev'].sort())
})
test('a log entry is the line the file has, which the page parses', () => {
  assert.equal(logLine({ timestamp: '2026-11-04 09:50:54.272', level: 'warn', file: 'src\\logger.rs', line: 270, message: '[renderer] fetch /x 3621 ms' }), '[2026-11-04 09:50:54.272] [WARN] [src\\logger.rs:270] [renderer] fetch /x 3621 ms')
  assert.equal(logLine({ timestamp: 't', level: 'info', message: 'm' }), '[t] [INFO] m')
})
test('the scale of a simulation is a whole percentage, and nothing means the real one', () => {
  assert.equal(percentOf(0), 100)
  assert.equal(percentOf(150.4), 150)
  assert.equal(percentOf('200'), 200)
})
test('the tools ask the native side with the names and the arguments the commands take', async () => {
  const invoke = fakeInvoke({ get_include_prerelease_preference: false, check_for_updates_for_channel: { is_latest: false, version: '2026.1105.1' }, dev_get_state: { cover: 'fast', coldCloak: false, dpi: 192 } })
  const tools = createDeveloperTools({ invoke, settings: { get: () => ({ devMode: true, logLevel: 'debug' }), save: async () => {} } })
  await tools.window.setCover('adaptive'); await tools.window.setCloak(true); await tools.window.simulateDpi(150); await tools.window.simulateDpi(0)
  await tools.webview.devtools(); await tools.webview.clearCache(); await tools.update.channel.set('prerelease')
  const names = Object.fromEntries(invoke.calls.map(([n, a]) => [n, a]))
  assert.deepEqual(names.dev_set_cover_mode, { mode: 'adaptive' })
  assert.deepEqual(names.dev_set_cold_cloak, { on: true })
  assert.deepEqual(invoke.calls.filter(([n]) => n === 'dev_simulate_dpi').map(([, a]) => a.percent), [150, 100])
  assert.deepEqual(names.set_include_prerelease_preference, { include: true })
  assert.ok('dev_open_devtools' in names && 'dev_clear_webview_cache' in names)
  assert.deepEqual(await tools.update.check(), { latest: false, version: '2026.1105.1' })
  assert.deepEqual(invoke.calls.at(-1), ['check_for_updates_for_channel', { channel: 'stable' }])
  assert.equal(await tools.debugOutput.get(), true)
  assert.equal(await tools.logLevel.get(), 'debug')
})
test('the settings are saved as the whole record with the one value changed', async () => {
  let saved = null
  const tools = createDeveloperTools({ invoke: fakeInvoke(), settings: { get: () => ({ devMode: false, logLevel: 'info', autoStart: true }), save: async (v) => { saved = v } } })
  await tools.debugOutput.set(true)
  assert.deepEqual(saved, { devMode: true, logLevel: 'info', autoStart: true })
  await tools.logLevel.set('trace')
  assert.deepEqual(saved, { devMode: false, logLevel: 'trace', autoStart: true })
})
test('the full reset asks the native side, empties the storage but the developer mode, and starts the window again', async () => {
  const invoke = fakeInvoke(); const local = memoryStorage({ 'foundation-ui.dev': '1', a: '1' }); const session = memoryStorage({ b: '1' }); let reloaded = 0
  const tools = createDeveloperTools({ invoke, settings: { get: () => ({}), save: async () => {} }, local, session, reload: () => { reloaded += 1 } })
  await tools.data.resetAll()
  assert.deepEqual(invoke.calls.map(([n]) => n), ['dev_reset_desktop_settings'])
  assert.deepEqual(local.keys(), ['foundation-ui.dev']); assert.deepEqual(session.keys(), []); assert.equal(reloaded, 1)
})
test('the telemetry test event is sent only as a real send, and a failure is a failure', async () => {
  const doc = { schema: 1 }
  const kit = (state) => ({ consent: () => 'on', queue: () => 2, report: (e) => ({ message: e.message }), document: () => doc, send: async () => ({ state, reason: 'http-403' }) })
  const ok = createDeveloperTools({ invoke: fakeInvoke(), settings: { get: () => ({}), save: async () => {} }, kit: kit('sent') })
  assert.deepEqual(await ok.telemetry.state(), { consent: true, installId: '', queue: 2 })
  assert.equal(await ok.telemetry.preview(), doc)
  assert.equal((await ok.telemetry.test()).state, 'sent')
  const bad = createDeveloperTools({ invoke: fakeInvoke(), settings: { get: () => ({}), save: async () => {} }, kit: kit('rejected') })
  await assert.rejects(bad.telemetry.test(), /http-403/)
  const off = createDeveloperTools({ invoke: fakeInvoke(), settings: { get: () => ({}), save: async () => {} }, kit: { ...kit('sent'), consent: () => 'off' } })
  assert.equal((await off.telemetry.state()).consent, false)
})
test('the raw reachability answer keeps the status, the time and the body', async () => {
  const realFetch = globalThis.fetch
  globalThis.fetch = async (url) => { assert.equal(url, 'http://h:1/api/network/reachability'); return new Response('{"lan":{"ok":true}}', { status: 200 }) }
  try {
    const tools = createDeveloperTools({ invoke: fakeInvoke(), settings: { get: () => ({}), save: async () => {} }, base: async () => 'http://h:1/' })
    const r = await tools.host.reach()
    assert.equal(r.status, 200); assert.deepEqual(r.body, { lan: { ok: true } }); assert.ok(r.ms >= 0)
  } finally { globalThis.fetch = realFetch }
})
test('the environment is gathered from the commands that exist and survives one that fails', async () => {
  const invoke = fakeInvoke({ get_system_info: { app_version: '2026.1104.1', os_version: 'Windows 11', arch: 'x86_64' }, get_gpus: ['RTX 4080'], dev_environment: { webview: '156.0', target: 'x86_64' }, get_include_prerelease_preference: new Error('no') })
  const tools = createDeveloperTools({ invoke, settings: { get: () => ({}), save: async () => {} }, hostVersion: async () => 'Sunshine 1', win: { devicePixelRatio: 2, screen: { width: 1920, height: 1080 } } })
  const env = await tools.environment()
  assert.equal(env.version, '2026.1104.1'); assert.equal(env.gpu, 'RTX 4080'); assert.equal(env.webview, '156.0'); assert.equal(env.display, '3840x2160 @200%'); assert.equal(env.host, 'Sunshine 1'); assert.equal(env.channel, '')
})

// the wiring is read from the sources (the router and the shell need the bundler)
const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')
test('the developer pages are the rail 开发者 (only in developer mode), 开发者选项 is one of them, 系统 and 关于 keep their entries', () => {
  const router = read('./useRouter.js'); const nav = read('./panelNav.js'); const shell = read('../components/SidebarMenu.vue'); const about = read('../components/m3e/AboutPage.vue'); const options = read('../components/m3e/DeveloperOptions.vue')
  assert.match(router, /\[ROUTES\.DEV_ISSUES\]: \{ rail: 'developer' \}/, 'Issue 与 PR is the first page of the rail')
  assert.match(router, /\[ROUTES\.DEVELOPER\]: \{ rail: 'developer' \}/)
  assert.ok(router.indexOf('[ROUTES.DEV_ISSUES]: {') < router.indexOf('[ROUTES.DEVELOPER]: {'))
  assert.match(router, /RAILS = \[[^\]]*'assistant', 'developer', 'settings', 'help'\]/, 'above 面板设置 and 帮助与关于')
  assert.match(nav, /const developer = runtime\.developer \?/)
  assert.match(nav, /\.\.\.developer,/, 'the 系统 list keeps its link')
  assert.match(nav, /if \(!runtime\.developer \|\| !D\) return \[\]/, 'no rows while developer mode is off')
  assert.match(shell, /<DeveloperPages v-else-if="DEV_PAGES\.includes\(entry\.id\)"/)
  assert.match(shell, /filter\(\(item\) => item\.id !== 'developer' \|\| developerMode\.enabled\.value\)/, 'the rail item is there only in developer mode')
  assert.match(shell, /NEEDS_NO_HOST = new Set\(\[[^\]]*ROUTES\.DEVELOPER,/, 'the page stays reachable while a host loss is simulated')
  assert.match(shell, /ROUTES\.DEV_ISSUES, ROUTES\.DEV_LOGS/)
  assert.match(shell, /@open-developer="router\.navigate\(ROUTES\.DEVELOPER\)"/)
  assert.match(about, /emit\('open-developer'\)/)
  assert.match(options, /data-panel-option="developer-page"/)
})
test('every command of the page is local-only and granted to the main window', () => {
  const build = read('../../../src-tauri/build.rs'); const cap = JSON.parse(read('../../../src-tauri/capabilities/main-developer.json')); const main = read('../../../src-tauri/src/main.rs')
  const commands = [...main.matchAll(/dev_tools::(dev_\w+),/g)].map((m) => m[1])
  assert.equal(commands.length, 9)
  for (const c of commands) {
    assert.ok(build.includes(`"${c}"`), `${c} is local-only`)
    assert.ok(cap.permissions.includes(`allow-${c.replaceAll('_', '-')}`), `${c} is granted`)
    assert.ok(!read('../../../src-tauri/webui-permissions.json').includes(c.replaceAll('_', '-')), `${c} is not granted to the Web UI`)
  }
  assert.deepEqual(cap.windows, ['main'])
})
