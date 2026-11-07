import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')

test('the display page shows the two lists as read-only rows and edits them in the shared popup', () => {
  const page = read('../components/VddSettings.vue')
  assert.match(page, /<MModeListRow kind="resolution" :values="resolutionList"/)
  assert.match(page, /<MModeListRow kind="rate" :values="rateList"/)
  assert.match(page, /<MModeListDialog v-model="modesOpen" :resolutions="resolutionList" :rates="rateList"/)
  assert.match(page, /const saveModes = \(\{ resolutions, rates \}\) => \{ resolutionList\.value = resolutions; rateList\.value = rates \}/)
  assert.doesNotMatch(page, /VddModeList|useEditableOptionField|sortedResolutionOptions|sortedRefreshRateOptions/)
})

test('the lists keep the persons order into the payload and back', () => {
  const page = read('../components/VddSettings.vue')
  assert.match(page, /g_refresh_rate: \[\.\.\.rateList\.value\]/)
  assert.match(page, /resolution: resolutionList\.value\.map/)
  assert.match(page, /setResolutionOptions = \(values = \[\]\) => \{ resolutionList\.value = cleanList\('resolution', values\) \}/)
})

test('the product is named 杂鱼虚拟显示器 in Chinese and ZakoDisplay in English, never the short Chinese form or 基地显示器', () => {
  const zh = read('../desktop/i18n/zh.js')
  const en = read('../desktop/i18n/en.js')
  assert.doesNotMatch(zh, /基地显示器|杂鱼显示器/)
  assert.match(zh, /title: '杂鱼虚拟显示器设置'/)
  assert.match(en, /title: 'ZakoDisplay Settings'/)
  assert.match(zh, /modeListNote: '/)
  assert.match(en, /modeListNote: '/)
  const tray = readFileSync(new URL('../../../src-tauri/src/tray.rs', import.meta.url), 'utf8')
  assert.match(tray, /vdd_settings: "杂鱼虚拟显示器设置"/)
  assert.match(tray, /vdd_settings: "ZakoDisplay Settings"/)
  assert.doesNotMatch(tray, /基地显示器|杂鱼显示器/)
  const doctor = readFileSync(new URL('../../../src-tauri/src/doctor_local.rs', import.meta.url), 'utf8')
  assert.match(doctor, /tr\(lang, "ZakoDisplay", "杂鱼虚拟显示器"\)/)
})
