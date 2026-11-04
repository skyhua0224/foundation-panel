import assert from 'node:assert/strict'
import test from 'node:test'
import { createReportLimiter, faultReport, isLongPoll, normalizeFrameReport, revealReport, REVEAL_SLOW_MS } from './frameReport.js'

const good = (over = {}) => ({ type: 'panel-frame-report', kind: 'frame', name: 'long-animation-frame', ms: 842.6, detail: 'script x', page: '/pin', ...over })

test('a report of the frame becomes one line with the page, the name and the time', () => {
  assert.deepEqual(normalizeFrameReport(good()), { kind: 'frame-frame', name: '/pin long-animation-frame', ms: 843, detail: 'script x' })
  assert.equal(normalizeFrameReport(good({ kind: 'error', name: 'TypeError: x is not a function', ms: 0 })).kind, 'frame-error')
})
test('what is not a report is nothing: other messages, unknown kinds, no name, not an object', () => {
  for (const bad of [null, 'x', 5, {}, good({ type: 'other' }), good({ kind: 'eval' }), good({ name: '' }), good({ name: undefined })]) assert.equal(normalizeFrameReport(bad), null)
})
test('the frame is another origin: lines are clipped, queries and hashes are cut, times are numbers in range', () => {
  const r = normalizeFrameReport(good({ name: 'x'.repeat(5000), detail: 'y\nz'.repeat(500), page: '/pin?pin=1234#filter=Pixel', ms: '1e99' }))
  assert.ok(r.name.length <= 80 + 1 + 160); assert.ok(r.detail.length <= 300); assert.ok(!/[\r\n\t]/.test(r.detail))
  assert.ok(r.name.startsWith('/pin x')); assert.ok(!/1234|Pixel/.test(r.name))
  assert.equal(normalizeFrameReport(good({ ms: 'abc' })).ms, 0)
  assert.equal(normalizeFrameReport(good({ ms: 1e99 })).ms, 3_600_000)
  assert.ok(!normalizeFrameReport(good({ detail: 'GET https://127.0.0.1:48081/api/clients/list?token=SECRET failed' })).detail.includes('SECRET'))
})
test('a page that fails in a loop writes at most the limit in a minute, then again', () => {
  let t = 1000
  const limiter = createReportLimiter({ perMinute: 3, now: () => t })
  assert.deepEqual([1, 2, 3, 4, 5].map(() => limiter.allow()), [true, true, true, false, false])
  t += 61_000
  assert.equal(limiter.allow(), true)
})
test('the pairing long poll is not a slow call, any other call is', () => {
  assert.equal(isLongPoll('/api/pairing/pending?wait=25&since=7'), true)
  assert.equal(isLongPoll('http://127.0.0.1:48081/api/pairing/pending'), true)
  assert.equal(isLongPoll('/api/foo?wait=30'), true)
  assert.equal(isLongPoll('/api/clients/list'), false)
  assert.equal(isLongPoll(undefined), false)
})
test('a quick reveal writes nothing; a slow one and a forced one write the page and how it was shown', () => {
  assert.equal(revealReport({ page: 'web-clients', ms: REVEAL_SLOW_MS - 1, by: 'painted' }), null)
  assert.deepEqual(revealReport({ page: 'web-clients', ms: 900, by: 'painted', fcp: 640 }), { kind: 'reveal', name: 'web-clients', ms: 900, detail: 'shown by painted, first contentful paint at 640 ms in the page' })
  const forced = revealReport({ page: '/pin', ms: 6000, by: 'ceiling' })
  assert.equal(forced.kind, 'reveal'); assert.match(forced.detail, /never said it had painted/)
  assert.equal(revealReport({ page: '/pin', ms: 10, by: 'ceiling' }).ms, 10, 'a forced reveal is written however short')
})
test('an error of the panel is a line with its message and where it was', () => {
  const error = new TypeError('x is not a function'); error.stack = 'TypeError: x is not a function\n    at a (app.js:1:2)\n    at b (app.js:3:4)'
  const r = faultReport({ error, filename: 'http://tauri.localhost/assets/all-abc.js?v=1', lineno: 10, colno: 5 })
  assert.equal(r.kind, 'error'); assert.equal(r.name, 'x is not a function'.replace(/^/, 'x is not a function'.slice(0, 0))); assert.match(r.detail, /^all-abc\.js:10:5 /); assert.match(r.detail, /at a \(app\.js:1:2\) \| at b/)
  assert.equal(faultReport({ reason: 'plain text' }).name, 'plain text')
  assert.equal(faultReport({}).name, 'error')
})

test('the long poll is recognised whatever the call is made with: a string, a URL, a Request', async () => {
  const { trackCalls } = await import('./slowCalls.js')
  let t = 0
  const lines = []
  const { wrapped } = trackCalls(async () => { t += 25000 }, { report: (...l) => lines.push(l), now: () => t, kind: 'fetch', limit: 500, ignore: (args) => isLongPoll(String(args[0]?.href ?? args[0]?.url ?? args[0])) })
  await wrapped('http://127.0.0.1:48081/api/pairing/pending?wait=25')
  await wrapped(new URL('http://127.0.0.1:48081/api/pairing/pending?wait=25&since=3'))
  await wrapped({ url: 'http://127.0.0.1:48081/api/pairing/pending' })
  assert.deepEqual(lines, [])
  await wrapped(new URL('http://127.0.0.1:48081/api/clients/list'))
  assert.equal(lines.length, 1)
})
