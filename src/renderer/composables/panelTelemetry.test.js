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
