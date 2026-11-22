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

test('foundation-error-event forwards structured error codes without user text', async () => {
  const calls = []
  const listeners = new Map()
  const store = new Map()
  globalThis.sessionStorage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) }
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  try {
    const panel = createPanelTelemetry({
      getBase: async () => 'http://127.0.0.1:19999/',
      getVersion: async () => '1.0.8',
      fetchFn: async (_url, options) => { calls.push(JSON.parse(options.body)); return { ok: true, status: 200 } },
      win: {
        addEventListener(name, fn) { listeners.set(name, fn) },
        removeEventListener(name, fn) { if (listeners.get(name) === fn) listeners.delete(name) },
      },
      doc: undefined,
    })
    await panel.start({ startMs: 50 })
    listeners.get('foundation-error-event')?.({ detail: { component: 'usbip', code: 'usbip-setup-016.0xe0000247', severity: 'error' } })
    await panel.stop()
    const errorEvent = calls.flatMap((b) => b.events).find((e) => e.kind === 'error')
    assert.deepEqual(errorEvent, {
      kind: 'error',
      name: 'error',
      props: { component: 'usbip', code: 'usbip-setup-016.0xe0000247', severity: 'error', count: 1 },
    })
  } finally { delete globalThis.sessionStorage; delete globalThis.localStorage }
})

test('the update source of the start record comes from the native side, which never answers empty: the used source, else the setting', async () => {
  globalThis.sessionStorage = { getItem: () => null, setItem: () => {} }
  globalThis.localStorage = { getItem: () => null, setItem: () => {} }
  try {
    // the words of the contract: mirror, cnb, github, other (auto counts as other); the miss only when there is one
    for (const [native, wire] of [['mirror', 'mirror'], ['github', 'github'], ['cnb', 'cnb'], ['auto', 'other']]) {
      const calls = []
      const panel = createPanelTelemetry({
        getBase: async () => 'http://127.0.0.1:19999/', getVersion: async () => '1.0.0', getUpdateSource: async () => native, getUpdateSourceMiss: async () => (native === 'cnb' ? 'version_lag' : ''),
        fetchFn: async (url, options) => { calls.push(JSON.parse(options.body)); return { ok: true, status: 200 } },
        win: { addEventListener() {}, removeEventListener() {} }, doc: undefined,
      })
      await panel.start({ startMs: 100 })
      await panel.stop()
      assert.equal(calls[0].events[0].props.update_source, wire, native)
      assert.equal(calls[0].events[0].props.update_source_miss, native === 'cnb' ? 'version_lag' : undefined, native)
    }
  } finally { delete globalThis.sessionStorage; delete globalThis.localStorage }
})
