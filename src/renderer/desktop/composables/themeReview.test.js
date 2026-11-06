import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { BRIDGED_KEYS, snapshotMaterialBridge, withBridgedColours } from './materialBridge.js'
import { readSavedTheme } from './useTheme.js'

// review rv-md-desk1 (2026-11-06): F-01 migration flag, F-02 a tweak keeps the colours, F-03 status tokens exist, F-04 nav items for keyboard and pad
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const fakeStorage = (init = {}) => {
  const map = new Map(Object.entries(init))
  return { getItem: (k) => (map.has(k) ? map.get(k) : null), setItem: (k, v) => map.set(k, String(v)), map }
}
const KEY = 'theme'

test('F-01 a clean install writes the migration flag, so a later cyberpunk choice survives the next start', () => {
  const storage = fakeStorage()
  assert.equal(readSavedTheme(storage, KEY), null)
  assert.equal(storage.getItem(KEY + '-m3'), '1', 'the flag is written even with no saved theme')
  storage.setItem(KEY, JSON.stringify({ preset: 'cyberpunk', vars: { '--fd-accent': '#00fff5' } }))
  const next = readSavedTheme(storage, KEY)
  assert.deepEqual(next.vars, { '--fd-accent': '#00fff5' })
})

test('F-01 an old cyberpunk save from before the Material redo is reset once', () => {
  const storage = fakeStorage({ [KEY]: JSON.stringify({ preset: 'cyberpunk', vars: { a: '1' } }) })
  assert.equal(readSavedTheme(storage, KEY).vars, null)
  assert.equal(storage.getItem(KEY + '-m3'), '1')
  storage.setItem(KEY, JSON.stringify({ preset: 'cyberpunk', vars: { a: '1' } }))
  assert.deepEqual(readSavedTheme(storage, KEY).vars, { a: '1' }, 'chosen again afterwards, it stays')
})

test('F-01 other presets and broken saves are not touched', () => {
  assert.equal(readSavedTheme(fakeStorage({ [KEY]: '{not json' }), KEY), null)
  assert.deepEqual(readSavedTheme(fakeStorage({ [KEY]: JSON.stringify({ preset: 'custom', vars: { a: '1' } }) }), KEY).vars, { a: '1' })
})

test('F-02 a tweak made on the Material preset keeps every bridged colour', () => {
  const style = new Map(BRIDGED_KEYS.map((key, i) => [key, `#0000${(i % 10).toString(16)}${i % 10}`]))
  const root = { style: { getPropertyValue: (key) => style.get(key) || '' } }
  const snapshot = snapshotMaterialBridge(root)
  assert.deepEqual(Object.keys(snapshot).sort(), [...BRIDGED_KEYS].sort())
  const vars = withBridgedColours({ '--fd-card-radius': '20px' }, snapshot)
  for (const key of BRIDGED_KEYS) assert.ok(vars[key], key)
  assert.equal(vars['--fd-card-radius'], '20px')
  assert.equal(withBridgedColours({ '--fd-accent': '#ff0000' }, snapshot)['--fd-accent'], '#ff0000', 'a value the user set wins over the snapshot')
})

test('F-02 setVar and importTheme fill the colours before they leave the Material preset', () => {
  const src = read('./useTheme.js')
  const setVar = src.slice(src.indexOf('function setVar('), src.indexOf('// 应用预设'))
  assert.ok(setVar.indexOf("activePreset.value === 'material'") < setVar.indexOf("activePreset.value = 'custom'"))
  assert.ok(setVar.includes('withBridgedColours(themeVars.value, snapshotMaterialBridge('))
  assert.ok(src.includes('preset === \'material\' ? vars : withBridgedColours('))
})

test('F-03 the status colours the bridge reads are defined by foundation-ui in light and dark', () => {
  const css = read('../../../../node_modules/foundation-ui/src/tokens/tokens.css')
  const dark = css.indexOf("[data-bs-theme='dark']")
  for (const token of ['success', 'warning']) {
    const re = new RegExp(`--md-sys-color-${token}:`, 'g')
    const at = [...css.matchAll(re)].map((m) => m.index)
    assert.ok(at.some((i) => i < dark), `${token} in light`)
    assert.ok(at.some((i) => i > dark), `${token} in dark`)
  }
})

test('F-04 a disabled nav item is out of the Tab order and the pad list; keyboard focus shows a ring', () => {
  const side = read('../components/DesktopSidebar.vue')
  assert.equal((side.match(/:tabindex="item\.disabled \? -1 : 0"/g) || []).length, 2, 'both rail groups')
  assert.ok(!/\stabindex="0"/.test(side))
  const nav = read('./useFocusNav.js')
  assert.ok(nav.includes("'.nav-item:not(.disabled)'"))
  const less = read('../desktop.less')
  const block = less.slice(less.indexOf("html:not([data-fd-gamepad='true'])"))
  assert.ok(block.includes('*:focus:not(:focus-visible)') && block.includes('*:focus-visible'), 'only mouse focus loses the ring')
  assert.ok(block.includes('.nav-item:focus-visible'))
})

test('F-02 the theme editor rows show the colours in force on the Material preset, not an empty dot', () => {
  const te = read('../components/ThemeEditor.vue')
  assert.ok(te.includes('const shown = ref({})') && te.includes("props.vars[row.key] || style?.getPropertyValue(row.key)"))
  assert.ok(te.includes(':text="shown[row.key]"') && te.includes('background: shown[row.key]') && te.includes(':value="shown[row.key]"'))
  assert.ok(!/vars\[row\.key\]\s*\}\s*\"><\/span>/.test(te), 'the dot does not read vars directly any more')
})
