import { computed } from 'vue'
import { createNav } from 'foundation-ui/vue'
import { settingsHash, DEFAULT_PLACE, parentPlace } from 'foundation-ui/settings/index.js'
import { useI18n } from '../desktop/i18n/index.js'

/** Destinations of the control panel. A page is a detail entry of one back stack (see foundation-ui createNav). */
export const ROUTES = {
  HOME: 'home', // overview: the tile page
  VDD_SETTINGS: 'vdd-settings',
  WELCOME: 'welcome',
  WEB_STREAM: 'web-stream',
  AI_ASSISTANT: 'ai-assistant',
  CONTROLLERS: 'controllers', // old name of GAMEPAD
  HDR_ENHANCED: 'hdr-enhanced',
  TOOLBOX: 'toolbox', // old name of DIAGNOSTICS
  LATENCY_CLOCK: 'latency-clock', // 延迟测试时钟, opened from the toolbox
  STREAM_TIMER: 'stream-timer', // 串流计时器, opened from the toolbox
  TEST_TOOLS: 'test-tools', // 测试工具 (patterns, frame pacing, A/V sync, channels, input, keep awake), opened from the toolbox
  PANEL_SETTINGS: 'panel-settings',
  SETTINGS_HUB: 'settings-hub', // the bottom bar's one entry for 主机设置, 输入与外设, 显示 and 系统: a list of those four
  STREAM_HUB: 'stream-hub', // 串流 of a narrow window: the pages of the stream rail as one page of grouped rows, like SETTINGS_HUB (the rail's list page does not exist on a phone)
  HELP: 'help',
  ABOUT: 'about',
  DEVELOPER: 'developer', // 开发者选项: a page of the 开发者 rail (developer mode on); 系统 and 关于 link to it
  DEV_ISSUES: 'dev-issues', // Issue 与 PR of our own repositories (the first page of the 开发者 rail)
  DEV_LOGS: 'dev-logs', // 实时日志
  DEV_API: 'dev-api', // API 检查器
  DEV_FLAGS: 'dev-flags', // 功能开关: the overlays and the animation speed
  DEV_BUILD: 'dev-build', // 构建与组件
  DEV_TESTS: 'dev-tests', // 测试工具 of the developers: the error dialog, a host that is away, the telemetry check
  DEV_HUB: 'dev-hub', // 开发者 of a narrow window: the pages as one list
  CAT_DEVELOPER: 'cat-developer',
  ANNOUNCEMENTS: 'announcements', // 公告: every announcement, opened from the bell of the caption bar
  CAT_STREAM: 'cat-stream',
  CAT_INPUT: 'cat-input',
  CAT_AUDIO: 'cat-audio',
  CAT_DISPLAY: 'cat-display',
  CAT_SYSTEM: 'cat-system',
  GAMEPAD: 'gamepad',
  MICROPHONE: 'microphone',
  USB: 'usb',
  // the Web pages of 输入与外设 in the frame (/config#tab=input&sub=keyboard); the gamepad is a page of 手柄 beside them, USB 转发 is the panel's own page
  WEB_INPUT_KEYBOARD: 'web-input-keyboard',
  WEB_INPUT_MOUSE: 'web-input-mouse',
  COMPONENTS: 'components',
  COMPONENT_STATUS: 'component-status',
  SUNSHINE_SERVICE: 'sunshine-service', // 系统 › Sunshine 服务: the SCM, process and API layers, the startup type, start, stop, restart
  DIAGNOSTICS: 'diagnostics',
  DOCTOR: 'doctor', // 体检: the host's health check
  NETWORK_OVERVIEW: 'network-overview', // 网络概览: adapters, public IPv4/IPv6, NAT type, CGNAT, UPnP and firewall (GET /api/network/overview)
  // Pages of the Web UI shown in the embedded frame
  WEB_CLIENTS: 'web-clients',
  WEB_APPS: 'web-apps',
  WEB_CONFIG: 'web-config',
  CAT_CONFIG: 'cat-config',
  // 个性化: a section of its own (the palette button in the foot of the rail opens its first page), four pages like 输入与外设: 主题, 磨砂与动效, 概览, 托盘图标
  CFG_APPEARANCE: 'cfg-appearance', // 主题: the first page (the old address of the one 个性化 page lands here)
  CFG_FROST: 'cfg-frost', // 磨砂与动效
  CFG_OVERVIEW: 'cfg-overview', // 概览: the options of the overview tiles
  CFG_TRAY: 'cfg-tray', // 托盘图标
  WEB_DOWNLOADS: 'web-downloads',
  WEB_PASSWORD: 'web-password',
  // Places of 显示与视频 and 音频 that are pages of the Web UI, shown in the frame (/config#tab=display&sub=output)
  WEB_DISPLAY_OUTPUT: 'web-display-output',
  WEB_DISPLAY_MODES: 'web-display-modes',
  WEB_DISPLAY_ENCODERS: 'web-display-encoders',
  WEB_DISPLAY_HDR: 'web-display-hdr',
  WEB_DISPLAY_ADVANCED: 'web-display-advanced', // 高级: capture, graphics card, compatibility, sync (the former 捕获与视频)
  WEB_DISPLAY_VIDEO: 'web-display-video', // retired: an old address of the page lands on WEB_DISPLAY_ADVANCED (ALIAS)
  WEB_AUDIO_SOUND: 'web-audio-sound',
  WEB_TROUBLESHOOTING: 'web-troubleshooting',
  LIVE_SESSION: 'live-session', // 当前串流: the bitrate of a live session and the host performance (the old 码率调整 and 主机性能 tools)
  RTSS: 'rtss', // RTSS 叠加: frame-rate limit, OSD, stream monitoring and HWiNFO (the old RTSS tool)
}

