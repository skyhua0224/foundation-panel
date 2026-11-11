import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// The page enter is one thing (foundation-ui nav/pageEnter.js). A routed page gets it from exactly one of two places: MScene plays it on the detail pane for every page the shell draws
// itself, and PanelWebHost plays it on the frame for every page that lives in the Web UI frame. A route that is neither (a new page whose branch was forgotten, or one that draws itself
// outside both) has no enter: that is how the virtual display page moved in and the encoding page did not.
const read = (rel) => fs.readFileSync(new URL(rel, import.meta.url), 'utf8')
const shell = read('../components/SidebarMenu.vue')
const host = read('../components/PanelWebHost.vue')
const m3e = (rel) => fs.readFileSync(new URL(`../../../node_modules/foundation-ui/${rel}`, import.meta.url), 'utf8')
// useRouter.js imports components, so it is read as text: ROUTES (name -> id), the keys of PAGES, and the routes webPathOf answers (the pages of the Web UI frame)
const router = read('./useRouter.js')
const block = (from, to) => router.slice(router.indexOf(from), router.indexOf(to, router.indexOf(from)))
const ROUTES = Object.fromEntries([...block('export const ROUTES', '}\n').matchAll(/^\s+([A-Z_0-9]+):\s*'([a-z0-9-]+)'/gm)].map((m) => [m[1], m[2]]))
const PAGES = Object.fromEntries([...block('export const PAGES', 'export const RAILS').matchAll(/\[ROUTES\.([A-Z_0-9]+)\]:/g)].map((m) => [ROUTES[m[1]], true]))
const FRAMED = new Set([...block('export const webPathOf', '}[route]').matchAll(/\[ROUTES\.([A-Z_0-9]+)\]:/g)].map((m) => ROUTES[m[1]]))
const webPathOf = (route) => (FRAMED.has(route) ? '/web' : null)
const NAME_OF = Object.fromEntries(Object.entries(ROUTES).map(([name, id]) => [id, name]))
// pages that are drawn by the branch of another route, or by one shared component, in the #detail template
const SAME_BRANCH = { [ROUTES.WEB_TROUBLESHOOTING]: ROUTES.DIAGNOSTICS, [ROUTES.COMPONENTS]: ROUTES.COMPONENT_STATUS }
const DEV_PAGES = [ROUTES.DEV_ISSUES, ROUTES.DEV_LOGS, ROUTES.DEV_API, ROUTES.DEV_FLAGS, ROUTES.DEV_BUILD, ROUTES.DEV_TESTS, ROUTES.DEVELOPER]
const APPEARANCE_PAGES = [ROUTES.CFG_APPEARANCE, ROUTES.CFG_FROST, ROUTES.CFG_OVERVIEW, ROUTES.CFG_TRAY] // 个性化: four routes, one shell branch (APPEARANCE_ROUTES.includes(entry.id))
const detail = shell.slice(shell.indexOf('<template #detail="{ entry }">'), shell.indexOf('<template #overlay>'))

test('every routed page is drawn either in the scene pane (MScene plays the enter) or in the Web UI frame (PanelWebHost plays it)', () => {
  const unwrapped = []
  const kinds = {}
  for (const route of Object.keys(PAGES)) {
    const framed = !!webPathOf(route)
    const target = SAME_BRANCH[route] || route
    const drawn = detail.includes(`entry.id === ROUTES.${NAME_OF[target]}`) || (DEV_PAGES.includes(route) && detail.includes('DEV_PAGES.includes(entry.id)')) || (APPEARANCE_PAGES.includes(route) && detail.includes('APPEARANCE_ROUTES.includes(entry.id)'))
    kinds[route] = framed ? 'frame' : drawn ? 'scene' : 'none'
    if (!framed && !drawn) unwrapped.push(route)
    assert.ok(!(framed && drawn), `${route} is drawn twice: in the frame and in the scene`)
  }
  assert.deepEqual(unwrapped, [], `routed pages that neither the scene nor the frame draws, so nothing plays their enter: ${unwrapped.join(', ')}`)
  assert.ok(Object.keys(PAGES).length >= 50, 'the router was parsed'); assert.ok(Object.values(kinds).filter((k) => k === 'frame').length >= 12, 'the places of the Web UI are framed pages')
  assert.equal(kinds[ROUTES.WEB_DISPLAY_ENCODERS], 'frame'); assert.equal(kinds[ROUTES.VDD_SETTINGS], 'scene')
})

test('the scene plays the enter for every change of the destination, whether the page draws an MPage or not, and a page no longer starts its own', () => {
  const scene = m3e('src/vue/MScene.vue')
  assert.match(scene, /import \{ playPageEnter \} from '..\/nav\/pageEnter\.js'/)
  assert.match(scene, /watch\(\(\) => detailEntry\.value\?\.key, \(key, before\) => \{[^}]*playPageEnter\(detailInner\.value\)/)
  assert.match(scene, /<div ref="detailInner" class="m-scene__inner"><slot v-if="detailEntry" name="detail"/)
  assert.match(m3e('src/css/pageloading.css'), /\.m-scene__pane \.m-page__column \{ animation: none; \}/, 'inside a scene the column does not play a second enter')
})

test('the frame plays the same enter when it is shown after a page change and when it moves to another place of the page it shows', () => {
  assert.match(host, /import \{[^}]*playPageEnter[^}]*\} from 'foundation-ui\/vue'/)
  assert.match(host, /const wasCovered = covered\.value[\s\S]*if \(wasCovered\) nextTick\(enter\)/, 'on the reveal of a covered frame')
  assert.match(host, /if \(!covered\.value\) \{ clearTimeout\(enterTimer\); enterTimer = setTimeout\(\(\) => \{ if \(!covered\.value\) enter\(\) \}, 40\); return \}/, 'on a place change the frame answers without a reload')
  assert.match(host, /<div ref="frameBox" class="panel-web__frame">/)
  assert.match(m3e('src/vue/index.js'), /export \{ playPageEnter, PAGE_ENTER \} from '..\/nav\/pageEnter\.js'/)
})
