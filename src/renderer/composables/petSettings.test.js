import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

// The pet sheet of 面板设置 has what the old pet window had besides the desktop observation: five controls and three buttons.
const sheet = fs.readFileSync(new URL('../components/m3e/PanelOptions.vue', import.meta.url), 'utf8')
const speech = fs.readFileSync(new URL('./usePetSpeech.js', import.meta.url), 'utf8')

test('the pet sheet has the five controls of the old pet window', () => {
  for (const id of ['petMaster', 'petLanguage', 'petRandom', 'petSpeechInterval', 'petJitter', 'deskObserve', 'observeInterval', 'pokeMita', 'petAiMissing', 'petShortcut', 'petShortcutError', 'petShortcutUnavailable']) assert.ok(sheet.includes(`data-panel-option="${id}"`), id)
})
test('the pet sheet has its three buttons: the history (view, clear) and the defaults', () => {
  for (const id of ['petHistory', 'petHistoryClear', 'petReset']) assert.ok(sheet.includes(`data-panel-option="${id}"`), id)
})
test('the speech writes the keys of the pet and tells it, as the old window did', () => {
  for (const key of ['PET_MASTER_KEY', 'PET_RANDOM_ENABLED_KEY', 'PET_RANDOM_INTERVAL_KEY', 'PET_RANDOM_JITTER_KEY']) assert.ok(speech.includes(`write(${key}`) || speech.includes(`persist(${key}`), key)
  assert.match(speech, /notifyPetConfigChanged\(\)/)
  assert.match(speech, /load_toolbar_shortcut_status/)
  assert.match(speech, /set_toolbar_shortcut_enabled/)
})

test('the pet on or off is the first row of the sheet, and the row of 面板设置 shows its state', () => {
  const at = sheet.indexOf("sheet === 'pet'")
  assert.ok(at > 0)
  assert.ok(sheet.indexOf('data-panel-option="petMaster"', at) < sheet.indexOf('data-panel-option="petRandom"', at), 'the master switch comes first')
  assert.match(sheet, /:text="speech\.masterEnabled\.value \? st\.petOn : st\.petOff" data-panel-option="pet"/)
})
test('the groups of the sheet: 对话, 桌面观察, 快捷键, then the reset', () => {
  const at = sheet.indexOf("sheet === 'pet'")
  const marks = ['t.petTool.tabs.speech', 't.petTool.vision', 't.petTool.tabs.shortcuts', 'data-panel-option="petReset"'].map((m) => sheet.indexOf(m, at))
  assert.ok(marks.every((m) => m > at) && marks.every((m, i) => i === 0 || m > marks[i - 1]), String(marks))
})
test('the desktop observation asks for consent and needs a configured assistant', () => {
  assert.match(sheet, /const togglePet = \(on\) => \{ if \(!on\) stopObserving\(\); else consent\.value = true \}/)
  assert.match(sheet, /:disabled="!speech\.masterEnabled\.value \|\| !speech\.aiReady\.value"/)
  assert.match(speech, /\/api\/ai\/config/)
})

// the pet's own menu: a right click or a long press opens it, and the entries to hide the pet and to open its settings are words, not guesses
const pet = fs.readFileSync(new URL('../toolbar/ToolbarApp.vue', import.meta.url), 'utf8')
test('the pet menu opens on a right click and on a long press, and says what its bubbles do', () => {
  assert.match(pet, /@contextmenu\.prevent="onIconContextMenu"/)
  assert.match(pet, /longPressTimer = setTimeout\(/)
  assert.match(pet, /<span class="bubble-caption"/)
  for (const t of [zh, en]) {
    for (const id of ['main', 'vdd', 'dpi', 'bitrate', 'shortcuts', 'pet-settings', 'close']) assert.ok(t.toolbar.captions[id], id)
  }
  assert.equal(zh.toolbar.close, '隐藏桌宠')
  assert.equal(zh.toolbar.petSettings, '桌宠设置…')
  assert.equal(en.toolbar.close, 'Hide pet')
  assert.equal(en.toolbar.petSettings, 'Pet settings…')
})
