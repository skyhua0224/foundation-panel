import assert from 'node:assert/strict'
import test from 'node:test'

// A document with a settable `hidden` and the listener list a real one has; fake timers count the ticks.
function setup() {
  const listeners = new Set()
  const intervals = new Map()
  let id = 0
  globalThis.document = {
    hidden: false,
    addEventListener: (type, fn) => type === 'visibilitychange' && listeners.add(fn),
    removeEventListener: (type, fn) => listeners.delete(fn),
  }
  globalThis.setInterval = (fn) => { intervals.set(++id, fn); return id }
  globalThis.clearInterval = (handle) => { intervals.delete(handle) }
  const tick = () => [...intervals.values()].forEach((fn) => fn())
  const setHidden = (value) => { document.hidden = value; [...listeners].forEach((fn) => fn()) }
  return { tick, setHidden, listeners, intervals }
}

test('a visible interval ticks while the page is shown and stays quiet while it is hidden', async () => {
  const world = setup()
  const { setVisibleInterval, clearVisibleInterval } = await import(`./pageVisibility.js?${Math.random()}`)
  let ticks = 0
  const token = setVisibleInterval(() => { ticks += 1 }, 1000)
  world.tick(); world.tick()
  assert.equal(ticks, 2)
  world.setHidden(true)
  assert.equal(world.intervals.size, 0, 'no timer runs while hidden')
  world.tick()
  assert.equal(ticks, 2)
  world.setHidden(false)
  assert.equal(ticks, 2, 'without resume nothing runs at once')
  world.tick()
  assert.equal(ticks, 3)
  clearVisibleInterval(token)
  assert.equal(world.intervals.size, 0)
  assert.equal(world.listeners.size, 0, 'the listener is removed with the timer')
})

test('resume runs the callback once when the page is shown again', async () => {
  const world = setup()
  const { setVisibleInterval, clearVisibleInterval } = await import(`./pageVisibility.js?${Math.random()}`)
  let ticks = 0
  const token = setVisibleInterval(() => { ticks += 1 }, 1000, { resume: true })
  world.setHidden(true)
  world.setHidden(false)
  assert.equal(ticks, 1)
  // a second event for the same visible state does not run it again
  world.setHidden(false)
  assert.equal(ticks, 1)
  clearVisibleInterval(token)
})

test('a page that starts hidden starts its timer when it is shown', async () => {
  const world = setup()
  document.hidden = true
  const { setVisibleInterval, clearVisibleInterval, onPageShown } = await import(`./pageVisibility.js?${Math.random()}`)
  let ticks = 0
  let shown = 0
  const token = setVisibleInterval(() => { ticks += 1 }, 1000)
  const off = onPageShown(() => { shown += 1 })
  assert.equal(world.intervals.size, 0)
  world.setHidden(false)
  assert.equal(world.intervals.size, 1)
  assert.equal(shown, 1)
  world.setHidden(true)
  assert.equal(shown, 1, 'hiding is not showing')
  off()
  clearVisibleInterval(token)
  assert.equal(ticks, 0)
})