/** rail: category of the page (rail item); list: false = the page takes the whole width (no list pane) */
export const PAGES = {
  [ROUTES.HOME]: { rail: 'overview', list: false },
  [ROUTES.WELCOME]: { rail: 'overview', list: false, push: true },
  [ROUTES.WEB_STREAM]: { rail: 'overview', list: false },
  [ROUTES.WEB_CLIENTS]: { rail: 'stream' },
  [ROUTES.WEB_APPS]: { rail: 'stream' },
  [ROUTES.WEB_DOWNLOADS]: { rail: 'stream' },
  [ROUTES.LIVE_SESSION]: { rail: 'stream' },
  [ROUTES.WEB_CONFIG]: { rail: 'config' },
  [ROUTES.CFG_APPEARANCE]: { rail: 'appearance' }, // the first page of a rail is the one the rail opens on
  [ROUTES.CFG_FROST]: { rail: 'appearance' },
  [ROUTES.CFG_OVERVIEW]: { rail: 'appearance' },
  [ROUTES.CFG_TRAY]: { rail: 'appearance' },
  [ROUTES.WEB_PASSWORD]: { rail: 'config' },
  [ROUTES.GAMEPAD]: { rail: 'input' },
  [ROUTES.WEB_INPUT_KEYBOARD]: { rail: 'input' },
  [ROUTES.WEB_INPUT_MOUSE]: { rail: 'input' },
  [ROUTES.WEB_AUDIO_SOUND]: { rail: 'audio' }, // the first page of a rail is the one the rail opens on
  [ROUTES.MICROPHONE]: { rail: 'audio' },
  [ROUTES.USB]: { rail: 'input' },
  [ROUTES.WEB_DISPLAY_OUTPUT]: { rail: 'display' }, // the page the rail opens on: the displays (the output display, the display mode)
  [ROUTES.WEB_DISPLAY_MODES]: { rail: 'display' },
  [ROUTES.WEB_DISPLAY_HDR]: { rail: 'display' },
  [ROUTES.HDR_ENHANCED]: { rail: 'display' },
  [ROUTES.WEB_DISPLAY_ENCODERS]: { rail: 'display' },
  [ROUTES.VDD_SETTINGS]: { rail: 'display' },
  [ROUTES.WEB_DISPLAY_ADVANCED]: { rail: 'display' },
  [ROUTES.SUNSHINE_SERVICE]: { rail: 'system' }, // the first page of 系统
  [ROUTES.COMPONENT_STATUS]: { rail: 'system' },
  [ROUTES.COMPONENTS]: { rail: 'system' },
  [ROUTES.TOOLBOX]: { rail: 'system' },
  [ROUTES.LATENCY_CLOCK]: { rail: 'system', list: false, push: true },
  [ROUTES.STREAM_TIMER]: { rail: 'system', list: false, push: true },
  [ROUTES.TEST_TOOLS]: { rail: 'system', list: false, push: true },
  [ROUTES.DIAGNOSTICS]: { rail: 'system' },
  [ROUTES.DOCTOR]: { rail: 'system' },
  [ROUTES.NETWORK_OVERVIEW]: { rail: 'system' },
  [ROUTES.AI_ASSISTANT]: { rail: 'assistant', list: false }, // 米塔 AI: the chat takes the whole width, its settings are behind the gear of the chat header
  [ROUTES.WEB_TROUBLESHOOTING]: { rail: 'system' },
  [ROUTES.RTSS]: { rail: 'system' },
  [ROUTES.PANEL_SETTINGS]: { rail: 'settings', list: false },
  [ROUTES.SETTINGS_HUB]: { rail: 'config', list: false },
  [ROUTES.HELP]: { rail: 'help', list: false },
  [ROUTES.ABOUT]: { rail: 'help', rootRail: 'about', list: false, push: true }, // pushed from 帮助与关于 (Back); a first-level page from the rail's About button (`root`, no Back, no rail item lit)
  // the 开发者 rail exists only while developer mode is on (seven taps on the version); its first page is Issue 与 PR
  [ROUTES.DEV_ISSUES]: { rail: 'developer' },
  [ROUTES.DEV_LOGS]: { rail: 'developer' },
  [ROUTES.DEV_API]: { rail: 'developer' },
  [ROUTES.DEV_FLAGS]: { rail: 'developer' },
  [ROUTES.DEV_BUILD]: { rail: 'developer' },
  [ROUTES.DEV_TESTS]: { rail: 'developer' },
  [ROUTES.DEVELOPER]: { rail: 'developer' },
  [ROUTES.DEV_HUB]: { rail: 'developer', list: false },
  [ROUTES.ANNOUNCEMENTS]: { rail: 'overview', list: false, push: true },
  [ROUTES.STREAM_HUB]: { rail: 'stream', list: false }, // after the pages of its rail: the first page of 串流 stays 客户端
}
// 开发者 sits above 面板设置 and 帮助与关于, the bottom group, so the rail above it is the same with developer mode off (the shell leaves the item out then)
/** The pages of 个性化, in list order (the rail itself is not in RAILS: its button is in the foot of the rail). */
export const APPEARANCE_ROUTES = Object.freeze([ROUTES.CFG_APPEARANCE, ROUTES.CFG_FROST, ROUTES.CFG_OVERVIEW, ROUTES.CFG_TRAY])
export const RAILS = ['overview', 'stream', 'config', 'input', 'audio', 'display', 'system', 'assistant', 'developer', 'settings', 'help']

