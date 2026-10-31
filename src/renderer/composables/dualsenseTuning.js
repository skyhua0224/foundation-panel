// Pure rules of the DualSense tuning page. The host publishes one descriptor per
// setting (ALK-DS5T/1); this module decides how those descriptors are laid out
// and compared. Nothing here touches the window, so it is covered by node tests.

import { localizedKeyLabel } from './dualsenseLabels.js'

export const SCHEMA = 'ALK-DS5T/1'
export const ADVANCED_GROUP_IDS = Object.freeze([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])

// Every day option belongs to one card. Keys 82-84 are the lightbar colour and are
// edited by the colour row rather than as three separate sliders.
export const COMMON_CARDS = Object.freeze([
  { id: 'haptics', icon: 'Bell', keys: [2, 3, 13, 14] },
  { id: 'triggers', icon: 'Pointer', keys: [24, 25] },
  { id: 'motion', icon: 'Compass', keys: [64, 75] },
  { id: 'lighting', icon: 'Sunny', keys: [81], lighting: true },
  { id: 'audio', icon: 'Headset', keys: [95] },
])

export const GROUP_ICONS = Object.freeze({
  1: 'Bell', 2: 'Iphone', 3: 'Pointer', 4: 'Aim', 5: 'Sort', 6: 'Compass',
  7: 'Mouse', 8: 'Sunny', 9: 'Headset', 10: 'Lock', 11: 'Refresh', 12: 'Share',
})

export const PRESET_CARDS = Object.freeze([
  { name: 'default', icon: 'RefreshLeft' },
  { name: 'competitive', icon: 'Lightning' },
  { name: 'immersive', icon: 'MagicStick' },
  { name: 'legacy', icon: 'Clock' },
])

const lightbar = (mode, r, g, b, brightness) => ({
  'ds5.led.lightbar.mode': mode,
  'ds5.led.lightbar.colorR': r,
  'ds5.led.lightbar.colorG': g,
  'ds5.led.lightbar.colorB': b,
  'ds5.led.lightbar.brightness': brightness,
})

export const PRESET_VALUES = Object.freeze({
  default: {
    'ds5.haptics.hd.gain': 1,
    'ds5.rumble.enabled': true,
    'ds5.rumble.strength': 1,
    'ds5.rumble.curve': 1,
    'ds5.rumble.lowGain': 1,
    'ds5.rumble.highGain': 1,
    'ds5.rumble.noiseGate': 0.02,
    ...lightbar('followGame', 0, 0, 0, 1),
  },
  competitive: {
    'ds5.haptics.hd.gain': 1.1,
    'ds5.rumble.enabled': true,
    'ds5.rumble.strength': 1.2,
    'ds5.rumble.curve': 1.2,
    'ds5.rumble.lowGain': 0.95,
    'ds5.rumble.highGain': 1,
    'ds5.rumble.noiseGate': 0.015,
    ...lightbar('fixed', 0, 92, 255, 0.8),
  },
  immersive: {
    'ds5.haptics.hd.gain': 1.4,
    'ds5.rumble.enabled': true,
    'ds5.rumble.strength': 1.3,
    'ds5.rumble.curve': 1.1,
    'ds5.rumble.lowGain': 1,
    'ds5.rumble.highGain': 1,
    'ds5.rumble.noiseGate': 0.02,
    ...lightbar('fixed', 255, 126, 24, 1),
  },
  legacy: {
    'ds5.haptics.hd.gain': 0,
    'ds5.rumble.enabled': true,
    'ds5.rumble.strength': 1,
    'ds5.rumble.curve': 0.5,
    'ds5.rumble.lowGain': 1,
    'ds5.rumble.highGain': 1,
    'ds5.rumble.noiseGate': 0.006,
    ...lightbar('followGame', 0, 0, 0, 1),
  },
})

// A fixed colour that is still black would switch the lightbar off, so the first
// time a person picks "fixed" they start from a visible blue instead.
export const FIRST_FIXED_COLOR = Object.freeze({ r: 74, g: 158, b: 255 })

export const isNumeric = (key) => ['f32', 'u32', 'i32'].includes(key.type)

/** Equality that tolerates float noise on numbers and compares the rest as text. */
export function sameValue(key, left, right) {
  if (isNumeric(key)) return Math.abs(Number(left) - Number(right)) < 1e-9
  return String(left) === String(right)
}

export const isModified = (key, value) => !sameValue(key, value, key.default)

