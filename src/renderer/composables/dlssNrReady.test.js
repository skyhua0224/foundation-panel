import test from 'node:test'
import assert from 'node:assert/strict'
import { createDlssNrReady, dlssNrUsable } from './dlssNrReady.js'

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
