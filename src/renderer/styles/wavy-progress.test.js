// Work of some length shows the M3 Expressive wavy progress (foundation-ui MProgress): the VDD driver install, the ViGEm and virtual mouse driver installs and removals.
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { en } from '../desktop/i18n/en.js'
import { zh } from '../desktop/i18n/zh.js'

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), 'utf8')

test('the VDD driver install shows the wave while it runs', () => {
  assert.match(read('components/VddSettings.vue'), /<MProgress v-if="isInstallingDriver" class="vdd-installing" :label="t\.vddSettings\.installingDriver" \/>/)
  for (const l of [en, zh]) assert.ok(l.vddSettings.installingDriver)
})

test('a driver install or removal in 输入驱动 shows the wave with the driver name', () => {
  const vue = read('components/controllersHub/InputDriversPanel.vue')
  assert.match(vue, /<MProgress v-for="d in \['vigem', 'vmouse'\]\.filter\(\(k\) => ops\[k\]\)"/)
  for (const l of [en, zh]) { assert.match(l.controllersHub.peripherals.installingDriver, /\{name\}/); assert.match(l.controllersHub.peripherals.uninstallingDriver, /\{name\}/) }
})
