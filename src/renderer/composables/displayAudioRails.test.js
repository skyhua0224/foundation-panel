import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

// the router and the nav need the shared package (the bundler resolves it), so the destinations are read from the source
const router = fs.readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8')
const nav = fs.readFileSync(new URL('./panelNav.js', import.meta.url), 'utf8')
const rowsOf = (rail) => { const at = nav.indexOf(`    ${rail}: [`); return [...nav.slice(at, nav.indexOf('    ],', at)).matchAll(/id: ROUTES\.(\w+), icon: '(\w+)'/g)].map((m) => ({ route: m[1], icon: m[2] })) }

test('音频 and 显示与视频 are two rails of their own, between 输入与外设 and 系统', () => {
  assert.match(router, /export const RAILS = \[[^\]]*'input', 'audio', 'display', 'system'/)
  assert.equal(zh.m3e.nav.audio, '音频')
  assert.equal(zh.m3e.nav.display, '显示与视频')
  assert.equal(en.m3e.nav.audio, 'Audio')
  assert.equal(en.m3e.nav.display, 'Display & video')
  assert.match(nav, /audio: 'volume_up'/)
})
test('the rows of the two rails: the places of the display and audio tabs, none alone in its list', () => {
  assert.deepEqual(rowsOf('display').map((r) => r.route), ['WEB_DISPLAY_OUTPUT', 'WEB_DISPLAY_MODES', 'WEB_DISPLAY_HDR', 'VDD_SETTINGS', 'WEB_DISPLAY_VIDEO', 'HDR_ENHANCED'])
  assert.deepEqual(rowsOf('audio').map((r) => r.route), ['WEB_AUDIO_SOUND', 'MICROPHONE'])
  assert.deepEqual(rowsOf('input').map((r) => r.route), ['GAMEPAD', 'USB'])
  assert.match(router, /\[ROUTES\.MICROPHONE\]: \{ rail: 'audio' \}/)
})
test('a rail opens on the first row of its list (the first page of the rail in PAGES)', () => {
  for (const [rail, rows] of [['display', rowsOf('display')], ['audio', rowsOf('audio')]]) {
    const inPages = [...router.matchAll(/\[ROUTES\.(\w+)\]: \{ rail: '(\w+)'/g)].filter((m) => m[2] === rail).map((m) => m[1])
    assert.equal(inPages[0], rows[0].route, rail)
  }
})
test('every row of the two rails has an icon of its own, and the two rails differ', () => {
  const icons = [...rowsOf('display'), ...rowsOf('audio')].map((r) => r.icon)
  assert.equal(new Set(icons).size, icons.length)
  const rail = Object.fromEntries([...nav.matchAll(/(\w+): '([a-z_]+)'/g)].map((m) => [m[1], m[2]]))
  assert.notEqual(rail.audio, rail.display)
})
test('each page of the frame has the Web path of its place, and a path of a place leads back to its row', () => {
  for (const [route, tab, sub] of [['WEB_DISPLAY_OUTPUT', 'display', 'output'], ['WEB_DISPLAY_MODES', 'display', 'modes'], ['WEB_DISPLAY_HDR', 'display', 'hdr'], ['WEB_DISPLAY_VIDEO', 'display', 'video'], ['WEB_AUDIO_SOUND', 'audio', 'sound']]) {
    assert.match(router, new RegExp(`\\[ROUTES\\.${route}\\]: \`/config\\$\\{settingsHash\\(\\{ tab: '${tab}', sub: '${sub}' \\}\\)\\}\``))
    assert.match(router, new RegExp(`'${tab}:${sub}': ROUTES\\.${route}`))
  }
  assert.match(router, /'display:virtual': ROUTES\.VDD_SETTINGS/)
  assert.match(router, /'audio:microphone': ROUTES\.MICROPHONE/)
})
test('the virtual display page is titled like its row', () => {
  const page = fs.readFileSync(new URL('../components/VddSettings.vue', import.meta.url), 'utf8')
  assert.match(page, /\|\| t\.value\.m3e\.display\.vdd\)\)/)
})
