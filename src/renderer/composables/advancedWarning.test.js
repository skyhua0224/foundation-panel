import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { en } from '../desktop/i18n/en.js'
import { zh } from '../desktop/i18n/zh.js'
import {
  ADVANCED_WARNING_HIDDEN_KEY,
  ADVANCED_WARNING_SECONDS,
  isAdvancedWarningHidden,
  remainingSeconds,
  setAdvancedWarningHidden,
} from './advancedWarning.js'

const memoryStorage = () => {
  const map = new Map()
  return {
    getItem: (key) => (map.has(key) ? map.get(key) : null),
    setItem: (key, value) => map.set(key, String(value)),
    removeItem: (key) => map.delete(key),
  }
}

const refusingStorage = () => ({
  getItem() { throw new Error('denied') },
  setItem() { throw new Error('denied') },
  removeItem() { throw new Error('denied') },
})

test('the warning shows until the panel stores the opt-out', () => {
  const storage = memoryStorage()
  assert.equal(isAdvancedWarningHidden(storage), false)
  setAdvancedWarningHidden(storage, true)
  assert.equal(storage.getItem(ADVANCED_WARNING_HIDDEN_KEY), '1')
  assert.equal(isAdvancedWarningHidden(storage), true)
  setAdvancedWarningHidden(storage, false)
  assert.equal(isAdvancedWarningHidden(storage), false)
})

test('only the stored value 1 counts as an opt-out, and blocked storage keeps the warning on', () => {
  const storage = memoryStorage()
  storage.setItem(ADVANCED_WARNING_HIDDEN_KEY, 'true')
  assert.equal(isAdvancedWarningHidden(storage), false)
  assert.equal(isAdvancedWarningHidden(null), false)
  assert.equal(isAdvancedWarningHidden(refusingStorage()), false)
  assert.doesNotThrow(() => setAdvancedWarningHidden(refusingStorage(), true))
  assert.doesNotThrow(() => setAdvancedWarningHidden(null, true))
})

test('the confirm countdown runs five whole seconds and then unlocks', () => {
  assert.equal(ADVANCED_WARNING_SECONDS, 5)
  assert.equal(remainingSeconds(0), 5)
  assert.equal(remainingSeconds(999), 5)
  assert.equal(remainingSeconds(1000), 4)
  assert.equal(remainingSeconds(4999), 1)
  assert.equal(remainingSeconds(5000), 0)
  assert.equal(remainingSeconds(60000), 0)
  assert.equal(remainingSeconds(-250), 5)
})

test('both panel languages carry the complete warning text for each section', () => {
  for (const [name, messages] of [['en', en], ['zh', zh]]) {
    const warning = messages.dualSense.tuning.advanced_warning
    for (const key of ['title', 'dont_show', 'confirm', 'confirm_wait']) {
      assert.ok(warning[key]?.trim(), `${name} is missing advanced_warning.${key}`)
    }
    assert.match(warning.confirm_wait, /\{seconds\}/)
    for (const scope of ['tuning', 'emulation']) {
      assert.ok(warning[scope].intro?.trim(), `${name} ${scope} intro`)
      assert.ok(warning[scope].outro?.trim(), `${name} ${scope} outro`)
      assert.ok(warning[scope].risks.length >= 3 && warning[scope].risks.every((risk) => risk.trim()), `${name} ${scope} risks`)
    }
  }
  assert.equal(zh.dualSense.tuning.advanced_warning.dont_show, '下次不再显示此警告')
})

test('both advanced entries ask first and never restore past the warning', async () => {
  const page = await readFile(new URL('../components/m3e/GamepadPage.vue', import.meta.url), 'utf8')
  // the tuning group and the emulation options both go through askAdvanced
  assert.match(page, /askAdvanced\('advanced'\)/)
  assert.match(page, /askAdvanced\('emulation', 'emulation'\)/)
  const ask = page.match(/const askAdvanced = \(target, scope = 'tuning'\) => \{[\s\S]*?\n\}\n/)?.[0] ?? ''
  assert.ok(ask.indexOf('isAdvancedWarningHidden') > 0)
  assert.ok(ask.indexOf('isAdvancedWarningHidden') < ask.indexOf('warning, { open: true'))
  assert.match(page, /<AdvancedWarningDialog[\s\S]*?@confirm="confirmWarning"/)
  // reopening after a reload never lands behind the warning
  assert.match(page, /startsWith\(p\)\)[\s\S]*?!isAdvancedWarningHidden\(panelStorage\(\)\)\) current\.value = ''/)
  // the countdown and the opt-out box live in the dialog
  const dialog = await readFile(new URL('../components/dualsense/AdvancedWarningDialog.vue', import.meta.url), 'utf8')
  assert.match(dialog, /ADVANCED_WARNING_SECONDS|remainingSeconds/)
})
