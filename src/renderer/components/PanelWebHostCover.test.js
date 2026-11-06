import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const src = readFileSync(new URL('./PanelWebHost.vue', import.meta.url), 'utf8')

test('the first cover of a cold start shows at once; the covers of page changes wait 140 ms so a fast switch does not flash', () => {
  assert.match(src, /<MPageLoading :delay="0" \/>/)
  assert.match(src, /'is-first': first/)
  assert.match(src, /\.panel-web\.is-covered:not\(\.is-first\) \.panel-web__cover \{ transition-delay: 140ms; \}/)
  assert.match(src, /first\.value = false/, 'the first cover ends with the first reveal')
})

test('the cover stays mounted and only its opacity toggles, so the rows\' sweep does not restart with .is-covered', () => {
  assert.doesNotMatch(src, /<div class="panel-web__cover"[^>]*v-(if|show)/)
  assert.doesNotMatch(src, /\.panel-web__cover[^{]*\{[^}]*display:\s*none/)
})
