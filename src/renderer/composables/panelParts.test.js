import test from 'node:test'
import assert from 'node:assert/strict'
import { nextTick, effectScope } from 'vue'
import { PANEL_PAGES, requestPanelPage, usePanelPart, panelPartWanted } from './panelParts.js'
import { readFileSync } from 'node:fs'
// ROUTES of useRouter.js, read from its source (it imports the UI framework)
const ROUTES = Object.fromEntries([...readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8').matchAll(/^  ([A-Z_]+): '([\w-]+)',/gm)].map((m) => [m[1], m[2]]))

test('the toolbar menu opens the new pages: DPI, bitrate, the shortcuts guide and the pet', () => {
  assert.deepEqual(Object.fromEntries(Object.entries(PANEL_PAGES).map(([k, v]) => [k, v.route])), {
    'desktop-scale': ROUTES.VDD_SETTINGS, 'live-session': ROUTES.LIVE_SESSION, shortcuts: ROUTES.HELP, pet: ROUTES.PANEL_SETTINGS,
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
