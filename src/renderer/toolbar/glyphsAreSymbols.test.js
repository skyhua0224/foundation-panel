import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { ICONS } from 'foundation-ui/icons'

// The pet's menu bubbles take markup, and PanelWebHost has a back arrow: neither may carry a drawing of its own (they were Material Icons, the older 24 px set, and a Sharp arrow);
// they draw the shared set (Material Symbols Rounded 400) the way every other icon of the panel does.
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const toolbar = read('./ToolbarApp.vue')
const host = read('../components/PanelWebHost.vue')

test('the menu of the pet draws glyphs of the shared set, none is a drawing of its own', () => {
  const menu = toolbar.slice(toolbar.indexOf('const menuItems = computed'), toolbar.indexOf('const captionOf'))
  assert.ok(!menu.includes('<svg'), 'no inline svg in the menu items')
  const names = [...menu.matchAll(/icon: glyph\('([a-z_]+)'\)/g)].map((m) => m[1])
  assert.deepEqual(names, ['home', 'desktop_windows', 'aspect_ratio', 'swap_vert', 'keyboard', 'settings', 'close'])
  for (const n of names) assert.ok(ICONS[n], `${n} is in the set`)
  assert.ok(toolbar.includes('viewBox="0 -960 960 960"'), 'on the grid of the set')
})

test('the back arrow of the framed page is the arrow_back glyph', () => {
  assert.ok(host.includes('<MIcon name="arrow_back" />'))
  assert.ok(!host.includes('<svg class="m-icon"'))
})
