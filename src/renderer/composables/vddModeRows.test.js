import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')

test('the page of the virtual display has no list of its own: the modes are the one list on 分辨率与刷新率, and the page links there', () => {
  const page = read('../components/VddSettings.vue')
  assert.doesNotMatch(page, /<MModeListRow|<MModeListDialog|const saveModes|const modesOpen/)
  assert.match(page, /emit\('webui-navigate', '\/config#tab=display&sub=modes'\)/)
  // the lists are still read from and written back to the driver's file with the page's own save (they are not lost), in the order the person gave
  assert.match(page, /g_refresh_rate: \[\.\.\.rateList\.value\]/)
  assert.match(page, /resolution: resolutionList\.value\.map/)
  assert.match(page, /setResolutionOptions = \(values = \[\]\) => \{ resolutionList\.value = cleanList\('resolution', values\) \}/)
  assert.doesNotMatch(page, /VddModeList|useEditableOptionField|sortedResolutionOptions|sortedRefreshRateOptions/)
})

test('the page of the virtual display is the device card, keep, headless and the way to modes, calibration and scaling; nothing of those is drawn twice', () => {
  const page = read('../components/VddSettings.vue')
  assert.match(page, /<MVddDevice/)
  for (const gone of ['DpiScaleSection', "activeSection === 'calibration'", 'launchPhysicalHdrCalibration()', 'MChoiceCards v-model="prepModel"']) assert.ok(!page.includes(gone), gone)
  for (const link of ['sub=modes', 'sub=hdr', 'sub=output']) assert.ok(page.includes(link), link)
})

test('the product is named 基地虚拟显示器 in Chinese and Foundation Virtual Display in English, never the short Chinese form or 基地显示器', () => {
  const zh = read('../desktop/i18n/zh.js')
  const en = read('../desktop/i18n/en.js')
  assert.doesNotMatch(zh, /基地显示器|杂鱼显示器/)
  assert.match(zh, /title: '基地虚拟显示器设置'/)
  assert.match(en, /title: 'Foundation Virtual Display Settings'/)
  const zhTW = read('../desktop/i18n/zh_TW.js')
  const enGB = read('../desktop/i18n/en_GB.js')
  const enUS = read('../desktop/i18n/en_US.js')
  assert.doesNotMatch(zhTW, /雜魚虛擬顯示器|ZakoDisplay/)
  assert.match(zhTW, /title: '基地虛擬顯示器設定'/)
  assert.doesNotMatch(enGB, /ZakoDisplay/)
  assert.doesNotMatch(enUS, /ZakoDisplay/)
  assert.match(enGB, /"title": "Foundation Virtual Display Settings"/)
  assert.match(enUS, /"title": "Foundation Virtual Display Settings"/)
  assert.match(zh, /modeListNote: '/)
  assert.match(en, /modeListNote: '/)
  const tray = readFileSync(new URL('../../../src-tauri/src/tray.rs', import.meta.url), 'utf8')
  assert.match(tray, /vdd_settings: "基地虚拟显示器设置"/)
  assert.match(tray, /vdd_settings: "Foundation Virtual Display Settings"/)
  assert.doesNotMatch(tray, /基地显示器|杂鱼|Zako/)
  const doctor = readFileSync(new URL('../../../src-tauri/src/doctor_local.rs', import.meta.url), 'utf8')
  assert.match(doctor, /tr\(lang, "Foundation Virtual Display", "基地虚拟显示器"\)/)
})

test('the picture enhancement page declares what its template uses (the display tree texts and the expander)', () => {
  const page = read('../components/HdrEnhancedManager.vue')
  assert.match(page, /const dt = computed\(\(\) => displayTreeTexts\(locale\.value\)\)/)
  assert.match(page, /const advancedOpen = ref\(false\)/)
  assert.match(page, /import \{ MExpander, displayTreeTexts,/)
})
