import test from 'node:test'
import assert from 'node:assert/strict'
import {
  advancedGroups, changedValues, colorWired, defaultEdits, detectPreset, formatValue,
  indexKeys, isModified, pendingCommonRows, presetEdits, visibleCards,
} from './dualsenseTuning.js'

const key = (id, name, extra = {}) => ({
  id, key: name, label: name, type: 'f32', group: 1, control: 'slider', tier: 'common',
  min: 0, max: 2, step: 0.01, default: 1, value: 1, stored: false, implemented: true,
  expert: false, unit: null, enum_names: [], ...extra,
})

const snapshot = () => ({
  schema: 'ALK-DS5T/1',
  keys: [
    key(2, 'ds5.haptics.mode', { type: 'enum', control: 'segmented', default: 'auto', value: 'auto', enum_names: ['auto', 'on'] }),
    key(3, 'ds5.haptics.hd.gain'),
    key(13, 'ds5.rumble.enabled', { type: 'bool', control: 'toggle', default: true, value: true }),
    key(14, 'ds5.rumble.strength'),
    key(24, 'ds5.trigger.adaptive.enabled', { type: 'bool', control: 'toggle', default: true, value: true, implemented: false }),
    key(81, 'ds5.led.lightbar.mode', { type: 'enum', default: 'followGame', value: 'followGame', enum_names: ['followGame', 'fixed', 'off'] }),
    key(82, 'ds5.led.lightbar.colorR', { type: 'u32', max: 255, step: 1, default: 0, value: 0 }),
    key(83, 'ds5.led.lightbar.colorG', { type: 'u32', max: 255, step: 1, default: 0, value: 0 }),
    key(84, 'ds5.led.lightbar.colorB', { type: 'u32', max: 255, step: 1, default: 0, value: 0 }),
    key(4, 'ds5.haptics.hd.lowBandDb', { tier: 'advanced', group: 1, min: -24, max: 12, step: 1, default: 0, value: 0, unit: 'dB' }),
    key(20, 'ds5.rumble.holdMs', { tier: 'advanced', group: 2, type: 'u32', min: 0, max: 500, step: 1, default: 40, value: 40, implemented: false }),
    key(21, 'ds5.rumble.watchdogMs', { tier: 'advanced', group: 2, type: 'u32', min: 0, max: 500, step: 1, default: 40, value: 90, stored: true }),
  ],
})

test('common cards keep only the rows the host implements and drop empty cards', () => {
  const index = indexKeys(snapshot())
  const cards = visibleCards(index)
  assert.deepEqual(cards.map((card) => card.id), ['haptics', 'lighting'])
  assert.deepEqual(cards[0].rows.map((row) => row.id), [2, 3, 13, 14])
  assert.deepEqual(pendingCommonRows(index).map((row) => row.id), [24])
})

test('the colour row needs the mode and all three channels', () => {
  assert.equal(colorWired(indexKeys(snapshot())), true)
  const partial = snapshot()
  partial.keys.find((row) => row.id === 84).implemented = false
  assert.equal(colorWired(indexKeys(partial)), false)
})

test('advanced groups list implemented rows first and keep unavailable ones apart', () => {
  const groups = advancedGroups(snapshot())
  assert.deepEqual(groups.map((group) => group.id), [1, 2])
  assert.deepEqual(groups[1].wired.map((row) => row.id), [21])
  assert.deepEqual(groups[1].pending.map((row) => row.id), [20])
  assert.equal(advancedGroups(snapshot(), { showPending: false })[1].pending.length, 0)
})

test('advanced filters match key names, labels and changed rows', () => {
  assert.deepEqual(advancedGroups(snapshot(), { search: 'lowband' }).flatMap((g) => g.wired.map((r) => r.id)), [4])
  assert.deepEqual(advancedGroups(snapshot(), { search: '看门狗' }), [])
  assert.deepEqual(advancedGroups(snapshot(), { search: '震动超时', locale: 'zh' }).flatMap((g) => g.wired.map((r) => r.id)), [21])
  const changed = advancedGroups(snapshot(), { onlyModified: true, dirty: { 4: true } })
  assert.deepEqual(changed.flatMap((g) => g.wired.map((r) => r.id)).sort(), [21, 4].sort())
})

test('modified means different from the host default, with float tolerance', () => {
  const gain = key(3, 'ds5.haptics.hd.gain')
  assert.equal(isModified(gain, 1), false)
  assert.equal(isModified(gain, 1 + 1e-12), false)
  assert.equal(isModified(gain, 1.2), true)
  const mode = key(2, 'm', { type: 'enum', default: 'auto' })
  assert.equal(isModified(mode, 'auto'), false)
  assert.equal(isModified(mode, 'on'), true)
})

test('a save carries only changed settings by name and flags expert ones', () => {
  const body = snapshot()
  body.keys[1].expert = true
  const edited = { 3: 1.4, 14: 1 }
  const { values, expert } = changedValues(body, edited, { 3: true })
  assert.deepEqual(values, { 'ds5.haptics.hd.gain': 1.4 })
  assert.deepEqual(expert.map((row) => row.id), [3])
  assert.deepEqual(changedValues(body, edited, {}).values, {})
})

test('presets change only implemented settings and are detected from the edits', () => {
  const body = snapshot()
  const index = indexKeys(body)
  const edits = presetEdits(index, 'competitive')
  assert.equal(edits[3], 1.1)
  assert.equal(edits[81], 'fixed')
  assert.equal(edits[24], undefined)

  const edited = Object.fromEntries(body.keys.map((row) => [row.id, row.value]))
  assert.equal(detectPreset(index, edited), 'default')
  Object.assign(edited, edits)
  assert.equal(detectPreset(index, edited), 'competitive')
  edited[3] = 1.23
  assert.equal(detectPreset(index, edited), 'custom')
})

test('restoring defaults touches only implemented settings that moved', () => {
  const body = snapshot()
  const edited = Object.fromEntries(body.keys.map((row) => [row.id, row.value]))
  edited[3] = 1.7
  edited[24] = false
  assert.deepEqual(defaultEdits(body, edited), { 3: 1, 21: 40 })
})

test('values show their precision and unit', () => {
  assert.equal(formatValue(key(3, 'g', { step: 0.01 }), 1.2), '1.20')
  assert.equal(formatValue(key(4, 'd', { step: 1, unit: 'dB' }), 3), '3 dB')
  assert.equal(formatValue(key(5, 'a', { step: 1, unit: 'deg' }), 12), '12°')
  assert.equal(formatValue(key(6, 'u', { type: 'u32', step: 1 }), 7), '7')
  assert.equal(formatValue(key(7, 'e', { type: 'enum' }), 'auto'), 'auto')
})