// the old component page is part of the component status page now
const ALIAS = { [ROUTES.CONTROLLERS]: ROUTES.GAMEPAD, [ROUTES.COMPONENTS]: ROUTES.COMPONENT_STATUS, [ROUTES.WEB_TROUBLESHOOTING]: ROUTES.DIAGNOSTICS, [ROUTES.WEB_DISPLAY_VIDEO]: ROUTES.WEB_DISPLAY_ADVANCED }
const CATEGORY = { [ROUTES.CAT_CONFIG]: 'config', [ROUTES.CAT_STREAM]: 'stream', [ROUTES.CAT_INPUT]: 'input', [ROUTES.CAT_AUDIO]: 'audio', [ROUTES.CAT_DISPLAY]: 'display', [ROUTES.CAT_SYSTEM]: 'system', [ROUTES.CAT_DEVELOPER]: 'developer' }
// The panel destination of a place of the display and audio tabs (the same places as foundation-ui's DISPLAY_PLACES and AUDIO_PLACES): the virtual display and the microphone are pages of the panel itself
const PLACE_ROUTE = {
  'display:output': ROUTES.WEB_DISPLAY_OUTPUT, 'display:modes': ROUTES.WEB_DISPLAY_MODES, 'display:encoders': ROUTES.WEB_DISPLAY_ENCODERS, 'display:advanced': ROUTES.WEB_DISPLAY_ADVANCED, 'display:video': ROUTES.WEB_DISPLAY_ADVANCED, 'display:hdr': ROUTES.WEB_DISPLAY_HDR, 'display:virtual': ROUTES.VDD_SETTINGS, 'display:quality': ROUTES.HDR_ENHANCED, 'display:quality': ROUTES.HDR_ENHANCED,
  'audio:sound': ROUTES.WEB_AUDIO_SOUND, 'audio:microphone': ROUTES.MICROPHONE,
  'input:gamepad': ROUTES.GAMEPAD, 'input:keyboard': ROUTES.WEB_INPUT_KEYBOARD, 'input:mouse': ROUTES.WEB_INPUT_MOUSE, 'input:usb': ROUTES.USB,
}
// a page inside the gamepad page (per game, a DualSense group) is 手柄 at that page
export const routeOfPlace = (tab, sub) => PLACE_ROUTE[`${tab}:${sub || DEFAULT_PLACE[tab]}`] || PLACE_ROUTE[`${tab}:${parentPlace(tab, sub)}`] || null
const PLACE_PATH = /^\/config(?:\.html)?#tab=(display|audio|input)(?:&sub=([\w-]+))?/
// tabs that were pages of their own: an old address still reaches what moved
const MOVED_TAB_PATH = /^\/config(?:\.html)?#tab=(advanced|encoders)(?:&|$)/
const MOVED_TAB_ROUTE = { advanced: ROUTES.WEB_DISPLAY_ADVANCED, encoders: ROUTES.WEB_DISPLAY_ENCODERS }
const WEB_PATHS = [
  [/^\/(?:pin|clients)(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_CLIENTS],
  [/^\/apps(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_APPS],
  [/^\/config(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_CONFIG],
  [/^\/downloads(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_DOWNLOADS],
  [/^\/password(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_PASSWORD],
  [/^\/troubleshooting(?:\.html)?(?:[?#/]|$)/, ROUTES.DIAGNOSTICS],
  [/^\/diagnostics(?:\.html)?(?:[?#/]|$)/, ROUTES.DIAGNOSTICS],
  [/^\/dualsense(?:\.html)?(?:[?#/]|$)/, ROUTES.GAMEPAD],
]
/** The panel destination that shows a Web UI path; the Web home is the panel's own overview. */
export const routeForWebPath = (path) => {
  const p = String(path || '/')
  const place = PLACE_PATH.exec(p)
  if (place && routeOfPlace(place[1], place[2])) return routeOfPlace(place[1], place[2])
  const moved = MOVED_TAB_PATH.exec(p)
  if (moved) return MOVED_TAB_ROUTE[moved[1]]
  for (const [re, route] of WEB_PATHS) if (re.test(p)) return route
  return ROUTES.HOME
}
/** The Web UI path of a Web page destination (null for panel pages). */
export const webPathOf = (route) => ({
  [ROUTES.WEB_CLIENTS]: '/pin', [ROUTES.WEB_APPS]: '/apps', [ROUTES.WEB_CONFIG]: '/config', [ROUTES.WEB_DOWNLOADS]: '/downloads',
  [ROUTES.WEB_PASSWORD]: '/password',
  [ROUTES.WEB_DISPLAY_OUTPUT]: `/config${settingsHash({ tab: 'display', sub: 'output' })}`, [ROUTES.WEB_DISPLAY_MODES]: `/config${settingsHash({ tab: 'display', sub: 'modes' })}`,
  [ROUTES.WEB_DISPLAY_ENCODERS]: `/config${settingsHash({ tab: 'display', sub: 'encoders' })}`, [ROUTES.WEB_DISPLAY_ADVANCED]: `/config${settingsHash({ tab: 'display', sub: 'advanced' })}`,
  [ROUTES.WEB_DISPLAY_HDR]: `/config${settingsHash({ tab: 'display', sub: 'hdr' })}`,
  [ROUTES.WEB_AUDIO_SOUND]: `/config${settingsHash({ tab: 'audio', sub: 'sound' })}`,
  // the gamepad, the keyboard and the mouse are pages of 输入与外设 in the Web UI: the rail destinations show those pages in the frame
  [ROUTES.GAMEPAD]: `/config${settingsHash({ tab: 'input', sub: 'gamepad' })}`,
  [ROUTES.WEB_INPUT_KEYBOARD]: `/config${settingsHash({ tab: 'input', sub: 'keyboard' })}`,
  [ROUTES.WEB_INPUT_MOUSE]: `/config${settingsHash({ tab: 'input', sub: 'mouse' })}`,
}[route] || null)

const routes = {}
for (const rail of new Set(Object.values(PAGES).filter((p) => p.list !== false).map((p) => p.rail))) routes[`${rail}:list`] = { role: 'list', rail }
for (const [id, p] of Object.entries(PAGES)) routes[id] = { role: 'detail', rail: p.rail, list: p.list, supporting: p.supporting, push: p.push, rootRail: p.rootRail }

// a phone has no list pages of the rails: 串流 is the stream hub, every other category is in the 设置 hub; Back from a page is that hub, whatever way the page was reached (as in the Web UI)
export const HUB_OF = { stream: ROUTES.STREAM_HUB, config: ROUTES.SETTINGS_HUB, input: ROUTES.SETTINGS_HUB, audio: ROUTES.SETTINGS_HUB, display: ROUTES.SETTINGS_HUB, system: ROUTES.SETTINGS_HUB, appearance: ROUTES.SETTINGS_HUB, developer: ROUTES.DEV_HUB }
export const PHONE_MAX = 600
export const isPhone = () => typeof window !== 'undefined' && window.innerWidth < PHONE_MAX

let nav = null
const shared = () => (nav ||= createNav({ routes, home: ROUTES.HOME, compact: isPhone, hubOf: HUB_OF }))

/** Navigation of the panel: one back stack for the whole window. */
export function useRouter() {
  const { t } = useI18n()
  const stack = shared()
  // opts.root: the page of a push route (关于) as a first-level page, from the rail's footer: no back arrow
  const navigate = (name, params, opts) => {
    const to = ALIAS[name] || name
    if (CATEGORY[to]) return stack.selectRail(CATEGORY[to])
    if (!PAGES[to]) { console.warn(`route ${name} does not exist`); return }
    stack.go(to, params, opts)
  }
  const currentRoute = computed(() => stack.current.value)
  const goHome = () => navigate(ROUTES.HOME)
  const goBack = () => stack.pop()
  return {
    nav: stack,
    currentRoute,
    navigate,
    goBack,
    goHome,
    isRoute: (name) => currentRoute.value === (ALIAS[name] || name),
    getCurrentRouteConfig: computed(() => ({ name: currentRoute.value, title: t.value.routeTitles?.[currentRoute.value] || '' })),
    ROUTES,
  }
}
