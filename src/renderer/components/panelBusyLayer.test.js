import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const layer = readFileSync(new URL('./PanelBusyLayer.vue', import.meta.url), 'utf8')
const shell = readFileSync(new URL('../sunshine-frame.html', import.meta.url), 'utf8')

// sunshine-frame.html resets the margin of every element, which removes the
// browser's own centring of a modal dialog. Without a rule of its own the
// download card sat at the window's top-left corner.
test('the busy layer restates its position after the shell margin reset', () => {
  assert.match(shell, /\* \{[^}]*margin: 0;/)
  const block = layer.match(/\.panel-busy \{([^}]*)\}/)
  assert.ok(block, 'the busy layer has its own rule')
  assert.match(block[1], /position: fixed;/)
  assert.match(block[1], /inset: 0;/)
  assert.match(block[1], /margin: auto;/)
  assert.match(block[1], /width: fit-content;/)
  assert.match(block[1], /height: fit-content;/)
})

test('the busy card is centred in the panel window, not on a monitor of its own', () => {
  assert.doesNotMatch(layer, /screenX|screenY|\.position\(|set_position/)
})
