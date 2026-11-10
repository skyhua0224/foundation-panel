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

test('the settings entry of the bottom bar lists every page of the five categories and of 个性化 in sections: any page is two presses away', () => {
  const hub = read('../components/m3e/SettingsHubPage.vue'); const menu = read('../components/SidebarMenu.vue')
  assert.ok(hub.includes('<MSection v-for="section in sections"')); assert.ok(hub.includes("const CATEGORIES = ['config', 'input', 'audio', 'display', 'system']"))
  assert.ok(hub.includes("emit('open', item)")); assert.ok(!/select/.test(hub), 'a row opens a page, not another list')
  assert.ok(menu.includes('<SettingsHubPage v-else-if="entry.id === ROUTES.SETTINGS_HUB" :groups="settingsList.groups.value" :runtime="runtimeView" :focus="entry.params?.section || \'\'" @open="onListNavigate" />'))
})

test('the list of a rail has its title at the top left where the list is a page of its own (the bottom bar of a narrow window), as 设置 has', () => {
  const menu = read('../components/SidebarMenu.vue')
  assert.ok(menu.includes('<template #list="{ entry, scene }">'))
  assert.ok(menu.includes(':title="railLabel(entry.rail)"') && menu.includes(':titled="scene.kind === \'single\'"'))
})

test('更多 of the bottom bar lists 关于 (the update check is on that page): 检查更新 is two presses away on a phone', () => {
  const menu = read('../components/SidebarMenu.vue')
  const sheet = menu.slice(menu.indexOf('<MSheet v-model="moreOpen"'), menu.indexOf('<MAnnouncementPopup'))
  assert.ok(sheet.includes('icon="info" :title="aboutTexts(locale).title"') && sheet.includes('router.navigate(ROUTES.ABOUT, undefined, { root: true })'))
  assert.ok(sheet.indexOf('icon="info"') < sheet.indexOf('icon="help"'))
})

test('the rail of 常规, 配置文件, 网络 and 安全与登录 is 主机设置 in both languages, hub and rail alike', () => {
  assert.ok(zh.includes("config: '主机设置', input:") && en.includes("config: 'Host settings', input:"))
  assert.ok(zh.includes("settings: ['主机设置', '常规、配置文件、网络、安全与登录']") && en.includes("settings: ['Host settings', 'General, profiles, network, security and sign-in']"))
  assert.ok(!/streamSettings: '串流设置'/.test(zh) && zh.includes("streamSettings: '主机设置'"))
})

test('系统 names the one page of logs, errors, export, AI diagnosis, restart and reset 诊断与日志', () => {
  assert.ok(zh.includes("diagnostics: '诊断与日志',") && en.includes("diagnostics: 'Diagnostics and logs',"))
  assert.ok(zh.includes("troubleshooting: '诊断与日志', components:") && en.includes("troubleshooting: 'Diagnostics and logs', lead:"))
  const system = nav.slice(nav.indexOf('system: ['), nav.indexOf('],', nav.indexOf('system: [')))
  assert.ok(system.includes('ROUTES.DIAGNOSTICS') && !system.includes('WEB_TROUBLESHOOTING'), 'one row in the list')
  assert.ok(router.includes('[ROUTES.WEB_TROUBLESHOOTING]: ROUTES.DIAGNOSTICS'), 'the old route opens the same page')
  assert.ok(router.includes('/troubleshooting(?:\\.html)?(?:[?#/]|$)/, ROUTES.DIAGNOSTICS'), 'the old address opens the same page')
})

test('the overview gets 最近操作 and the live controls of a stream: the keys used last, the bitrate command, the DualSense tuning', () => {
  const home = read('../components/PanelHome.vue')
  assert.ok(home.includes('recent: recentKeys.value, canRuntime: canRuntime.value, ds5Keys: ds5Keys.value'))
  assert.ok(home.includes("case 'runtime-bitrate': runtimeBitrate(a.value); break") && home.includes("case 'ds5-tune': ds5Tune(a.value); break"))
  assert.ok(home.includes('/api/runtime/bitrate?bitrate=${kbps}&clientname=${encodeURIComponent(client)}'))
  assert.ok(home.includes("{ schema: 'ALK-DS5T/1', values: { [key]: value } }"))
  assert.ok(home.includes('if (r.ok) { state.config = r.config; rememberRecent(key) }'), 'a setting saved from a tile is a recent one')
})

