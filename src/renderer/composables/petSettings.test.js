import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// The pet sheet of 面板设置 has what the old pet window had besides the desktop observation: five controls and three buttons.
const sheet = fs.readFileSync(new URL('../components/m3e/PanelOptions.vue', import.meta.url), 'utf8')
const speech = fs.readFileSync(new URL('./usePetSpeech.js', import.meta.url), 'utf8')

test('the pet sheet has the five controls of the old pet window', () => {
  for (const id of ['petMaster', 'petRandom', 'petSpeechInterval', 'petJitter', 'petShortcut']) assert.ok(sheet.includes(`data-panel-option="${id}"`), id)
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
