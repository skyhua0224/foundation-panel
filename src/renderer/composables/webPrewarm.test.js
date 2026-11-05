import test from 'node:test'
import assert from 'node:assert/strict'
import { PREWARM_PATHS, startWebPrewarm } from './webPrewarm.js'

const fastSleep = () => new Promise((r) => setImmediate(r))

test('loads the pages one after the other, then ends once', async () => {
  const loaded = []
  let ended = 0
  startWebPrewarm({ base: 'http://127.0.0.1:48082', load: async (url) => { loaded.push(url) }, visible: () => true, idleFor: () => 5000, done: () => { ended += 1 }, sleep: fastSleep })
  for (let i = 0; i < 50; i++) await fastSleep()
  assert.deepEqual(loaded, PREWARM_PATHS.map((p) => `http://127.0.0.1:48082${p}`))
  assert.equal(ended, 1)
})

test('waits while the window is hidden or in use', async () => {
  const loaded = []
  let visible = false, idle = 0
  startWebPrewarm({ base: 'b', paths: ['/a'], load: async (url) => { loaded.push(url) }, visible: () => visible, idleFor: () => idle, done: () => {}, sleep: fastSleep })
  for (let i = 0; i < 20; i++) await fastSleep()
  assert.deepEqual(loaded, [])
  visible = true
  for (let i = 0; i < 20; i++) await fastSleep()
  assert.deepEqual(loaded, [], 'the person used the window a moment ago')
  idle = 3000
  for (let i = 0; i < 20; i++) await fastSleep()
  assert.deepEqual(loaded, ['b/a'])
})

test('stop ends it without loading further pages; a failing page does not end it', async () => {
  const loaded = []
  let ended = 0
  const stop = startWebPrewarm({ base: '', paths: ['/a', '/b', '/c'], load: async (url) => { loaded.push(url); if (url === '/a') throw new Error('x'); if (url === '/b') stop() }, visible: () => true, idleFor: () => 9999, done: () => { ended += 1 }, sleep: fastSleep })
  for (let i = 0; i < 50; i++) await fastSleep()
  assert.deepEqual(loaded, ['/a', '/b'])
  assert.equal(ended, 1)
})
