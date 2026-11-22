// (router.nav.current, not router.currentRoute: under Node the panel and the shared package load two copies of Vue, and a computed of one does not track the refs of the other)
// The route is the one source of the rail, the list row and the title. These tests run the panel's REAL route tables (useRouter.js, panelNav.js, frameFollow.js) over the shared back stack.
import './_testing/foundationUi.mjs'
import assert from 'node:assert/strict'
import test from 'node:test'
import { zh } from '../desktop/i18n/zh.js'

const { ROUTES, PAGES, RAILS, HUB_OF, routeForWebPath, routeOfPlace, webPathOf, useRouter } = await import('./useRouter.js')
const { pageItems, barCurrent, BAR_GROUP_OF, railItems } = await import('./panelNav.js')
const { framedMove } = await import('./frameFollow.js')
const { parseSettingsHash } = await import('foundation-ui/settings/index.js')
// the index is the product's (the panel carries none); the routes need only a setting or two of it, made up here
const SEARCH_INDEX = [{ id: 'bind_address', title: 'config.bind_address', path: ['tabs.network'], icon: 'lan', route: 'config', params: { tab: 'network', focus: 'bind_address' } }]

const deps = { routeForWebPath, routeOfPlace, parseHash: (hash) => parseSettingsHash(hash, SEARCH_INDEX), configRoute: ROUTES.WEB_CONFIG }
const ALIASED = new Set([ROUTES.CONTROLLERS, ROUTES.COMPONENTS, ROUTES.WEB_TROUBLESHOOTING])

test('every page leads to a rail item (or is a first-level page with none lit) and the rail has a destination on the phone bar', () => {
  const known = new Set([...RAILS, 'appearance', 'about'])
  for (const [route, page] of Object.entries(PAGES)) {
    assert.ok(known.has(page.rail), `${route}: rail "${page.rail}" is not a rail of the shell`)
    assert.ok(BAR_GROUP_OF[page.rail], `${route}: the bottom bar has no destination for "${page.rail}"`)
  }
  for (const rail of RAILS) assert.ok(barCurrent(rail), `bar entry of ${rail}`)
})

test('navigating to a page selects exactly that page and its rail (the stack computes both, nothing else holds a selection)', () => {
  const router = useRouter()
  for (const [route, page] of Object.entries(PAGES)) {
    if (ALIASED.has(route)) continue
    router.navigate(ROUTES.HOME)
    router.navigate(route, undefined, page.push ? { root: false } : undefined)
    assert.equal(router.nav.current.value, route, `${route}: the route is the page`)
    assert.equal(router.nav.rail.value, page.rail, `${route}: the rail is ${page.rail}`)
  }
  router.navigate(ROUTES.HOME)
})

test('every page of a rail with a list is a row of that list (the row that is lit is the page that is shown)', () => {
  const rowsOf = (rail) => pageItems(zh, rail, { developer: true, rtssInstalled: true, dev: { pages: {}, lines: {} } }).map((i) => i.id)
  for (const [route, page] of Object.entries(PAGES)) {
    if (ALIASED.has(route) || page.list === false || page.push) continue
    if (page.rail === 'config') continue // the settings rows come from the shared settings list (places), not from pageItems
    assert.ok(rowsOf(page.rail).includes(route), `${route} is a page of "${page.rail}" but no row of that list stands for it`)
  }
})

test('a Web page asked for by its address maps back to the same route: the frame reporting what the panel asked for is no move (no loop)', () => {
  const router = useRouter()
  for (const [route] of Object.entries(PAGES)) {
    const path = webPathOf(route)
    if (!path) continue
    router.navigate(route)
    const params = router.nav.detail.value?.params
    assert.equal(framedMove(path, { route, params }, deps), null, `${route} → ${path}`)
  }
  router.navigate(ROUTES.HOME)
})

test('a frame that moves from any Web page to any other makes the panel follow: the rail and the row are those of the page shown', () => {
  const router = useRouter()
  const webRoutes = Object.keys(PAGES).filter((r) => webPathOf(r) && !ALIASED.has(r))
  for (const from of webRoutes) {
    for (const to of webRoutes) {
      if (from === to) continue
      // the settings page is addressed with its tab (a bare /config names none: the page keeps its own)
      const path = to === ROUTES.WEB_CONFIG ? '/config#tab=general' : webPathOf(to)
      const move = framedMove(path, { route: from }, deps)
      assert.ok(move, `${from} → ${path} is a move`)
      router.navigate(move.route, move.params)
      assert.equal(router.nav.current.value, to, `${from} → ${to}`)
      assert.equal(router.nav.rail.value, PAGES[to].rail, `${to}: rail follows`)
    }
  }
  router.navigate(ROUTES.HOME)
})

test('the frame reaching the Web index, diagnostics or the password page: each is the panel page that stands for it', () => {
  assert.equal(framedMove('/troubleshooting', { route: ROUTES.WEB_CONFIG, params: { tab: 'files' } }, deps).route, ROUTES.DIAGNOSTICS)
  assert.equal(framedMove('/password', { route: ROUTES.WEB_CONFIG, params: { tab: 'network' } }, deps).route, ROUTES.WEB_PASSWORD)
  assert.equal(framedMove('/pin#client-list', { route: ROUTES.WEB_CONFIG, params: { tab: 'network' } }, deps).route, ROUTES.WEB_CLIENTS)
  assert.deepEqual(framedMove('/config#bind_address', { route: ROUTES.WEB_CLIENTS }, deps).route, ROUTES.WEB_CONFIG)
})

test('the narrow window: a rail whose list does not exist on the phone is its hub, and the bar lights the hub for every settings category', () => {
  for (const rail of ['config', 'input', 'audio', 'display', 'system']) assert.equal(barCurrent(rail), 'settings-hub')
  for (const rail of Object.keys(HUB_OF)) assert.ok(HUB_OF[rail], rail)
  assert.equal(barCurrent('stream'), 'stream')
})

test('the rail list names every rail once', () => {
  const ids = railItems(zh).map((i) => i.id)
  assert.deepEqual(ids, RAILS)
})
