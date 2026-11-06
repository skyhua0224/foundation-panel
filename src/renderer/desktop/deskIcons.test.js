import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { ICONS } from 'foundation-ui/icons'

// The icon set of foundation-ui is a subset of Material Symbols: a name that is not in it draws nothing (a gap where the glyph belongs).
// Every icon the settings, theme editor, tools, devices and stream pages name must be in the set.
const root = new URL('./', import.meta.url)
const files = [
  ...fs.readdirSync(new URL('components/settings/', root)).filter((f) => f.endsWith('.vue')).map((f) => `components/settings/${f}`),
  'components/ThemeEditor.vue', 'components/DeskPage.vue', 'components/PanelLink.vue', 'components/PanelLinkSection.vue', 'panelLinks.js', 'views/DashboardView.vue', 'components/AppToolbar.vue',
  'views/SettingsView.vue', 'views/ToolsView.vue', 'views/DevicesView.vue', 'views/StreamView.vue',
]

test('every icon name of the settings, theme, tools, devices and stream pages is in the icon set', () => {
  const missing = []
  for (const file of files) {
    const text = fs.readFileSync(new URL(file, root), 'utf8')
    const names = [...text.matchAll(/\sicon="([a-z0-9_]+)"/g), ...text.matchAll(/<MIcon[^>]*\sname="([a-z0-9_]+)"/g), ...text.matchAll(/\bicon: '([a-z0-9_]+)'/g)].map((m) => m[1])
    for (const name of names) if (!ICONS[name]) missing.push(`${file}: ${name}`)
  }
  assert.deepEqual(missing, [])
})
