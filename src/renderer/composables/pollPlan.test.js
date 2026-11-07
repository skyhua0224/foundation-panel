import { test } from 'node:test'
import assert from 'node:assert/strict'
import { POLL_BASE_MS, POLL_BOOST_MS, BOOST_WINDOW_MS, pollDelay, boostUntilFor, hasStartingSession } from './pollPlan.js'

test('the poll is three seconds, one second inside a boost window, and the window only grows', () => {
  assert.equal(pollDelay(1000, 0), POLL_BASE_MS)
  const until = boostUntilFor(1000, 'returned')
  assert.equal(until, 1000 + BOOST_WINDOW_MS.returned)
  assert.equal(pollDelay(1000 + 5000, until), POLL_BOOST_MS)
  assert.equal(pollDelay(until, until), POLL_BASE_MS, 'at the end of the window the base poll is back')
  assert.equal(boostUntilFor(1000, 'starting', until), until, 'a shorter reason does not cut a running window')
  assert.equal(boostUntilFor(1000, 'unknown', 0), 1000)
})
test('a session that is not RUNNING yet is a stream that is starting', () => {
  assert.equal(hasStartingSession([{ state: 'RUNNING' }]), false)
  assert.equal(hasStartingSession([{ state: 'running' }]), false)
  assert.equal(hasStartingSession([{ state: 'STARTING' }]), true)
  assert.equal(hasStartingSession([{ state: 'RUNNING' }, { state: 'CONNECTING' }]), true)
  assert.equal(hasStartingSession([]), false)
  assert.equal(hasStartingSession(null), false)
  assert.equal(hasStartingSession([{}]), false)
})
test('usePanelHome polls on that plan, not on a fixed interval', async () => {
  const fs = await import('node:fs')
  const src = fs.readFileSync(new URL('./usePanelHome.js', import.meta.url), 'utf8')
  assert.match(src, /pollDelay\(Date\.now\(\), boostUntil\)/)
  assert.match(src, /visibilitychange/)
  assert.doesNotMatch(src, /setInterval\(fast, 3000\)/)
})

test('usePanelHome takes the measured bitrate from sessions[] or perf/current and keeps the budget apart (mock data: the real host does not send it yet)', async () => {
  const fs = await import('node:fs')
  const src = fs.readFileSync(new URL('./usePanelHome.js', import.meta.url), 'utf8')
  assert.match(src, /s\.measured_bitrate_kbps = psess\.measured_bitrate_kbps/)
  assert.match(src, /push\(state\.series\.bitrate, Number\.isFinite\(s\.measured_bitrate_kbps\) \? s\.measured_bitrate_kbps : s\.bitrate\)/)
})
