import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const src = readFileSync(new URL('./PanelWebHost.vue', import.meta.url), 'utf8')

test('the first cover of a cold start shows at once; the covers of page changes wait 300 ms so a fast switch does not flash', () => {
  assert.match(src, /<MPageLoading :delay="0" \/>/)
  assert.match(src, /'is-first': first/)
  assert.match(src, /const VEIL_MS = 300/)
  assert.match(src, /\.panel-web\.is-veil \.panel-web__cover \{ opacity: 1; \}/)
  assert.doesNotMatch(src, /transition-delay/, 'a CSS delay is dropped where animations are off: the veil is a timer')
  assert.match(src, /first\.value = false/, 'the first cover ends with the first reveal')
})

test('the cover stays mounted and only its opacity toggles, so the rows\' sweep does not restart with .is-covered', () => {
  assert.doesNotMatch(src, /<div class="panel-web__cover"[^>]*v-(if|show)/)
  assert.doesNotMatch(src, /\.panel-web__cover[^{]*\{[^}]*display:\s*none/)
})

test('the Back bubble and the loading cover take the inset from the host\'s own width: the host is the container, so a framed first page has one left edge on a phone', () => {
  assert.match(src, /\.panel-web \{[^}]*container-type: inline-size;/)
  assert.match(src, /@container \(max-width: 700px\) \{ \.panel-web__bar, \.panel-web__cover \{ --m-gutter: 16px; \} \}/)
  assert.match(src, /@container \(max-width: 560px\) \{ \.panel-web__bar, \.panel-web__cover \{ --m-gutter: 12px; \} \}/)
  assert.doesNotMatch(src, /@container \([^)]*\) \{ \.panel-web \{/, 'a query on the host itself looks at an ancestor that is not there')
})
test('the bar of the Back bubble has air under it, so the scrolled page is clipped below the bubble and not at its edge', () => {
  // the column of the pages: the inset under the maximum, centred beyond it (pageTitlePosition.test.js), and the 8 px under the bubble stay
  assert.match(src, /\.panel-web__bar \{ padding: 8px max\(var\(--m-gutter\), calc\(\(100% - 960px\) \/ 2\)\); flex: none; \}/)
})
