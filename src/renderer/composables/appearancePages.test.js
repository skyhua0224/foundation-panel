import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// 2026-11-10: 个性化 is a section with four pages like 输入与外设 (主题, 磨砂与动效, 概览, 托盘图标), not one page that ends in the tray settings
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const router = read('./useRouter.js'); const nav = read('./panelNav.js'); const zh = read('../desktop/i18n/zh.js'); const en = read('../desktop/i18n/en.js')
const menu = read('../components/SidebarMenu.vue'); const page = read('../components/m3e/AppearancePage.vue'); const hub = read('../components/m3e/SettingsHubPage.vue')

test('the four routes are pages of one rail with a list, the first of them is the old address of 个性化', () => {
  for (const [name, id] of [['CFG_APPEARANCE', 'cfg-appearance'], ['CFG_FROST', 'cfg-frost'], ['CFG_OVERVIEW', 'cfg-overview'], ['CFG_TRAY', 'cfg-tray']]) {
    assert.ok(router.includes(`${name}: '${id}'`), name)
    assert.ok(router.includes(`[ROUTES.${name}]: { rail: 'appearance' }`), `${name} is a page of the rail with a list pane (no list: false)`)
  }
  assert.ok(router.includes('export const APPEARANCE_ROUTES = Object.freeze([ROUTES.CFG_APPEARANCE, ROUTES.CFG_FROST, ROUTES.CFG_OVERVIEW, ROUTES.CFG_TRAY])'))
  assert.ok(!/RAILS = \[[^\]]*'appearance'/.test(router), 'the rail is the palette button in the foot, not an item of the rail')
  assert.ok(router.includes('appearance: ROUTES.SETTINGS_HUB'), 'a phone: Back from a page of 个性化 is the settings hub')
})

test('the list of 个性化 names the four pages, the second lines come from the rows, and a search for a page finds it', () => {
  const list = nav.slice(nav.indexOf('    appearance: ['), nav.indexOf('    stream: ['))
  assert.deepEqual([...list.matchAll(/id: ROUTES\.(\w+),/g)].map((m) => m[1]), ['CFG_APPEARANCE', 'CFG_FROST', 'CFG_OVERVIEW', 'CFG_TRAY'])
  assert.deepEqual([...list.matchAll(/text: lines\['([\w-]+)'\]/g)].map((m) => m[1]), ['appearance-theme', 'appearance-frost', 'appearance-home', 'appearance-tray'])
  assert.ok(nav.includes("[ROUTES.CFG_APPEARANCE]: 'appearance-theme', [ROUTES.CFG_FROST]: 'appearance-frost', [ROUTES.CFG_OVERVIEW]: 'appearance-home', [ROUTES.CFG_TRAY]: 'appearance-tray'"), 'the search keywords of each page')
  assert.ok(zh.includes("appearance: { theme: '主题', frost: '磨砂与动效', overview: '概览', tray: '托盘图标' }"))
  assert.ok(en.includes("appearance: { theme: 'Theme', frost: 'Frost and motion', overview: 'Overview', tray: 'Tray icon' }"))
  assert.ok(menu.includes("id === 'appearance' ? appearanceLabel.value"), 'the path of a result and the header of the list are 个性化')
})

test('each page draws its part: the tray settings only on the tray page, read only there', () => {
  assert.ok(menu.includes('<AppearancePage v-else-if="APPEARANCE_ROUTES.includes(entry.id)" :page="entry.id" />'))
  assert.ok(page.includes("[ROUTES.CFG_APPEARANCE]: 'theme', [ROUTES.CFG_FROST]: 'frost', [ROUTES.CFG_OVERVIEW]: 'home'"))
  assert.ok(page.includes('<MAppearance v-else :locale="String(locale)" :part="part" />'))
  assert.ok(page.includes('<MTraySettings v-if="isTray"'))
  assert.ok(page.includes("watch(isTray, async (on) => {") && page.includes('tray_prefs_get'), 'the tray record is read when the tray page shows')
  assert.ok(!page.includes('MSection'), 'a page of its own: no section titled like the page')
})

test('the palette button of the rail foot opens the first page and is lit on every page of 个性化; the narrow window lists the four pages', () => {
  assert.ok(menu.includes(`:current="nav.rail.value === 'appearance'" @click="router.navigate(ROUTES.CFG_APPEARANCE)"`))
  assert.ok(hub.includes("id: 'appearance', label: labelOf(locale.value, 'shell.appearance'), items: pageItems(t.value, 'appearance', props.runtime)"), 'the settings hub has a section 个性化 with the four pages')
  assert.ok(menu.includes("router.navigate(ROUTES.SETTINGS_HUB, { section: 'appearance' })"), '更多 › 个性化 of the bottom bar opens the hub at that section')
  assert.ok(hub.includes('data-hub-section') && hub.includes('scrollIntoView'))
  assert.ok(menu.includes("'appearance', 'settings', 'help'].flatMap"), 'Back names the page that was left')
  assert.ok(menu.includes('...APPEARANCE_ROUTES') && menu.includes("PAGES[id].list !== false"), 'the pages need no host and are search results')
})
