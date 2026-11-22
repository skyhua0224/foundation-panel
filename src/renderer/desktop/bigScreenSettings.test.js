import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import { BITRATE_PRESETS_MBPS, bitrateChips, bitrateFromMbps, bitrateKbps, bitrateNudge, bitrateSliderMbps } from 'foundation-ui/settings'
import { isSliderElement, sliderKeyFor } from './composables/useFocusNav.js'

// Requirement for the big screen's 串流配置: the big-screen settings pages are the same
// sections and rows (MSection / MGroup / MItem, the shared bitrate control, MSeg, MSwitch) as the Web UI, the Panel and the phone, only larger (useDeskScale) and operable with a pad.
const dir = fileURLToPath(new URL('./', import.meta.url))
const read = (file) => readFileSync(join(dir, file), 'utf8')
const PAGES = ['views/StreamView.vue', 'views/SettingsView.vue', 'views/DevicesView.vue', 'views/ToolsView.vue']
const PARTS = ['components/DeskPage.vue', 'components/PanelLink.vue', 'components/PanelLinkSection.vue', ...['SettingsCard', 'SettingsRow', 'SettingsToggleRow', 'SettingsToggleSection', 'SettingsAboutCard', 'SettingsAppearanceCard'].map((n) => `components/settings/${n}.vue`)]

test('no hero card classes are left on the big-screen settings pages: the pages and their parts have no tile, hero, number or shape styles of their own', () => {
  const banned = /dsk-bit\b|dsk-bit__|dsk-codec|dsk-flag|dsk-tile|dsk-device|dsk-idle|dsk-pair\b|dsk-diag|dsk-link__(art|shape)|dsk-card__icon|dsk-stream__col|\bburst\b|squircle|cookie|clover|sunny|m-turn|<MHero|<MScallop|font-size: ?(128|96|72)px/
  for (const file of [...PAGES, ...PARTS]) assert.doesNotMatch(read(file), banned, file)
})

test('the pages are built of the shared components: MSection, MGroup, MItem; the settings pages hold no <section> of their own', () => {
  for (const file of PAGES) {
    const text = read(file)
    assert.match(text, /<DeskPage/, file)
    assert.match(text, /<MSection|<PanelLinkSection|<Settings[A-Za-z]+Card|<SettingsToggleSection/, file)
    assert.doesNotMatch(text.slice(0, text.indexOf('<script')), /<section\b/, `${file} has its own <section>`)
  }
  assert.match(read('components/settings/SettingsCard.vue'), /<MSection[^>]*>\s*<MGroup/)
  assert.match(read('components/PanelLinkSection.vue'), /<MSection[\s\S]*<MGroup[\s\S]*<PanelLink /)
  assert.match(read('components/PanelLink.vue'), /<MItem\s+action/)
  assert.match(read('components/DeskPage.vue'), /<MPage :title="title" :wide="wide"/)
})

test('串流 is one bitrate row (value, steps, slider, chips) from the shared bitrate module, then codec rows, HDR and launch as switch rows', () => {
  const view = read('views/StreamView.vue')
  assert.match(view, /<MItem class="dsk-bitrate" icon="monitor_heart" :title="t\.stream\.bitrateLimit"[^>]*stack>/)
  // the order the pad walks: value row (steps), slider, chips
  assert.ok(view.indexOf('icon="remove"') < view.indexOf('<MSlider') && view.indexOf('<MSlider') < view.indexOf('<MSeg :model-value="bitrateKbps"'))
  assert.match(view, /<MSlider :model-value="bitrateSliderProgress\(bitrateKbps\)" :min="0" :max="SLIDER_MAX_PROGRESS" :step="1" unit=""/)
  assert.equal((view.match(/<SettingsToggleRow/g) || []).length, 3, 'the codecs (a v-for), HDR, launch mode')
  assert.match(view, /<MSeg v-if="configData\[codec\.key\] > 0"/)
  for (const gone of ['bitrateNumber', 'bitrateUnit', 'heroText']) assert.ok(!view.includes(gone), gone)
  assert.match(view, /t\.stream\.hostAudio/)
})

