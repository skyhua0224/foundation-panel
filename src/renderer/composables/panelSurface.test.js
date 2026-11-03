import test from 'node:test'
import assert from 'node:assert/strict'
import { createSurfaceReporter } from './panelSurface.js'

const manualClock = () => {
  const queue = []
  return {
    schedule(fn) {
      queue.push(fn)
      return () => queue.splice(queue.indexOf(fn), 1)
    },
    tick() { queue.splice(0).forEach((fn) => fn()) },
    pending: () => queue.length,
  }
}

test('a burst of root changes reports the settled surface once', () => {
  const clock = manualClock()
  let rgb = [45, 38, 40]
  const sent = []
  const reporter = createSurfaceReporter({ read: () => rgb, send: (c) => sent.push(c), schedule: clock.schedule })
  reporter.request(); reporter.request(); reporter.request()
  assert.equal(clock.pending(), 1)
  rgb = [20, 18, 19]
  clock.tick()
  assert.deepEqual(sent, [[20, 18, 19]])
})

test('the same colour is not sent again; a new one is', () => {
  const clock = manualClock()
  let rgb = [45, 38, 40]
  const sent = []
  const reporter = createSurfaceReporter({ read: () => rgb, send: (c) => sent.push(c), schedule: clock.schedule })
  reporter.request(); clock.tick()
  reporter.request(); clock.tick()
  rgb = [240, 248, 255]
  reporter.request(); clock.tick()
  assert.deepEqual(sent, [[45, 38, 40], [240, 248, 255]])
})

test('an unreadable surface sends nothing', () => {
  const clock = manualClock()
  const sent = []
  const reporter = createSurfaceReporter({ read: () => null, send: (c) => sent.push(c), schedule: clock.schedule })
  reporter.request(); clock.tick()
  assert.deepEqual(sent, [])
})
