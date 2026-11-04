import test from 'node:test'
import assert from 'node:assert/strict'
import { startSlowCallLog, trackCalls } from './slowCalls.js'

const clock = () => { let t = 0; return { now: () => t, advance: (ms) => { t += ms } } }

test('a call under the limit writes nothing, one over it writes its name and time', async () => {
  const c = clock(); const lines = []
  const slow = (ms) => new Promise((resolve) => { c.advance(ms); resolve('x') })
  const { wrapped } = trackCalls((name, ms) => slow(ms), { report: (...line) => lines.push(line), now: c.now, kind: 'invoke', limit: 500 })
  await wrapped('quick', 100)
  await wrapped('slow_probe', 900)
  assert.deepEqual(lines.map(([kind, name, ms]) => [kind, name, ms]), [['invoke', 'slow_probe', 900]])
})

test('the calls that have not returned are listed with their age', async () => {
  const c = clock(); let release
  const hang = () => new Promise((resolve) => { release = resolve })
  const { wrapped, pending } = trackCalls(hang, { report: () => {}, now: c.now, kind: 'fetch', nameOf: () => '/api/config' })
  const call = wrapped('/api/config')
  c.advance(2500)
  assert.deepEqual(pending(), [{ name: '/api/config', ms: 2500 }])
  release(1); await call
  assert.deepEqual(pending(), [])
})

test('a call that throws is reported and rethrown', () => {
  const c = clock(); const lines = []
  const { wrapped } = trackCalls(() => { c.advance(700); throw new Error('no') }, { report: (...l) => lines.push(l), now: c.now, kind: 'invoke', limit: 500 })
  assert.throws(() => wrapped('cmd'), /no/)
  assert.equal(lines.length, 1)
})

test('the Tauri runtime is not touched, and the commands that have not returned are part of the late-timer report', () => {
  const internals = {}
  const invoke = () => Promise.resolve()
  Object.defineProperty(internals, 'invoke', { value: invoke, writable: false, enumerable: true })
  let tick = null
  const lines = []
  const win = {
    __TAURI_INTERNALS__: internals,
    setInterval: (fn) => { tick = fn; return 1 },
    clearInterval: () => {},
  }
  let now = 0
  const realNow = performance.now.bind(performance)
  performance.now = () => now
  try {
    const stop = startSlowCallLog((...line) => lines.push(line), win, () => [{ name: 'get_vdd_status', ms: 4000 }])
    assert.equal(internals.invoke, invoke)
    now = 5000
    tick()
    assert.equal(lines.length, 1)
    assert.equal(lines[0][0], 'timer')
    assert.match(lines[0][3], /get_vdd_status 4000ms/)
    stop()
  } finally { performance.now = realNow }
})