export function indexKeys(snapshot) {
  const keys = snapshot?.keys || []
  return {
    byId: new Map(keys.map((key) => [key.id, key])),
    byName: new Map(keys.map((key) => [key.key, key])),
  }
}

export const isImplemented = (index, id) => index.byId.get(id)?.implemented === true

/** Common cards with the rows the running host implements; empty cards are dropped. */
export function visibleCards(index) {
  return COMMON_CARDS
    .map((card) => ({
      ...card,
      rows: card.keys.filter((id) => isImplemented(index, id)).map((id) => index.byId.get(id)),
    }))
    .filter((card) => card.rows.length)
}

export const colorWired = (index) =>
  [81, 82, 83, 84].every((id) => isImplemented(index, id))

/** Common settings the host knows but does not implement yet. */
export function pendingCommonRows(index) {
  return COMMON_CARDS.flatMap((card) =>
    card.keys.filter((id) => index.byId.has(id) && !isImplemented(index, id)).map((id) => index.byId.get(id)))
}

/**
 * Advanced settings grouped by function. Implemented rows come first, rows the
 * host cannot apply yet are kept apart at the end of their group.
 */
export function advancedGroups(snapshot, { search = '', onlyModified = false, showPending = true, dirty = {}, locale = 'zh' } = {}) {
  const query = search.trim().toLowerCase()
  const keys = (snapshot?.keys || []).filter((key) => {
    if (key.tier !== 'advanced') return false
    if (!showPending && !key.implemented) return false
    if (onlyModified && !dirty[key.id] && !key.stored) return false
    if (!query) return true
    return key.key.toLowerCase().includes(query)
      || String(key.label || '').toLowerCase().includes(query)
      || localizedKeyLabel(key, locale).toLowerCase().includes(query)
  })
  return ADVANCED_GROUP_IDS
    .map((id) => {
      const inGroup = keys.filter((key) => key.group === id)
      return {
        id,
        icon: GROUP_ICONS[id] || 'Operation',
        wired: inGroup.filter((key) => key.implemented),
        pending: inGroup.filter((key) => !key.implemented),
      }
    })
    .filter((group) => group.wired.length + group.pending.length > 0)
}

export const advancedCount = (snapshot) =>
  (snapshot?.keys || []).filter((key) => key.tier === 'advanced').length

/** Name of the preset whose values the edited profile matches, or `custom`. */
export function detectPreset(index, edited) {
  for (const [name, values] of Object.entries(PRESET_VALUES)) {
    const followsGame = values['ds5.led.lightbar.mode'] === 'followGame'
    const matches = Object.entries(values).every(([keyName, value]) => {
      const key = index.byName.get(keyName)
      if (!key) return true
      if (followsGame && /lightbar\.color[RGB]$/.test(keyName)) return true
      return String(edited[key.id]) === String(value)
    })
    if (matches) return name
  }
  return 'custom'
}

/** Values a preset changes, limited to what the host implements. */
export function presetEdits(index, name) {
  const edits = {}
  for (const [keyName, value] of Object.entries(PRESET_VALUES[name] || {})) {
    const key = index.byName.get(keyName)
    if (key?.implemented) edits[key.id] = value
  }
  return edits
}

/** Default values of every implemented setting that is not at its default now. */
export function defaultEdits(snapshot, edited) {
  const edits = {}
  for (const key of snapshot?.keys || []) {
    if (!key.implemented || sameValue(key, edited[key.id], key.default)) continue
    edits[key.id] = key.default
  }
  return edits
}

/** The request body for a save: changed settings only, by name. */
export function changedValues(snapshot, edited, dirty) {
  const values = {}
  const expert = []
  for (const key of snapshot?.keys || []) {
    if (!dirty[key.id]) continue
    values[key.key] = edited[key.id]
    if (key.expert) expert.push(key)
  }
  return { values, expert }
}

/** Number of digits a slider with this step shows. */
export function decimals(key) {
  const step = Number(key.step)
  const integer = key.type === 'u32' || key.type === 'i32'
  if (integer || !Number.isFinite(step) || step >= 1 || step <= 0) return 0
  return Math.min(6, Math.max(0, Math.ceil(-Math.log10(step))))
}

export function formatValue(key, value) {
  const text = isNumeric(key)
    ? (Number.isFinite(Number(value)) ? Number(value).toFixed(decimals(key)) : String(value))
    : String(value)
  if (key.unit === 'deg') return `${text}°`
  return key.unit ? `${text} ${key.unit}` : text
}
