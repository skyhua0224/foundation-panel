import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { frameOrigin, postToFrame } from './frameOrigin.js'

const BASE = 'http://tauri.localhost/sunshine-frame.html'
const frame = (src, sent = []) => ({ src, contentWindow: { postMessage: (m, o) => sent.push([m, o]) } })

test('a frame on an address has its origin, a frame that sleeps has none', () => {
  assert.equal(frameOrigin(frame('http://127.0.0.1:48081/pin'), BASE), 'http://127.0.0.1:48081')
  assert.equal(frameOrigin(frame('https://127.0.0.1:57990/'), BASE), 'https://127.0.0.1:57990')
  for (const sleeping of ['about:blank', '', 'data:text/html,x', 'blob:http://x/1', 'javascript:0']) assert.equal(frameOrigin(frame(sleeping), BASE), '', sleeping)
  assert.equal(frameOrigin(null, BASE), '')
  assert.equal(frameOrigin({ src: 'http://[bad' }, BASE), '')
})
test('nothing is sent to a sleeping frame (postMessage with the origin "null" throws)', () => {
  const sent = []
  assert.equal(postToFrame(frame('about:blank', sent), { type: 'x' }, BASE), false)
  assert.deepEqual(sent, [])
  assert.equal(postToFrame(frame('http://127.0.0.1:48081/', sent), { type: 'x' }, BASE), true)
  assert.deepEqual(sent, [[{ type: 'x' }, 'http://127.0.0.1:48081']])
  assert.equal(postToFrame({ src: 'http://127.0.0.1:48081/' }, { type: 'x' }, BASE), false, 'no window yet')
})
test('the pending bridge of the panel uses it for both its messages', () => {
  const src = fs.readFileSync(new URL('../components/PanelPending.vue', import.meta.url), 'utf8')
  assert.match(src, /postToFrame\(frame\(\), message\)/)
  assert.match(src, /postToFrame\(frame\(\), \{ type: 'foundation-pending', op: 'hello' \}\)/)
  assert.ok(!/postMessage\(message, originOf/.test(src))
})

// the frame component sends to an http(s) origin only: the place message and the replies (a custom scheme or about:blank has the origin 'null')
test('SunshineFrame sends only to an http(s) origin', async () => {
  const fs = await import('node:fs')
  const src = fs.readFileSync(new URL('../components/SunshineFrame.vue', import.meta.url), 'utf8')
  assert.match(src, /if \(!\/\^https\?:\\\/\\\/\/\.test\(origin\)\) return false/)
  assert.match(src, /return \/\^https\?:\$\/\.test\(url\.protocol\) \? url\.origin : ''/)
  assert.ok(!/\$\{url\.protocol\}\/\/\$\{url\.host\}/.test(src), 'no origin is built by hand')
})
