import test from 'node:test'
import assert from 'node:assert/strict'
import { nextTick, effectScope } from 'vue'
import { PANEL_PAGES, requestPanelPage, usePanelPart, panelPartWanted, deliverPendingPanelPage } from './panelParts.js'
import { readFileSync } from 'node:fs'
// ROUTES of useRouter.js, read from its source (it imports the UI framework)
const ROUTES = Object.fromEntries([...readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8').matchAll(/^  ([A-Z_]+): '([\w-]+)',/gm)].map((m) => [m[1], m[2]]))

test('native entries open the shell pages and the toolbar menu keeps its routes', () => {
  assert.deepEqual(Object.fromEntries(Object.entries(PANEL_PAGES).map(([k, v]) => [k, v.route])), {
    about: ROUTES.ABOUT, pairing: ROUTES.WEB_CLIENTS, logs: ROUTES.DIAGNOSTICS, quality: ROUTES.HDR_ENHANCED, rtss: ROUTES.RTSS,
    vdd: ROUTES.VDD_SETTINGS, 'desktop-scale': ROUTES.WEB_DISPLAY_OUTPUT, 'live-session': ROUTES.LIVE_SESSION, shortcuts: ROUTES.HELP, pet: ROUTES.PANEL_SETTINGS,
    // the rows 在 Panel 中打开 of the big screen
    'tool-paths': ROUTES.PANEL_SETTINGS, 'file-sharing': ROUTES.PANEL_SETTINGS, 'panel-advanced': ROUTES.PANEL_SETTINGS, 'display-modes': ROUTES.WEB_DISPLAY_MODES, 'input-drivers': ROUTES.COMPONENT_STATUS,
  })
  assert.equal(requestPanelPage('nothing'), null)
})

test('a page takes its part once, only when it accepts it', async () => {
  const scope = effectScope()
  const taken = []
  let helpShown = false
  scope.run(() => {
    usePanelPart('pet', () => { taken.push('pet') })
    usePanelPart('shortcuts', () => (helpShown ? taken.push('shortcuts') : false))
  })
  assert.equal(requestPanelPage('pet'), ROUTES.PANEL_SETTINGS)
  await nextTick(); await nextTick()
  assert.deepEqual(taken, ['pet']); assert.equal(panelPartWanted.value, '')
  assert.equal(requestPanelPage('shortcuts'), ROUTES.HELP)
  await nextTick(); await nextTick()
  assert.deepEqual(taken, ['pet']); assert.equal(panelPartWanted.value, 'shortcuts')
  scope.stop()
  // the page that shows later takes it
  helpShown = true
  effectScope().run(() => usePanelPart('shortcuts', () => (helpShown ? taken.push('shortcuts') : false)))
  await nextTick(); await nextTick()
  assert.deepEqual(taken, ['pet', 'shortcuts']); assert.equal(panelPartWanted.value, '')
})

test('a page left by the toolbar opens once, whether the ping or the mount takes it first', async () => {
  // the host's slot (panel_page.rs): taking empties it
  let slot = 'desktop-scale'
  const invoke = async (cmd) => { assert.equal(cmd, 'take_pending_panel_page'); await Promise.resolve(); const page = slot; slot = null; return page }
  const opened = []
  const results = await Promise.all([deliverPendingPanelPage(invoke, (p) => opened.push(p)), deliverPendingPanelPage(invoke, (p) => opened.push(p))])
  assert.deepEqual(opened, ['desktop-scale'])
  assert.deepEqual(results.sort(), ['desktop-scale', null].sort())
  // nothing waiting, an unknown name, a host that cannot answer: nothing opens
  assert.equal(await deliverPendingPanelPage(invoke, (p) => opened.push(p)), null)
  slot = 'no-such-page'
  assert.equal(await deliverPendingPanelPage(invoke, (p) => opened.push(p)), null)
  assert.equal(await deliverPendingPanelPage(async () => { throw new Error('ipc') }, (p) => opened.push(p)), null)
  assert.deepEqual(opened, ['desktop-scale'])
})
