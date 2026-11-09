import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// A page the rail or a row of its list opens is first level: no Back beside the list. The scene shows Back where the list is hidden behind the page (single pane) or for a page pushed over another one.
// So `push: true` belongs to exactly the pages opened over another page, never to a place of the rails.
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const router = read('./useRouter.js')
const PUSHED = ['ABOUT', 'ANNOUNCEMENTS', 'LATENCY_CLOCK', 'STREAM_TIMER', 'TEST_TOOLS', 'WELCOME']

test('only pages opened over another page are pushed (they alone show Back beside a list)', () => {
  const pages = router.slice(router.indexOf('export const PAGES = {'), router.indexOf('export const RAILS'))
  const pushed = [...pages.matchAll(/\[ROUTES\.([A-Z_0-9]+)\]: \{[^}]*push: true/g)].map((m) => m[1]).sort()
  assert.deepEqual(pushed, PUSHED, 'a new push: true page must be opened over another page, not be a place of a rail; update this list with the back audit')
})

test('the panel draws its own Back bar only where the scene says so, and not while the framed page has a Back of its own', () => {
  const host = read('../components/PanelWebHost.vue')
  // the decision is ownBackBar (frameBack.js): where the scene says so, not on a sub-page, and not where the page drew the arrow in its title row (frameBack.test.js)
  assert.ok(host.includes('v-if="ownBackBar({ canBack: scene?.canBack.value, sub, inline: inlineBack })"'))
})
