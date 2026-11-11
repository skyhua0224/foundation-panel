import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { createProxyGate, FIRST_ROUND_DELAYS } from './proxyGate.js'

const instant = () => Promise.resolve()

test('one slow start is not a verdict: the check is tried again before anything is shown', async () => {
  const seen = []
  let calls = 0
  const gate = createProxyGate({ verify: async () => (++calls >= 3), onShown: () => seen.push('shown'), onCleared: () => seen.push('cleared'), sleep: instant })
  assert.equal(await gate.run(), true)
  assert.equal(calls, 3)
  assert.deepEqual(seen, [], 'the page never said anything')
})

test('when the whole first round fails the page says so, and the check goes on by itself until it answers', async () => {
  const seen = []
  let calls = 0
  const gate = createProxyGate({ verify: async () => (++calls > FIRST_ROUND_DELAYS.length + 2), onShown: () => seen.push('shown'), onCleared: () => seen.push('cleared'), sleep: instant })
  assert.equal(await gate.run(), true)
  assert.equal(calls, FIRST_ROUND_DELAYS.length + 3)
  assert.deepEqual(seen, ['shown', 'cleared'], 'shown once, cleared without a click')
})

test('a check that throws counts as a failed try, and the button tries at once', async () => {
  let calls = 0
  let release
  const sleep = (ms) => (ms >= 5000 ? new Promise((resolve) => { release = resolve }) : Promise.resolve())
  const gate = createProxyGate({ verify: async () => { calls += 1; if (calls < 6) throw new Error('blocked'); return true }, sleep, delays: [0, 0, 0, 0] })
  const done = gate.run()
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(gate.shown, true)
  assert.equal(calls, 4)
  gate.retryNow(); await new Promise((resolve) => setImmediate(resolve))
  gate.retryNow(); await new Promise((resolve) => setImmediate(resolve))
  assert.equal(await done, true)
  assert.equal(calls, 6)
})

test('stop ends the wait', async () => {
  const gate = createProxyGate({ verify: async () => false, sleep: (ms) => (ms >= 5000 ? new Promise(() => {}) : Promise.resolve()), delays: [0] })
  const done = gate.run()
  await new Promise((resolve) => setImmediate(resolve))
  gate.stop()
  assert.equal(await done, false)
})

test('the frame uses the gate: no single check that blanks the session, and the old failure page is gone', () => {
  const src = fs.readFileSync(new URL('../components/SunshineFrame.vue', import.meta.url), 'utf8')
  assert.match(src, /createProxyGate\(/)
  assert.doesNotMatch(src, /if \(!await verifyLocalProxy\(\)\)/, 'the one-shot check')
  assert.doesNotMatch(src, /loading-overlay|loading-container|loading-text|proxy-health-retry|PixelMplus|YouYuan|skew\(|morandi/, 'the old failure page and its styles')
  assert.doesNotMatch(src, /window\.location\.reload\(\)/, 'a retry does not reload the whole panel')
})

test('the status page is one design for the three causes, in the shell, and the pages under it are inert', () => {
  const shell = fs.readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
  assert.match(shell, /:status="offlineShown"/)
  assert.match(shell, /<MHostOffline v-if="offlineShown" :cause="offlineCause"/)
  assert.match(shell, /:can-restart="offlineCause !== 'proxy'"/, 'restarting Sunshine does not fix the proxy')
  assert.match(shell, /class="panel-web-keep" :inert="offlineShown/, 'the frame stays mounted and cannot be reached')
  assert.match(shell, /:disabled="hostUnusable"/, '连接设备 pairs through the host')
  const frame = fs.readFileSync(new URL('../components/SunshineFrame.vue', import.meta.url), 'utf8')
  assert.match(frame, /proxyBlock\.blocked = true/)
})

test('the pages of the panel itself still open while the host cannot be used', () => {
  const shell = fs.readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
  const set = shell.match(/const NEEDS_NO_HOST = new Set\(\[([^\]]+)\]\)/)[1]
  for (const route of ['PANEL_SETTINGS', 'HELP', 'ABOUT', 'ANNOUNCEMENTS']) assert.match(set, new RegExp(`ROUTES\\.${route}\\b`), `${route} works without the host`)
  assert.match(set, /\.\.\.APPEARANCE_ROUTES\b/, 'the four pages of 个性化 work without the host')
})
