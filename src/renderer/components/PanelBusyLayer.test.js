import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'

test('a close request cannot end the busy layer while the work runs', () => {
  const layer = readFileSync(new URL('./PanelBusyLayer.vue', import.meta.url), 'utf8')
  assert.match(layer, /closedby="none" @cancel\.prevent @close="keepOpen"/)
  assert.match(layer, /controllerBusyText\.value && el && !el\.open\) el\.showModal\(\)/)
})
