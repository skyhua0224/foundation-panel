import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { undeclaredIdentifiers } from 'foundation-ui/testing/scope-check'
import { openBigScreenWithNotice } from './openBigScreen.js'
import { en } from '../desktop/i18n/en.js'
import { zh } from '../desktop/i18n/zh.js'

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')

test('outside the panel (no Tauri) opening says why and does not throw', async () => {
  const said = []
  assert.equal(await openBigScreenWithNotice((m) => said.push(m), 'Could not open the big screen'), false)
  assert.equal(said.length, 1)
  assert.match(said[0], /^Could not open the big screen: /)
})

test('the words of the entry exist in both languages', () => {
  for (const t of [en, zh]) {
    for (const key of ['openBigScreen', 'openBigScreenHelp', 'openBigScreenFailed']) assert.ok(t.sidebar[key]?.length >= 4, key)
  }
  assert.equal(zh.sidebar.openBigScreen, '打开大屏')
  assert.equal(en.sidebar.openBigScreen, 'Open Big Screen')
})

test('every way in the panel opens it through the one route: overview tile, search action, Help row, Panel settings row', () => {
  const home = read('../components/PanelHome.vue')
  assert.match(home, /canOpenDesktop: true/)
  assert.match(home, /case 'open-desktop': void openBigScreenWithNotice/)
  assert.match(read('../components/SidebarMenu.vue'), /openDesktop: \(\) => openBigScreenWithNotice/)
  assert.match(read('../components/ControlPanelPage.vue'), /data-help-row="bigScreen" @click="openDesktop"/)
  assert.match(read('../components/m3e/PanelOptions.vue'), /data-panel-option="openBigScreen" @click="openDesktop"/)
  for (const file of ['PanelHome.vue', 'SidebarMenu.vue', 'ControlPanelPage.vue', 'm3e/PanelOptions.vue']) {
    assert.deepEqual(undeclaredIdentifiers(read(`../components/${file}`), { filename: file }), [], file)
  }
})
