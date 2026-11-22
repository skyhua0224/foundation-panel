import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const layer = readFileSync(new URL('./PanelBusyLayer.vue', import.meta.url), 'utf8')
const shell = readFileSync(new URL('../sunshine-frame.html', import.meta.url), 'utf8')
const finalCssValue = (block, property) => block
  .replace(/\/\*[\s\S]*?\*\//g, '')
  .split(';')
  .map((declaration) => declaration.trim())
  .filter((declaration) => declaration.startsWith(property + ':'))
  .at(-1)
  ?.slice(property.length + 1)
  .trim()

// A close request (Esc, a frame close) cannot end the busy layer while the work runs: the dialog
// declares no close watcher (closedby="none"), prevents the cancel event, and when Chromium closes it
// anyway (close watchers need an activation, see keepOpen) it comes straight back until the busy text
// is gone.
test('a close request cannot end the busy layer while the work runs', () => {
  assert.match(layer, /closedby="none" @cancel\.prevent @close="keepOpen"/)
  assert.match(layer, /controllerBusyText\.value && el && !el\.open\) el\.showModal\(\)/)
})

// sunshine-frame.html resets the margin of every element, which removes the
// browser's own centring of a modal dialog. Without a rule of its own the
// download card sat at the window's top-left corner.
test('the busy layer restates its position after the shell margin reset', () => {
  assert.match(shell, /\* \{[^}]*margin: 0;/)
  const block = layer.match(/\.panel-busy \{([^}]*)\}/)
  assert.ok(block, 'the busy layer has its own rule')
  assert.equal((layer.match(/\.panel-busy\s*\{/g) || []).length, 1, 'the dialog has one positioning rule')
  assert.equal(finalCssValue(block[1], 'position'), 'fixed')
  assert.equal(finalCssValue(block[1], 'inset'), '0')
  assert.equal(finalCssValue(block[1], 'margin'), 'auto')
  assert.equal(finalCssValue(block[1], 'width'), 'fit-content')
  assert.equal(finalCssValue(block[1], 'height'), 'fit-content')
})

test('the busy card is centred in the panel window, not on a monitor of its own', () => {
  assert.doesNotMatch(layer, /screenX|screenY|\.position\(|set_position/)
})
