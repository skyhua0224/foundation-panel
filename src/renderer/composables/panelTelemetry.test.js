import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { createPanelTelemetry } from './panelTelemetry.js'

const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')

test('the panel hands its events to the host through its proxy, in the contract shape', async () => {
  const calls = []
  const store = new Map()
  globalThis.sessionStorage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) }
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  try {
    const panel = createPanelTelemetry({
      getBase: async () => 'http://127.0.0.1:19999/', getVersion: async () => '1.4.0.1112.杂鱼', getUpdateSource: () => 'mirror',
      fetchFn: async (url, options) => { calls.push({ url, options, body: JSON.parse(options.body) }); return { ok: true, status: 200 } },
      win: { addEventListener() {}, removeEventListener() {} }, doc: undefined,
    })
    await panel.start({ startMs: 640.2 })
    panel.feature('pair_open')
    await panel.stop()
    assert.equal(calls.length, 1)
    assert.equal(calls[0].url, 'http://127.0.0.1:19999/api/telemetry/events')
    assert.equal(calls[0].options.method, 'POST'); assert.equal(calls[0].options.keepalive, true)
    const { product, version, events } = calls[0].body
    assert.equal(product, 'panel'); assert.equal(version, '1.4.0.1112.杂鱼')
    assert.deepEqual(events[0], { kind: 'usage', name: 'app_start', props: { cold: true, start_ms: 640, update_source: 'mirror' } })
    assert.deepEqual(events[1], { kind: 'usage', name: 'feature_used', props: { feature: 'panel.pair_open' } })
  } finally { delete globalThis.sessionStorage; delete globalThis.localStorage }
})

test('no proxy yet, or a host that is away: nothing throws and nothing is kept for ever', async () => {
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  try {
    let base = ''
    const panel = createPanelTelemetry({ getBase: async () => base, fetchFn: async () => { throw new Error('down') }, win: { addEventListener() {} }, doc: undefined })
    await panel.start()
    panel.feature('a')
    await assert.doesNotReject(panel.stop())
    base = 'http://127.0.0.1:1'
    panel.feature('b')
    await assert.doesNotReject(panel.stop())
  } finally { delete globalThis.localStorage }
})

test('the panel shell starts it once, tells it the page, and the host is the single owner of identity and choices', () => {
  const shell = read('../components/SidebarMenu.vue')
  assert.match(shell, /createPanelTelemetry\(/)
  assert.match(shell, /panelTelemetry\.start\(/)
  assert.match(shell, /panelTelemetry\.page\(/)
  assert.match(shell, /SOURCE_KEY/)
  assert.doesNotMatch(shell, /firebase|google-analytics|gtag/i)
})

test('the reports the Rust shell wrote travel as error_report events through the same sender, at start and when the shell says it wrote one', async () => {
  const calls = []
  const store = new Map()
  globalThis.sessionStorage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) }
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  const props = { component: 'panel-shell', lang: 'rust', fingerprint: '027fe2f9e2fdcceb', code: 'panic', severity: 'fatal', loc: 'src/clipboard.rs:1196', frames: ['src/clipboard.rs:1196'], crumbs: ['3|act|save_settings'], log: ['[x] ERROR a'], count: 1 }
  let waiting = [{ kind: 'error', name: 'error_report', props }, { kind: 'usage', name: 'app_start', props: {} }, null, { kind: 'error', name: 'error_report', props: 'text' }]
  let notify = () => {}
  let listening = true
  try {
    const panel = createPanelTelemetry({
      getBase: async () => 'http://127.0.0.1:19999',
      drainShellErrors: async () => { const out = waiting; waiting = []; return out },
      onShellError: async (fn) => { notify = fn; return () => { listening = false } },
      fetchFn: async (url, options) => { calls.push(JSON.parse(options.body)); return { ok: true, status: 200 } },
      win: { addEventListener() {}, removeEventListener() {} }, doc: undefined,
    })
    await panel.start()
    await panel.stop()
    const errors = calls.flatMap((c) => c.events).filter((e) => e.name === 'error_report')
    assert.equal(errors.length, 1, 'only the well-formed error_report event is taken')
    assert.deepEqual(errors[0], { kind: 'error', name: 'error_report', props })
    // the shell survives a panic of a thread: it says so, the page takes the new report
    waiting = [{ kind: 'error', name: 'error_report', props: { ...props, severity: 'error' } }]
    notify()
    await new Promise((resolve) => setTimeout(resolve, 20))
    assert.equal(calls.flatMap((c) => c.events).filter((e) => e.name === 'error_report').length, 2)
    assert.equal(listening, false, 'stop() stops listening')
  } finally { delete globalThis.sessionStorage; delete globalThis.localStorage }
})

test('a shell that cannot be asked (a browser, a failing command) changes nothing', async () => {
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  try {
    const panel = createPanelTelemetry({
      getBase: async () => 'http://127.0.0.1:19999',
      drainShellErrors: async () => { throw new Error('no such command') },
      onShellError: async () => { throw new Error('no event bus') },
      fetchFn: async () => ({ ok: true, status: 200 }), win: { addEventListener() {} }, doc: undefined,
    })
    await assert.doesNotReject(panel.start())
    await assert.doesNotReject(panel.stop())
  } finally { delete globalThis.localStorage }
})

test('the shell wires the drain command and the shell-error event into the panel telemetry', () => {
  const shell = read('../components/SidebarMenu.vue')
  assert.match(shell, /invoke\('error_capture_drain'\)/)
  assert.match(shell, /listen\('error-capture'/)
})