test('the window\'s own context menu is mounted with what the panel can do, and the production build lets Shift+right-click through to the platform', () => {
  const menu = read('../components/SidebarMenu.vue'); const poly = read('../tauri-polyfill.js')
  assert.ok(menu.includes('<MContextProvider :locale="locale" :callbacks="ctxCallbacks" :capabilities="{ product: \'panel\', developer: developerMode.enabled.value }" />'))
  assert.ok(menu.includes("restartHost: () => hostConn.restart('service')") && menu.includes('openExternal: (u) => openExternalUrl(u)'))
  assert.ok(poly.includes("if (e.shiftKey || localStorage.getItem('foundation-ui.contextmenu') === 'native') return undefined"))
})

test('the test tools are a page of the toolbox: a route, an entry of the registry in both languages, the page in the shell', () => {
  assert.ok(router.includes("TEST_TOOLS: 'test-tools'") && router.includes('[ROUTES.TEST_TOOLS]: { rail: \'system\', list: false, push: true }'))
  const registry = read('./toolsRegistry.js'); assert.ok(registry.includes("id: 'test-tools'") && registry.includes('ctx.router.navigate(ROUTES.TEST_TOOLS)'))
  assert.ok(zh.includes("testTools: '测试工具'") && en.includes("testTools: 'Test tools'"))
  assert.ok(read('../components/SidebarMenu.vue').includes('<TestToolsPage v-else-if="entry.id === ROUTES.TEST_TOOLS" />'))
  assert.ok(read('../components/m3e/ToolboxPage.vue').includes("'test-tools': 'science'"))
})

test('the help page names the Web page of the logs with the name that page has (日志与排错), not the panel\'s 诊断', () => {
  const help = read('../components/ControlPanelPage.vue')
  assert.ok(help.includes(`:title="t.m3e.system.troubleshooting" @click="emit('webui-navigate', '/troubleshooting')"`))
})

test('the tools people come for are rows of 系统: the latency clock, the test tools and the timer are two presses (the toolbox made them three)', () => {
  const nav = read('./panelNav.js')
  const system = nav.slice(nav.indexOf('    system: ['), nav.indexOf('  return pages[rail]'))
  assert.deepEqual([...system.matchAll(/id: ROUTES\.(\w+),/g)].map((m) => m[1]), ['SUNSHINE_SERVICE', 'COMPONENT_STATUS', 'NETWORK_OVERVIEW', 'DIAGNOSTICS', 'DOCTOR', 'TOOLBOX', 'LATENCY_CLOCK', 'TEST_TOOLS', 'STREAM_TIMER', 'RTSS'])
  assert.ok(nav.includes("[ROUTES.LATENCY_CLOCK]: 'latency-clock', [ROUTES.TEST_TOOLS]: 'test-tools', [ROUTES.STREAM_TIMER]: 'stream-timer'"))
})

test('the list of 串流 has no 添加应用: adding an app is on the page 串流应用 (its FAB), the list is only pages', () => {
  const menu = read('../components/SidebarMenu.vue')
  assert.ok(!menu.includes('m-navlist__add') && !menu.includes("#top"), 'no button in the list pane')
  assert.ok(!menu.includes("c_act-add-app"))
  assert.ok(menu.includes("addApp: () => jumps.jump(() => { emit('webui-navigate', '/apps#guide-add'); router.navigate(ROUTES.WEB_APPS) })"), 'the search action still opens the page to add, as a jump so the back chip appears')
})

// fs52-gamepad-test-rumble: the panel's gamepad tool rumbles a stream's pad through the host
test('the panel test tools page hands the gamepad tool a call to the host rumble route', () => {
  const page = read('../components/m3e/TestToolsPage.vue')
  assert.ok(page.includes(':host-rumble="hostRumble"') && page.includes("'/api/dualsense/rumble-test'"))
  assert.ok(page.includes("sunshine.getProxyUrl()") && page.includes('status: response.status'), 'the address of the host and the HTTP status of a refusal')
})
