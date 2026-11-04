import test from 'node:test'
import assert from 'node:assert/strict'

// the package's invoke calls window.__TAURI_INTERNALS__.invoke: a runtime that answers as the host does, with the time it is given
let clockNow = 0
const answers = []
globalThis.window = { __TAURI_INTERNALS__: { invoke: (cmd, args) => { answers.push([cmd, args]); return new Promise((resolve) => { clockNow += (args && args.ms) || 0; resolve(`done ${cmd}`) }) }, transformCallback: () => 1 } }
const core = await import('./tauri-core-timed.js')

test('the commands are timed with the slow-call limit, and the answer is the host\'s', async () => {
  const lines = []
  core.configureInvokeTiming({ report: (...line) => lines.push(line), now: () => clockNow, limit: 500 })
  assert.equal(await core.invoke('quick', { ms: 100 }), 'done quick')
  assert.equal(await core.invoke('slow_probe', { ms: 900 }), 'done slow_probe')
  assert.deepEqual(lines.map(([kind, name, ms]) => [kind, name, ms]), [['invoke', 'slow_probe', 900]])
  assert.deepEqual(answers.map(([cmd]) => cmd), ['quick', 'slow_probe'])
})

test('the commands that have not returned are listed with their age', async () => {
  let release
  window.__TAURI_INTERNALS__.invoke = () => new Promise((resolve) => { release = resolve })
  core.configureInvokeTiming({ report: () => {}, now: () => clockNow, limit: 500 })
  const call = core.invoke('get_vdd_status')
  clockNow += 2500
  assert.deepEqual(core.invokePending(), [{ name: 'get_vdd_status', ms: 2500 }])
  release('x'); await call
  assert.deepEqual(core.invokePending(), [])
})

test('a command that fails is reported when slow and the failure reaches the caller', async () => {
  const lines = []
  window.__TAURI_INTERNALS__.invoke = () => { clockNow += 800; return Promise.reject(new Error('refused')) }
  core.configureInvokeTiming({ report: (...line) => lines.push(line), now: () => clockNow, limit: 500 })
  await assert.rejects(core.invoke('probe_that_fails'), /refused/)
  assert.deepEqual(lines.map(([kind, name]) => [kind, name]), [['invoke', 'probe_that_fails']])
})

test('the rest of the package is passed on, and the runtime object is left alone', () => {
  assert.equal(typeof core.convertFileSrc, 'function')
  assert.equal(typeof core.Channel, 'function')
  const descriptor = Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__, 'invoke')
  assert.equal(descriptor.writable, true) // our own stub: never replaced by the timing, only called
})

test('the cost of the timing around one call', async () => {
  window.__TAURI_INTERNALS__.invoke = () => Promise.resolve(1)
  core.configureInvokeTiming({ report: () => {} })
  const rounds = 200000
  for (let i = 0; i < 20000; i += 1) { await core.invoke('warm'); await window.__TAURI_INTERNALS__.invoke('warm') }
  const time = async (fn) => { const t = performance.now(); for (let i = 0; i < rounds; i += 1) await fn(); return (performance.now() - t) * 1e6 / rounds }
  const bare = await time(() => window.__TAURI_INTERNALS__.invoke('x'))
  const timed = await time(() => core.invoke('x'))
  console.log(`# per call: bare ${bare.toFixed(0)} ns, timed ${timed.toFixed(0)} ns, overhead ${(timed - bare).toFixed(0)} ns`)
  assert.ok(timed - bare < 20000, 'the timing costs less than 20 microseconds a call')
})
