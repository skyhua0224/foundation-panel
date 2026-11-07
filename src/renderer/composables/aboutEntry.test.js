import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createNav } from '../../../node_modules/foundation-ui/src/nav/backstack.js' // the back stack itself (the package root loads .vue files, which node cannot)

// 关于 has two doors. The About button in the footer of the rail opens it as a first-level page (no Back); 帮助与关于 pushes it over the help page (Back). How it was entered decides.
// The router and the sidebar need the bundler (.vue files), so their wiring is read from the source and the stack is run with the route as the table declares it.
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const router = read('./useRouter.js')
const sidebar = read('../components/SidebarMenu.vue')
const line = (name) => router.split('\n').find((l) => l.includes(`[ROUTES.${name}]: {`))
const flag = (l, k) => new RegExp(`${k}: true`).test(l)
const routeOf = (name) => { const l = line(name); return { role: 'detail', rail: /rail: '([a-z]+)'/.exec(l)[1], rootRail: /rootRail: '([a-z]+)'/.exec(l)?.[1], list: flag(l, 'list') ? true : false, push: flag(l, 'push') || undefined } }
const routes = () => ({ home: { role: 'detail', rail: 'overview', list: false }, help: routeOf('HELP'), about: routeOf('ABOUT'), appearance: routeOf('CFG_APPEARANCE') })

test('关于 from the footer of the rail is a first-level page, from 帮助与关于 it is pushed', () => {
  assert.equal(routes().about.push, true)
  assert.equal(routes().about.rootRail, 'about', 'the footer door lights no rail item')
  const nav = createNav({ routes: routes(), home: 'home', mirror: 'replace' })
  nav.go('help')
  nav.go('about')
  assert.equal(nav.pushed.value, true, 'opened from 帮助与关于: Back')
  assert.equal(nav.canPop.value, true)
  nav.go('home')
  nav.go('about', undefined, { root: true })
  assert.equal(nav.pushed.value, false, 'opened from the footer button: no Back')
  assert.equal(nav.canPop.value, false)
  assert.notEqual(nav.rail.value, 'help', 'and the rail item 帮助与关于 is not lit')
})

test('the footer buttons and the More sheet open 关于 first level; 帮助与关于 keeps pushing it; 个性化 has no Back wherever it is opened from', () => {
  assert.ok(sidebar.includes('router.navigate(ROUTES.ABOUT, undefined, { root: true })" />'), 'the footer button')
  assert.ok(sidebar.includes('moreGo(() => router.navigate(ROUTES.ABOUT, undefined, { root: true }))'), 'the More sheet of a narrow window')
  assert.ok(sidebar.includes('@open-about="router.navigate(ROUTES.ABOUT)"'), 'the help page pushes it')
  assert.ok(router.includes('stack.go(to, params, opts)'), 'navigate passes the door on')
  assert.equal(routes().appearance.push, undefined, '个性化 is no pushed page')
  const nav = createNav({ routes: routes(), home: 'home', mirror: 'replace' })
  nav.go('help')
  nav.go('appearance')
  assert.equal(nav.canPop.value, false, '个性化 from the footer has no Back')
})
