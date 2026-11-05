import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// round 42: 输入与外设 is four pages (手柄, 键盘, 鼠标与触控, USB 转发); 串流设置 has no 输入 row; the settings entry of the bottom bar lists every page
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const router = read('./useRouter.js'); const nav = read('./panelNav.js'); const zh = read('../desktop/i18n/zh.js'); const en = read('../desktop/i18n/en.js')

test('the four pages: the places of the input tab lead to them, a page inside the gamepad page leads to 手柄', () => {
  assert.ok(router.includes("'input:gamepad': ROUTES.GAMEPAD, 'input:keyboard': ROUTES.WEB_INPUT_KEYBOARD, 'input:mouse': ROUTES.WEB_INPUT_MOUSE, 'input:usb': ROUTES.USB"))
  assert.ok(router.includes('PLACE_ROUTE[`${tab}:${parentPlace(tab, sub)}`]'), 'a page inside the gamepad page (per game, a DualSense group) is 手柄 at that page')
  assert.ok(router.includes("parentPlace } from 'foundation-ui/settings/index.js'"))
})

test('the Web address of each page is its place, and the address of a place leads back to its page', () => {
  assert.ok(router.includes('#tab=(display|audio|input)'))
  for (const [route, sub] of [['GAMEPAD', 'gamepad'], ['WEB_INPUT_KEYBOARD', 'keyboard'], ['WEB_INPUT_MOUSE', 'mouse']]) assert.ok(router.includes(`[ROUTES.${route}]: \`/config\${settingsHash({ tab: 'input', sub: '${sub}' })}\``), route)
  assert.ok(router.includes('/^\\/dualsense'), 'the old address of the DualSense page still leads to 手柄')
})

test('the rail 输入与外设 lists the four pages with their names, and the second lines come from the rows', () => {
  const input = nav.slice(nav.indexOf('    input: ['), nav.indexOf('    audio: ['))
  assert.deepEqual([...input.matchAll(/id: ROUTES\.(\w+),/g)].map((m) => m[1]), ['GAMEPAD', 'WEB_INPUT_KEYBOARD', 'WEB_INPUT_MOUSE', 'USB'])
  assert.deepEqual([...input.matchAll(/text: lines\.(\w+)/g)].map((m) => m[1]), ['gamepad', 'keyboard', 'mouse', 'usb'])
  assert.ok(nav.includes("[ROUTES.WEB_INPUT_KEYBOARD]: 'keyboard', [ROUTES.WEB_INPUT_MOUSE]: 'mouse'"))
  assert.ok(zh.includes("gamepad: '手柄', keyboard: '键盘', mouse: '鼠标与触控', microphone: '麦克风', usb: 'USB 转发'"))
  assert.ok(en.includes("gamepad: 'Controller', keyboard: 'Keyboard', mouse: 'Mouse and touch', microphone: 'Microphone', usb: 'USB forwarding'"))
})

test('the settings entry of the bottom bar lists every page of the five categories in sections: any page is two presses away', () => {
  const hub = read('../components/m3e/SettingsHubPage.vue'); const menu = read('../components/SidebarMenu.vue')
  assert.ok(hub.includes('<MSection v-for="section in sections"')); assert.ok(hub.includes("const CATEGORIES = ['config', 'input', 'audio', 'display', 'system']"))
  assert.ok(hub.includes("emit('open', item)")); assert.ok(!/select/.test(hub), 'a row opens a page, not another list')
  assert.ok(menu.includes('<SettingsHubPage v-else-if="entry.id === ROUTES.SETTINGS_HUB" :groups="settingsList.groups.value" :runtime="runtimeView" @open="onListNavigate" />'))
})