test('the values round-trip as everywhere else: max_bitrate defaults to 0 (自动), whole Kbps strings go out, the chips and the slider read the same value', () => {
  const view = read('views/StreamView.vue')
  assert.match(view, /max_bitrate: '0'/)
  assert.match(view, /setCfg\('max_bitrate', String\(bitrateFromProgress\(v\)\)\)/)
  assert.match(view, /setCfg\('max_bitrate', String\(v\)\)/)
  assert.match(view, /setCfg\('hdr_prep', on \? 'automatic' : 'no_operation'\)/)
  assert.match(view, /saveSettingKey\(hostApi, key, value\)/)
  for (const raw of ['0', '20000', '80000', '65432', undefined]) {
    const kbps = bitrateKbps(raw)
    assert.equal(String(bitrateFromMbps(bitrateSliderMbps(kbps))), raw === '65432' ? '65000' : String(kbps), raw) // the slider writes whole Mbps; a chip writes the stored Kbps itself
  }
  assert.deepEqual(bitrateChips('65432', '自动', [20, 50, 100, 0]).map((c) => c.value), [0, 20000, 50000, 65432, 100000])
  assert.equal(bitrateNudge(0, 5, 200), 5000)
  assert.ok(BITRATE_PRESETS_MBPS.includes(0))
})

test('the settings page writes the same keys through the same state as before (the rows only changed their look)', () => {
  const view = read('views/SettingsView.vue')
  for (const key of ['autoStart', 'startMinimized', 'autoStartSunshine', 'autoYieldOnLaunch', 'gamepadCursorEnabled', 'oskAutoOpen', 'navSounds', 'notifications', 'connectionNotify', 'updateNotify']) assert.ok(view.includes(`key: '${key}'`), key)
  assert.match(view, /disabled: !settings\.value\.notifications/)
  assert.match(view, /@update-value="setBigScreenSetting"/)
  assert.match(view, /<SettingsActions @reset="resetSettings" @save="saveSettings"/)
})

test('a whole row is the target of a switch row, a press on the switch or on the choice below it is not flipped twice', () => {
  const row = read('components/settings/SettingsToggleRow.vue')
  assert.match(row, /closest\?\.\('\.m-switch, \.m-item__control'\)/)
  assert.match(row, /emit\('update:modelValue', !props\.modelValue\)/)
})

test('the pad moves a slider with left and right (one step, ten once held) and leaves it with up and down', () => {
  const nav = read('composables/useFocusNav.js')
  assert.match(nav, /if \(\(direction === 'left' \|\| direction === 'right'\) && isSliderElement\(current\)\) return nudgeSlider\(current, direction\)/)
  const el = (attrs, extra = {}) => ({ getAttribute: (k) => attrs[k] ?? null, tagName: 'SPAN', ...extra })
  assert.equal(isSliderElement(el({ role: 'slider' })), true)
  assert.equal(isSliderElement(el({ role: 'slider', 'aria-disabled': 'true' })), false)
  assert.equal(isSliderElement(el({}, { tagName: 'INPUT', type: 'range' })), true)
  assert.equal(isSliderElement(el({ role: 'radio' })), false)
  assert.equal(isSliderElement(null), false)
  assert.equal(sliderKeyFor('right', 100000), 'ArrowRight')
  assert.equal(sliderKeyFor('left', 100500), 'ArrowLeft')
  let key = ''
  for (let i = 0; i < 7; i++) key = sliderKeyFor('right', 200000 + i * 120)
  assert.equal(key, 'PageUp')
  assert.equal(sliderKeyFor('right', 300000), 'ArrowRight', 'a pause starts again with one step')
  for (let i = 0; i < 7; i++) key = sliderKeyFor('left', 400000 + i * 120)
  assert.equal(key, 'PageDown')
})

test('every control of the pages is something the focus system finds: buttons, switches, segmented buttons, slider handles, fields', () => {
  const nav = read('composables/useFocusNav.js')
  assert.match(nav, /'button:not\(\[disabled\]\)'/)
  assert.match(nav, /'\[tabindex\]:not\(\[tabindex="-1"\]\)'/)
  assert.match(nav, /'\[data-focusable\]:not\(\[disabled\]\)/)
})
