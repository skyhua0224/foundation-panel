import { computed } from 'vue'
import { createNav } from 'foundation-ui/vue'
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
  PANEL_SETTINGS: 'panel-settings',
  HELP: 'help',
  CAT_STREAM: 'cat-stream',
  CAT_INPUT: 'cat-input',
  CAT_DISPLAY: 'cat-display',
  CAT_SYSTEM: 'cat-system',
  GAMEPAD: 'gamepad',
  MICROPHONE: 'microphone',
  USB: 'usb',
  COMPONENTS: 'components',
  DIAGNOSTICS: 'diagnostics',
  // Pages of the Web UI shown in the embedded frame
  WEB_CLIENTS: 'web-clients',
  WEB_APPS: 'web-apps',
  WEB_CONFIG: 'web-config',
  CFG_DUALSENSE: 'cfg-dualsense', // the DualSense tuning page, listed with the stream settings
  CAT_CONFIG: 'cat-config',
  CFG_APPEARANCE: 'cfg-appearance', // 个性化 among the stream settings
  WEB_DOWNLOADS: 'web-downloads',
  WEB_PASSWORD: 'web-password',
  WEB_TROUBLESHOOTING: 'web-troubleshooting',
}

/** rail: category of the page (rail item); list: false = the page takes the whole width (no list pane) */
export const PAGES = {
  [ROUTES.HOME]: { rail: 'overview', list: false },
  [ROUTES.WELCOME]: { rail: 'overview', list: false },
  [ROUTES.WEB_STREAM]: { rail: 'overview', list: false },
  [ROUTES.WEB_CLIENTS]: { rail: 'stream' },
  [ROUTES.WEB_APPS]: { rail: 'stream' },
  [ROUTES.WEB_DOWNLOADS]: { rail: 'stream' },
  [ROUTES.WEB_CONFIG]: { rail: 'config' },
  [ROUTES.CFG_DUALSENSE]: { rail: 'config' },
  [ROUTES.CFG_APPEARANCE]: { rail: 'config' },
  [ROUTES.WEB_PASSWORD]: { rail: 'config' },
  [ROUTES.GAMEPAD]: { rail: 'input' },
  [ROUTES.MICROPHONE]: { rail: 'input' },
  [ROUTES.USB]: { rail: 'input' },
  [ROUTES.VDD_SETTINGS]: { rail: 'display' },
  [ROUTES.HDR_ENHANCED]: { rail: 'display' },
  [ROUTES.COMPONENTS]: { rail: 'system' },
  [ROUTES.DIAGNOSTICS]: { rail: 'system' },
  [ROUTES.AI_ASSISTANT]: { rail: 'system' },
  [ROUTES.WEB_TROUBLESHOOTING]: { rail: 'system' },
  [ROUTES.PANEL_SETTINGS]: { rail: 'settings', list: false },
  [ROUTES.HELP]: { rail: 'help', list: false },
}
export const RAILS = ['overview', 'stream', 'config', 'input', 'display', 'system', 'settings', 'help']

const ALIAS = { [ROUTES.CONTROLLERS]: ROUTES.GAMEPAD, [ROUTES.TOOLBOX]: ROUTES.DIAGNOSTICS }
const CATEGORY = { [ROUTES.CAT_CONFIG]: 'config', [ROUTES.CAT_STREAM]: 'stream', [ROUTES.CAT_INPUT]: 'input', [ROUTES.CAT_DISPLAY]: 'display', [ROUTES.CAT_SYSTEM]: 'system' }
const WEB_PATHS = [
  [/^\/(?:pin|clients)(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_CLIENTS],
  [/^\/apps(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_APPS],
  [/^\/config(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_CONFIG],
  [/^\/downloads(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_DOWNLOADS],
  [/^\/password(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_PASSWORD],
  [/^\/troubleshooting(?:\.html)?(?:[?#/]|$)/, ROUTES.WEB_TROUBLESHOOTING],
  [/^\/dualsense(?:\.html)?(?:[?#/]|$)/, ROUTES.GAMEPAD],
]
/** The panel destination that shows a Web UI path; the Web home is the panel's own overview. */
export const routeForWebPath = (path) => {
  const p = String(path || '/')
  for (const [re, route] of WEB_PATHS) if (re.test(p)) return route
  return ROUTES.HOME
}
/** The Web UI path of a Web page destination (null for panel pages). */
export const webPathOf = (route) => ({
  [ROUTES.WEB_CLIENTS]: '/pin', [ROUTES.WEB_APPS]: '/apps', [ROUTES.WEB_CONFIG]: '/config', [ROUTES.WEB_DOWNLOADS]: '/downloads',
  [ROUTES.WEB_PASSWORD]: '/password', [ROUTES.WEB_TROUBLESHOOTING]: '/troubleshooting',
}[route] || null)

const routes = {}
for (const rail of new Set(Object.values(PAGES).filter((p) => p.list !== false).map((p) => p.rail))) routes[`${rail}:list`] = { role: 'list', rail }
for (const [id, p] of Object.entries(PAGES)) routes[id] = { role: 'detail', rail: p.rail, list: p.list, supporting: p.supporting }

let nav = null
const shared = () => (nav ||= createNav({ routes, home: ROUTES.HOME }))

/** Navigation of the panel: one back stack for the whole window. */
export function useRouter() {
  const { t } = useI18n()
  const stack = shared()
  const navigate = (name, params) => {
    const to = ALIAS[name] || name
    if (CATEGORY[to]) return stack.selectRail(CATEGORY[to])
    if (!PAGES[to]) { console.warn(`route ${name} does not exist`); return }
    stack.go(to, params)
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
