import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// the narrow window (a phone): 概览, 串流, 设置 and its pages are one app (the Web UI's phone design): the same hub page for 串流 as for 设置, Back is the hub, the caption bar is the Web UI's
const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')
const router = read('./useRouter.js')
const shell = read('../components/SidebarMenu.vue')
const stream = read('../components/m3e/StreamHubPage.vue')
const hub = read('../components/m3e/SettingsHubPage.vue')
const layout = fs.readFileSync(new URL('../../../node_modules/foundation-ui/src/css/layout.css', import.meta.url), 'utf8')

test('a phone has no list pages of the rails: the nav is told which hub stands for which rail, and the window size', () => {
  assert.match(router, /export const HUB_OF = \{ stream: ROUTES\.STREAM_HUB, config: ROUTES\.SETTINGS_HUB, input: ROUTES\.SETTINGS_HUB, audio: ROUTES\.SETTINGS_HUB, display: ROUTES\.SETTINGS_HUB, system: ROUTES\.SETTINGS_HUB, appearance: ROUTES\.SETTINGS_HUB, developer: ROUTES\.DEV_HUB \}/)
  assert.match(router, /createNav\(\{ routes, home: ROUTES\.HOME, compact: isPhone, hubOf: HUB_OF \}\)/)
  assert.match(router, /export const isPhone = \(\) => typeof window !== 'undefined' && window\.innerWidth < PHONE_MAX/)
  assert.match(router, /PHONE_MAX = 600/)
})
test('the stream hub is a page of the stream rail declared after its pages, so 客户端 stays the first page of the wide rail', () => {
  const pages = router.slice(router.indexOf('export const PAGES = {'), router.indexOf('export const RAILS'))
  assert.match(pages, /\[ROUTES\.STREAM_HUB\]: \{ rail: 'stream', list: false \}/)
  assert.ok(pages.indexOf('[ROUTES.STREAM_HUB]') > pages.indexOf('[ROUTES.LIVE_SESSION]'), 'after the pages of 串流')
  assert.ok(pages.indexOf('[ROUTES.WEB_CLIENTS]') < pages.indexOf('[ROUTES.STREAM_HUB]'))
})
test('串流 is the same page as 设置: MPage, one group of MItem rows, no action (添加应用 is on the page 串流应用); no page opens by itself on a phone', () => {
  for (const src of [stream, hub]) { assert.match(src, /<MPage :title=/); assert.match(src, /<MGroup>/); assert.match(src, /<MItem v-for="item in [a-zA-Z.]+"[^>]*action :icon="item\.icon" :title="item\.label" :text="item\.text"/) }
  assert.ok(!/#actions|MButton|'add'/.test(stream), 'the hub lists pages; the app list adds apps')
  assert.match(shell, /<StreamHubPage v-else-if="entry\.id === ROUTES\.STREAM_HUB"/)
  assert.match(shell, /if \(!isPhone\(\)\) emit\('webui-navigate', webPathOf\(ROUTES\.WEB_CLIENTS\)\)/)
})
test('the shell converts the stack when the window crosses 600 px, and starts wide', () => {
  assert.match(shell, /const now = isPhone\(\); if \(now !== wasPhone\) \{ wasPhone = now; if \(now\) nav\.narrow\(\); else nav\.widen\(\) \}/)
  assert.match(shell, /onMounted\(\(\) => \{ nav\.widen\(\); nav\.attach\(\) \}\)/)
})
test('the caption bar of a phone is the Web UI\'s: icon buttons of one size, no empty slot for the bell, the brand gives way; the sizes are shared in m3e', () => {
  assert.match(layout, /\.m-caption \.m-icon-btn, \.m-caption \.m-caption-connect \{ width: 44px; height: 44px;/)
  assert.match(layout, /\.m-caption \.m-statusbell--rest \{ display: none; \}/)
  assert.match(layout, /\.m-caption \.m-caption-connect \{ background: transparent;/)
  assert.match(shell, /\.panel-caption \{ grid-template-columns: minmax\(0, 1fr\) auto auto; \} \.panel-caption__brand \{ min-width: 0; overflow: hidden;/)
})
