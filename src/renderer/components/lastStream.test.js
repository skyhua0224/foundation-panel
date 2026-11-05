import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// The after-stream scenario of the smart overview runs 30 minutes from the end of the last stream (m3e homeSmart.js, ms since epoch): the signal must be the saved history's last session end, not a constant.
test('the smart overview is told when the last stream ended', () => {
  const src = fs.readFileSync(new URL('./PanelHome.vue', import.meta.url), 'utf8')
  assert.doesNotMatch(src, /lastStreamAt: 0\b/, 'a constant 0 means the after-stream scenario can never appear')
  assert.match(src, /lastStreamAt: \(void historyTick\.value, history\.last\(\)\?\.end \|\| 0\)/, 'the end of the last saved session, recomputed when a session ends')
})
