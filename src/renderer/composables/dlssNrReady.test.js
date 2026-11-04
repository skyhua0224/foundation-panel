import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createDlssNrReady, dlssNrUsable, qualityParts } from './dlssNrReady.js'

const usable = { installed: true, host_supported: true, adapter_present: true }

test('the live controls show only for an installed and usable DLSS NR', () => {
  assert.equal(dlssNrUsable(usable), true)
  assert.equal(dlssNrUsable({ ...usable, installed: false }), false)
  assert.equal(dlssNrUsable({ ...usable, host_supported: false }), false)
  assert.equal(dlssNrUsable({ ...usable, adapter_present: false }), false)
  assert.equal(dlssNrUsable(null), false)
})

test('not installed hides them, installed and supported shows them', async () => {
  const missing = createDlssNrReady(async () => ({ success: true, data: { ...usable, installed: false } }))
  const a = missing.use(); await missing.read()
  assert.equal(a.value, false)
  const ready = createDlssNrReady(async () => ({ success: true, data: usable }))
  const b = ready.use(); await ready.read()
  assert.equal(b.value, true)
})

test('unknown keeps them hidden: while reading, and when the call fails', async () => {
  let release
  const slow = createDlssNrReady(() => new Promise((r) => { release = r }))
  const shown = slow.use()
  assert.equal(shown.value, false)
  assert.equal(slow.state.value, null)
  await new Promise(setImmediate)
  assert.equal(shown.value, false)
  release({ success: true, data: usable }); await slow.read()
  assert.equal(shown.value, true)
  for (const fail of [async () => ({ success: false, message: 'denied' }), async () => { throw new Error('ipc') }]) {
    const s = createDlssNrReady(fail); const v = s.use(); await s.read()
    assert.equal(v.value, false); assert.equal(s.state.value, null)
  }
})

test('the status is read once and shared; an answer handed in from 画质增强 updates every user', async () => {
  let calls = 0
  const s = createDlssNrReady(async () => { calls++; return { success: true, data: { ...usable, installed: false } } })
  const a = s.use(), b = s.use(); await s.read()
  s.use()
  assert.equal(calls, 1)
  assert.equal(a.value, false)
  s.note(usable)
  assert.equal(a.value, true); assert.equal(b.value, true)
  // a failed read later does not hide what is known
  s.note(null)
  assert.equal(a.value, true)
})

test('画质增强 before DLSS NR is ready: the component state, the install flow and the output format; the overlay, its keys and the NR parts only once ready', () => {
  for (const unknownOrMissing of [null, false]) {
    const p = qualityParts(unknownOrMissing)
    assert.deepEqual([p.components, p.install, p.session, p.output], [true, true, true, true])
    assert.deepEqual([p.overlay, p.keys, p.nrState, p.liveControls], [false, false, false, false])
  }
  const ready = qualityParts(true)
  assert.deepEqual([ready.overlay, ready.keys, ready.nrState, ready.liveControls], [true, true, true, true])
})

test('the page puts the overlay switch, the shortcut keys and the live NR parts under those parts', () => {
  const page = readFileSync(new URL('../components/HdrEnhancedManager.vue', import.meta.url), 'utf8')
  const tag = (re) => { const m = re.exec(page); assert.ok(m, String(re)); return m[0] }
  assert.match(tag(/<MGroup[^>]*>\s*<MSwitchItem :model-value="visible"/), /v-if="parts\.overlay"/)
  assert.match(tag(/<MSection[^>]*:title="text\.keys"[^>]*>/), /v-if="parts\.keys"/)
  assert.match(tag(/<div v-else-if="current === 'keys'[^>]*>/), /parts\.keys/)
  assert.match(tag(/<MItem[^>]*:title="text\.enhancement"[^>]*>/), /v-if="parts\.nrState"/)
  assert.match(tag(/<NrSessionControls[^>]*>/), /parts\.liveControls/)
  assert.match(tag(/<MButton[^>]*@click="setVisibility\(true\)"[^>]*>/), /parts\.liveControls/)
  const live = readFileSync(new URL('../components/m3e/LiveSessionPage.vue', import.meta.url), 'utf8')
  assert.match(live, /<MSection v-if="nrReady" :title="t\.m3e\.display\.quality">/)
})
