import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

// the router and the nav need the shared package (the bundler resolves it), so the destinations are read from the source
const router = fs.readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8')
const nav = fs.readFileSync(new URL('./panelNav.js', import.meta.url), 'utf8')

test('米塔 AI is a destination of the rail, the chat takes the whole width, and it has left the 系统 list', () => {
  assert.match(router, /export const RAILS = \[[^\]]*'system', 'assistant', 'developer', 'settings'/)
  assert.match(router, /\[ROUTES\.AI_ASSISTANT\]: \{ rail: 'assistant', list: false \}/)
  const system = nav.slice(nav.indexOf('system: ['), nav.indexOf('],', nav.indexOf('system: [')))
  assert.ok(!/AI_ASSISTANT/.test(system), 'not in the 系统 list')
  assert.match(nav, /assistant: 'smart_toy'/, 'a monochrome Material Symbol like every other destination')
  assert.ok(!/mita-rail|image:/.test(nav), 'no colour picture in the rail')
})
test('米塔 AI is labelled 米塔 AI (Mita AI), never AI 助手', () => {
  for (const [t, label, banned] of [[zh, '米塔 AI', /AI 助手/], [en, 'Mita AI', /AI Assistant/i]]) {
    assert.equal(t.m3e.nav.assistant, label)
    assert.equal(t.aiAssistant.title, label)
    assert.equal(t.sidebar.aiAssistant, label)
    assert.ok(!banned.test(t.aiAssistant.title))
  }
})

test("the bottom bar of a narrow window has five destinations with full labels and the settings categories behind one", async () => {
  const src = fs.readFileSync(new URL("./panelNav.js", import.meta.url), "utf8")
  const ids = [...src.slice(src.indexOf("export function barItems")).matchAll(/id: \x27([a-z-]+)\x27, icon/g)].map((m) => m[1])
  assert.deepEqual(ids, ["overview", "stream", "settings-hub", "assistant", "more"], "three to five, as the Material navigation bar has")
  for (const [lang, max] of [[zh, 6], [en, 10]]) { const n = lang.m3e.nav; for (const label of [n.overview, n.stream, n.barSettings, n.assistant, n.barMore]) assert.ok(label && [...label].length <= max, label) }
  const map = Object.fromEntries([...src.matchAll(/(\w+): \x27(overview|stream|settings-hub|assistant|more)\x27/g)].map((m) => [m[1], m[2]]))
  for (const rail of ["config", "input", "display", "system"]) assert.equal(map[rail], "settings-hub", rail)
  assert.equal(map.settings, "more"); assert.equal(map.help, "more")
})

test('网络概览 is a page of the 系统 list, two steps from the rail, and opens 串流设置 › 网络 for a fix', () => {
  assert.match(router, /NETWORK_OVERVIEW: 'network-overview'/)
  assert.match(router, /\[ROUTES\.NETWORK_OVERVIEW\]: \{ rail: 'system' \}/, 'rail 系统, with the list pane: rail, then the row')
  const system = nav.slice(nav.indexOf('system: ['), nav.indexOf('],', nav.indexOf('system: [')))
  assert.match(system, /id: ROUTES\.NETWORK_OVERVIEW, icon: 'lan', tone: 'primary', label: c\.system\.network, text: lines\.network/)
  assert.match(nav, /\[ROUTES\.NETWORK_OVERVIEW\]: 'network'/, 'the row line comes from the shared rowLines')
  assert.equal(zh.m3e.system.network, '网络概览')
  assert.equal(en.m3e.system.network, 'Network overview')
  const menu = fs.readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
  assert.match(menu, /<NetworkOverviewPage v-else-if="entry\.id === ROUTES\.NETWORK_OVERVIEW" @open-network="jumpConfig\(\{ tab: 'network' \}\)" \/>/)
  const page = fs.readFileSync(new URL('../components/m3e/NetworkOverviewPage.vue', import.meta.url), 'utf8')
  assert.match(page, /createNetworkOverview\(\{/, 'the shared state of foundation-ui, the same as the Web UI page')
  assert.match(page, /@recheck="state\.refresh\(true\)"/, '重新检测 asks the host again; opening the page reads its cache')
  assert.ok(!/setInterval|setTimeout/.test(page), 'nothing asks the host on a timer')
})
