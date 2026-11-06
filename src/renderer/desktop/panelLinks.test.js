import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { PANEL_LINKS } from './panelLinks.js'
import { en } from './i18n/en.js'
import { zh } from './i18n/zh.js'

// The rows "在 Panel 中打开": the table of the big screen, the host's allow-list and the panel's pages are one list, and every row has its texts.
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
const parts = read('../composables/panelParts.js')
const rust = read('../../../src-tauri/src/commands.rs')
const main = read('../../../src-tauri/src/main.rs')
const panelPages = [...parts.slice(parts.indexOf('export const PANEL_PAGES'), parts.indexOf('export const PANEL_PAGE_PATHS')).matchAll(/^\s+'?([a-z-]+)'?: \{ route:/gm)].map((m) => m[1])
const hostPages = [...rust.slice(rust.indexOf('pub const PANEL_PAGES'), rust.indexOf('];', rust.indexOf('pub const PANEL_PAGES'))).matchAll(/"([a-z-]+)"/g)].map((m) => m[1])

test('the host allow-list of panel pages is the list of PANEL_PAGES of the renderer', () => {
  assert.deepEqual([...hostPages].sort(), [...panelPages].sort())
  assert.ok(main.includes('commands::open_panel_page,'), 'the command is registered')
  assert.ok(rust.includes('windows::open_main_panel_page(&app, &page)'), 'it asks the one main window, through open_main_panel_page')
  assert.ok(rust.includes('if !PANEL_PAGES.contains(&page.as_str())'), 'a name that is not on the list is refused first')
})

test('every row of the big screen opens a page that exists, and has its texts in both languages', () => {
  for (const [key, link] of Object.entries(PANEL_LINKS)) {
    assert.ok(hostPages.includes(link.page), `${key}: ${link.page} is a page the host opens`)
    for (const [name, texts] of [['en', en], ['zh', zh]]) {
      assert.ok(texts.panel.links[key]?.title && texts.panel.links[key]?.items, `${key} in ${name}`)
    }
  }
  assert.deepEqual(Object.keys(en.panel.links).sort(), Object.keys(PANEL_LINKS).sort())
  assert.equal(zh.panel.open, '在 Panel 中打开')
})

test('the panel takes the parts it is asked for: tool paths, file sharing, advanced', () => {
  const options = read('../components/m3e/PanelOptions.vue')
  for (const part of ['helpers', 'sharing', 'expert', 'pet']) assert.ok(options.includes(`usePanelPart('${part}'`), part)
  for (const link of Object.values(parts.match(/'(tool-paths|file-sharing|panel-advanced)': \{ route: '([a-z-]+)', part: '([a-z]+)' \}/g) || [])) assert.ok(link.includes("route: 'panel-settings'"))
})

test('a press on a row asks the host, and a refusal is said in a snackbar', () => {
  const link = read('./components/PanelLink.vue')
  assert.ok(link.includes('openInPanel(meta.page)') && link.includes('useNotify().snack(t.value.panel.openFailed'))
  assert.ok(read('./panelLinks.js').includes("tauriInvoke('open_panel_page', { page })"))
})

test('the pad moves inside the page before it leaves for the rail or the docked bar (the rows under the dock were unreachable)', () => {
  const nav = read('./composables/useFocusNav.js')
  assert.ok(nav.includes("current.closest('.desktop-window-main')") && nav.includes("current.closest('.desktop-sidebar')"))
  assert.ok(nav.includes('focusables.filter((element) => page.contains(element))'))
})

test('design guard: every page of the big screen opens with a hero band and its headings keep the colour of their tile', () => {
  const page = read('./components/DeskPage.vue')
  assert.ok(page.includes('<MHero v-if="icon"') && page.includes('.dsk-page :is(h1, h2, h3, h4, p) { color: inherit; }'))
  for (const view of ['DevicesView', 'StreamView', 'SettingsView', 'ToolsView']) assert.match(read(`./views/${view}.vue`), /<DeskPage[^>]*\sicon="[a-z_]+"/, view)
  for (const view of ['StreamView', 'SettingsView', 'ToolsView']) assert.ok(read(`./views/${view}.vue`).includes('<PanelLinkSection'), `${view} has its rows 在 Panel 中打开`)
  assert.ok(read('./components/DesktopWindow.vue').includes('id="dsk-dock"'))
})
