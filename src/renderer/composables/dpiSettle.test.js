import test from 'node:test'
import assert from 'node:assert/strict'
import { createSettleJudge, followDpiCover } from './dpiSettle.js'

test('the layout has to match for the stable time, and a wrong layout starts the time again', () => {
  const judge = createSettleJudge({ width: 960, height: 600, stableMs: 100 })
  assert.equal(judge.sample(640, 400, 1.5, 0), false) // matches: the time starts
  assert.equal(judge.sample(640, 400, 1.5, 60), false)
  assert.equal(judge.sample(854, 534, 1.5, 80), false) // the layout for the old size at the new scale
  assert.equal(judge.sample(640, 400, 1.5, 100), false) // starts again
  assert.equal(judge.sample(640, 400, 1.5, 150), false)
  assert.equal(judge.sample(640, 400, 1.5, 200), true)
})

test('the rounding of a fractional scale is allowed, a different size is not', () => {
  const judge = createSettleJudge({ width: 1601, height: 1001, stableMs: 0 })
  assert.equal(judge.sample(1281, 801, 1.25, 0), true) // 1601.25 x 1001.25
  assert.equal(createSettleJudge({ width: 1601, height: 1001, stableMs: 0 }).sample(1300, 801, 1.25, 0), false)
})

function setup() {
  let handler
  const calls = []
  const frames = []
  let t = 0
  const win = { innerWidth: 854, innerHeight: 534, devicePixelRatio: 1.5 }
  followDpiCover({
    listen: (name, fn) => { handler = fn; return Promise.resolve(() => {}) },
    invoke: (name, args) => { calls.push([name, args]); return Promise.resolve() },
    raf: (fn) => frames.push(fn),
    now: () => t,
    win,
  })
  const step = (ms) => { t += ms; frames.splice(0).forEach((fn) => fn()) }
  return { win, calls, step, frames, send: (payload) => handler({ payload }) }
}

test('the page reports once its layout has matched the window for a while', () => {
  const { win, calls, step, send } = setup()
  send({ width: 960, height: 600, token: 7 })
  step(16)
  assert.deepEqual(calls, [])
  win.innerWidth = 640; win.innerHeight = 400
  for (let i = 0; i < 12; i += 1) step(16)
  assert.deepEqual(calls, [['panel_dpi_settled', { token: 7 }]])
})

test('an older cover is dropped when a newer one arrives, and the page stops asking after the give-up time', () => {
  const { win, calls, step, frames, send } = setup()
  send({ width: 960, height: 600, token: 1 })
  send({ width: 960, height: 600, token: 2 })
  win.innerWidth = 640; win.innerHeight = 400
  for (let i = 0; i < 12; i += 1) step(16)
  assert.deepEqual(calls.map(([, args]) => args.token), [2])
  win.innerWidth = 854; win.innerHeight = 534
  send({ width: 960, height: 600, token: 3 })
  for (let i = 0; i < 60; i += 1) step(16)
  assert.equal(calls.length, 1)
  assert.equal(frames.length, 0)
})

test('with stable frames the layout has to match for that many frames in a row, however long they take', () => {
  const judge = createSettleJudge({ width: 960, height: 600, stableMs: 0, stableFrames: 2 })
  assert.equal(judge.sample(640, 400, 1.5, 0), false)
  assert.equal(judge.sample(854, 534, 1.5, 16), false) // the wrong layout resets the count
  assert.equal(judge.sample(640, 400, 1.5, 32), false)
  assert.equal(judge.sample(640, 400, 1.5, 48), true)
})
