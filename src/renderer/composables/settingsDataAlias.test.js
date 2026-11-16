import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')

test('the settings data is the product\'s, not the panel\'s: components import it through @settings-data, which the build points at the product checkout', () => {
  assert.match(read('../components/SidebarMenu.vue'), /import \{ SEARCH_INDEX \} from '@settings-data'/)
  assert.doesNotMatch(read('../components/SidebarMenu.vue'), /SEARCH_INDEX[^\n]*from 'foundation-ui\/settings/, 'the index does not come from the shared package any more')
  assert.match(read('../components/m3e/SettingsHubPage.vue'), /import '@settings-data'/)
  const vite = read('../../../vite.config.js')
  assert.match(vite, /find: \/\^@settings-data\$\/, replacement: settingsData/)
  assert.match(vite, /SETTINGS_DATA_DIR/)
  assert.match(vite, /settingsDataEmpty\.js/, 'a build without the product next to it falls back to the empty set')
})

test('without product data the set is empty and the shared labels answer the key (nothing throws)', async () => {
  const { register } = await import('node:module')
  const root = process.env.FOUNDATION_UI_DIR
  if (!root) return // the shared package is a sibling checkout; the other panel tests need it as well
  const { pathToFileURL } = await import('node:url')
  register('data:text/javascript,' + encodeURIComponent(`export async function resolve(s, c, n) { const m = /^foundation-ui\\/(settings\\/.+\\.js)$/.exec(s); if (m) return { url: new URL('src/' + m[1], ${JSON.stringify(pathToFileURL(root + '/').href)}).href, shortCircuit: true }; return n(s, c) }`))
  const { SEARCH_INDEX, LABELS } = await import('./settingsDataEmpty.js')
  const { labelOf, getSearchIndex, hasSettingsData } = await import('foundation-ui/settings/provider.js')
  assert.deepEqual([SEARCH_INDEX.length, Object.keys(LABELS)], [0, ['en']])
  assert.equal(hasSettingsData(), true)
  assert.equal(getSearchIndex().length, 0)
  assert.equal(labelOf('zh', 'tabs.display'), 'tabs.display')
})
