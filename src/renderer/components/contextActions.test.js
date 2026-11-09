import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// the object actions of the window's context menu in the panel (SPEC-context-menu): each is the function of the control it mirrors
const menu = readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')

test('运行体检 is 全部检查, 打开 Web UI is the address of the host in the system browser, the pet is the sheet the toolbar menu opens', () => {
  assert.ok(menu.includes("doctor: () => { componentStatus.openPage(); void componentStatus.checkAll() }"))
  assert.ok(menu.includes("openWebUi: async () => { const base = String((await sunshine.getUrl()) || ''); if (base) await openExternalUrl(base) }"))
  assert.ok(menu.includes("get pet() { return [{ id: 'pet-settings', label: contextTexts(locale.value).petSettings, icon: 'pets', run: () => openPanelPage('pet') }] }"), 'the label follows the language')
  assert.ok(menu.includes('<MContextProvider :locale="locale" :callbacks="ctxCallbacks" :capabilities="{ product: \'panel\', developer: developerMode.enabled.value }" />'))
})

test('the rows of the list pane that have a tile on the overview offer 固定到概览 through the functions of the search results\' pin', () => {
  assert.ok(menu.includes("const ROW_TILES = { [ROUTES.WEB_CLIENTS]: 'clients', [ROUTES.WEB_APPS]: 'apps', [ROUTES.WEB_DOWNLOADS]: 'downloads', [ROUTES.DIAGNOSTICS]: 'diagnostics' }"))
  assert.ok(menu.includes("isPinnedOnHome('panel-home', spec) ? pinTexts(locale.value).unpin : pinTexts(locale.value).pin"))
  assert.ok(menu.includes("if (isPinnedOnHome('panel-home', spec)) unpinFromHome('panel-home', spec)") && menu.includes("else if (pinToHome('panel-home', spec).ok)"))
  assert.ok(menu.includes("onPin: (doc) => pinToHome('panel-home', doc.pin)"), 'the search results pin to the same layout')
})
