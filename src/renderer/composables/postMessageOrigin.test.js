import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { frameOrigin, postToFrame } from './frameOrigin.js'

// Field report (Firestore, 2026.1104.121813 and 2026.1104.155142): "SyntaxError: Failed to execute 'postMessage' on 'Window': Invalid target origin 'null'" on the home page.
// It was PanelPending.vue: `postMessage(message, originOf(el))` with `new URL(el.src).origin`, which is 'null' for a frame on about:blank (a frame that sleeps behind a page of the panel), every 600 ms (fc6135b, after
// those two releases). The rule for every message to a frame: an http(s) origin, or nothing is sent. '*' (no origin at all) is the other form that cannot throw.
const src = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
function* walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (['node_modules', '_harness'].includes(e.name)) continue
    const full = path.join(dir, e.name)
    if (e.isDirectory()) yield* walk(full)
    else if (/\.(vue|js)$/.test(e.name) && !/\.test\.js$/.test(e.name)) yield full
  }
}

test('a frame that sleeps on about:blank is told nothing, and nothing throws', () => {
  const sent = []
  const sleeping = { src: 'about:blank', contentWindow: { postMessage: (m, o) => { if (o === 'null') throw new SyntaxError("Invalid target origin 'null'"); sent.push(o) } } }
  assert.equal(frameOrigin(sleeping, 'http://tauri.localhost/'), '')
  assert.equal(postToFrame(sleeping, { type: 'x' }, 'http://tauri.localhost/'), false)
  const live = { src: 'http://127.0.0.1:48081/config', contentWindow: sleeping.contentWindow }
  assert.equal(postToFrame(live, { type: 'x' }, 'http://tauri.localhost/'), true)
  assert.deepEqual(sent, ['http://127.0.0.1:48081'])
})

test('every postMessage with a target origin in the panel goes through a guard for an http(s) origin', () => {
  const unguarded = []
  for (const file of walk(src)) {
    const text = fs.readFileSync(file, 'utf8')
    for (const m of text.matchAll(/(\w+)?\??\.postMessage\(/g)) {
      if (['channel', 'speechBroadcast', 'worker'].includes(m[1])) continue // a BroadcastChannel or a worker: no target origin
      const call = text.slice(m.index, m.index + 360)
      if (/'\*'\s*\)/.test(call.slice(0, call.indexOf('postMessage(', 20) > 0 ? call.indexOf('postMessage(', 20) : call.length))) continue // no origin: cannot throw
      const around = text.slice(Math.max(0, m.index - 700), m.index + 120)
      const guarded = /https\?/.test(around) || /postToFrame|frameOrigin\(/.test(around) || /origin !== 'null'/.test(around)
      if (!guarded) unguarded.push(`${path.relative(src, file)}: ${call.slice(0, 80).replace(/\s+/g, ' ')}`)
    }
  }
  assert.deepEqual(unguarded, [])
})

test('the pending bridge does not send with the raw origin of a frame address', () => {
  const pending = fs.readFileSync(path.join(src, 'components/PanelPending.vue'), 'utf8')
  assert.doesNotMatch(pending, /postMessage\([^)]*originOf\(/, 'the message of the field report')
  assert.match(pending, /postToFrame\(/)
})
