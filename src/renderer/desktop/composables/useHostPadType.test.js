import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { HOST_PAD_TYPE_POLL_MS } from './useHostPadType.js'

test('the host is asked only while a pad is connected, every few seconds, and a failed or empty answer falls back to the local pad', () => {
  const source = readFileSync(new URL('./useHostPadType.js', import.meta.url), 'utf8')
  assert.equal(HOST_PAD_TYPE_POLL_MS, 5000)
  assert.match(source, /watch\(gamepadConnected/)
  assert.match(source, /\/api\/runtime\/sessions/)
  assert.match(source, /catch \{[^}]*setHostPadType\(null\)/s)
  assert.match(source, /else \{ stop\(\); setHostPadType\(null\) \}/)
})
