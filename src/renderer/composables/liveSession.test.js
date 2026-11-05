import test from 'node:test'
import assert from 'node:assert/strict'
import { formatBitrate, validBitrate, liveSessions, perfSessionOf, budgetUsage, perfStatus, pipelineSegments, formatUptime, BITRATE } from './liveSession.js'

test('bitrates read as the tool window wrote them and stay in its range', () => {
  assert.equal(formatBitrate(20000), '20 Mbps')
  assert.equal(formatBitrate(800), '800 Kbps')
  assert.ok(validBitrate(BITRATE.min) && validBitrate(BITRATE.max))
  assert.ok(!validBitrate(999) && !validBitrate(800001) && !validBitrate(NaN))
})

test('only sessions that stream take a bitrate', () => {
  assert.deepEqual(liveSessions([{ client_name: 'a', state: 'RUNNING' }, { client_name: 'b', state: 'STOPPING' }, { client_name: 'c', state: 'STOPPED' }]).map((s) => s.client_name), ['a'])
  assert.deepEqual(liveSessions(null), [])
})

test('the performance snapshot shows the latest active session with the thresholds of the tool window', () => {
  const snap = { latest_session_id: 2, sessions: [{ session_id: 1, active: true }, { session_id: 2, active: true, fps: 60 }, { session_id: 3, active: false }] }
  assert.equal(perfSessionOf(snap).session_id, 2)
  assert.equal(perfSessionOf({ sessions: [{ active: false }] }), null)
  const s = (avg, p95) => ({ fps: 60, host_latency: { samples: 10, avg_ms: avg, p95_ms: p95 } })
  assert.equal(Math.round(budgetUsage(s(5, 8.3333))), 50)
  assert.equal(perfStatus(null), 'idle')
  assert.equal(perfStatus(s(1, 1), true), 'error')
  assert.equal(perfStatus({ fps: 60, host_latency: {} }), 'waiting')
  assert.equal(perfStatus(s(5, 8)), 'good')
  assert.equal(perfStatus(s(20, 20)), 'warning')
  assert.equal(perfStatus(s(40, 40)), 'slow')
})

test('pipeline stages with samples come in pipeline order', () => {
  const session = { pipeline: { encode: { samples: 3, p95_ms: 2 }, capture_to_convert: { samples: 1 }, convert: { samples: 0 } } }
  assert.deepEqual(pipelineSegments(session).map((x) => x.key), ['capture_to_convert', 'encode'])
  assert.equal(formatUptime(125000), '2m 5s')
  assert.equal(formatUptime(0), '--')
})
