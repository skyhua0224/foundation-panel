import './_testing/foundationUi.mjs'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

// the second copy of Vue in the package looks for a document when it loads
globalThis.document ??= { createElement: () => ({}) }

const { ROUTES, RAILS, PAGES } = await import('./useRouter.js')
const { railItems, pageItems } = await import('./panelNav.js')
const { GAMEPAD_PAGE_ROUTES, dualsenseBack, isDualSenseEntry } = await import('./gamepadPages.js')
const { settingsGroups, categoryOfRow, labelOf, pageCrumb, buildSettingsEntries, searchEntries, SEARCH_INDEX, SETTINGS_PAGES } = await import('foundation-ui/settings/index.js')
const { zh } = await import('../desktop/i18n/zh.js')
const { en } = await import('../desktop/i18n/en.js')

const label = (key) => labelOf('zh', key)

test('DualSense is no rail item and no row of any list pane of the panel', () => {
  for (const t of [zh, en]) {
    const rails = railItems(t)
    assert.ok(!rails.some((r) => /dualsense/i.test(`${r.id} ${r.label}`)))
    for (const rail of RAILS) {
      const rows = rail === 'config' ? settingsGroups({ label }).flatMap((g) => g.items) : pageItems(t, rail, {})
      for (const row of rows) assert.ok(!/dualsense/i.test(`${row.id} ${row.label}`), `${rail}: ${row.id} ${row.label}`)
      assert.ok(!rows.some((row) => row.id === ROUTES.CFG_DUALSENSE))
    }
  }
})

test('the 手柄 destination of 输入与外设 and the DualSense page of the settings are the same page', async () => {
  assert.deepEqual([...GAMEPAD_PAGE_ROUTES], [ROUTES.GAMEPAD, ROUTES.CFG_DUALSENSE])
  const source = await readFile(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
  // one component serves both routes
  assert.match(source, /<GamepadPage v-else-if="entry\.id === ROUTES\.GAMEPAD \|\| entry\.id === ROUTES\.CFG_DUALSENSE"/)
  // the rail item 手柄 opens ROUTES.GAMEPAD; the old address /dualsense opens it too
  assert.equal(PAGES[ROUTES.GAMEPAD].rail, 'input')
  assert.equal(PAGES[ROUTES.CFG_DUALSENSE].rail, 'config')
  const handle = pageItems(zh, 'input', {}).find((p) => p.id === ROUTES.GAMEPAD)
  assert.equal(handle.label, '手柄')
  assert.ok(isDualSenseEntry(ROUTES.CFG_DUALSENSE) && !isDualSenseEntry(ROUTES.GAMEPAD))
})

test('the DualSense page of the settings goes 输入 > 手柄 > DualSense and its back bubble returns to 输入', () => {
  assert.ok(pageCrumb('dualsense', label).join(' › ').startsWith('输入 › 手柄 › DualSense'))
  const back = dualsenseBack()
  assert.equal(back.route, ROUTES.WEB_CONFIG)
  assert.equal(back.params.tab, 'input')
  assert.equal(categoryOfRow('config', back.params), 'input')
  assert.equal(SETTINGS_PAGES.dualsense.category, 'input')
})

test('search shows 输入 › 手柄 › DualSense', () => {
  const entries = buildSettingsEntries({ index: SEARCH_INDEX, label, altLabel: (k) => labelOf('en', k) })
  const hit = searchEntries(entries, 'dualsense', 5).find((e) => e.id === 'dualsense_page')
  assert.ok(hit)
  assert.ok([hit.path, hit.title].join(' › ').startsWith('输入 › 手柄 › DualSense'))
})
