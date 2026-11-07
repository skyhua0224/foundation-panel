import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')

test('the display page shows the two lists as read-only rows and edits them in the shared popup', () => {
  const page = read('../components/VddSettings.vue')
  assert.match(page, /<MModeListRow kind="resolution" :values="resolutionList"/)
  assert.match(page, /<MModeListRow kind="rate" :values="rateList"/)
  assert.match(page, /<MModeListDialog v-model="modesOpen" :resolutions="resolutionList" :rates="rateList"/)
  assert.match(page, /:saver="saveModes"/)
  assert.match(page, /const saveModes = async \(\{ resolutions, rates \}\) =>/)
  // the popup saves for real: read the file again, put in only the two lists, write (the Rust save writes vdd_settings.xml and asks the driver to reload)
  assert.match(page, /vdd\.loadSettings\(\)[\s\S]*data\.global = [\s\S]*g_refresh_rate: rates\.slice\(\)[\s\S]*resolution: nodes[\s\S]*vdd\.saveSettings\(data\)/)
  // success updates the saved snapshot of those two lists only, a failure returns the error and nothing else changes
  assert.match(page, /lastSavedSnapshot\.value = JSON\.stringify\(saved\)/)
  assert.match(page, /if \(!result\?\.success\) return t\.value\.vddSettings\.saveFailed/)
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
