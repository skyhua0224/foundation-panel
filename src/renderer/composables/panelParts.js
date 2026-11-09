import { nextTick, ref, watch } from 'vue'
import { rtssInstalled } from './rtssInstall.js'

// The pages another window opens in the main window (the floating toolbar's menu, toolbar.rs emits `open-panel-page`): the route, and the part
// of the page to show there (route ids of useRouter.js ROUTES, as strings: this module is loaded by tests without the UI framework). SidebarMenu navigates; the page takes its part once it shows (a section scrolls into view, a sheet opens).
export const PANEL_PAGES = Object.freeze({
  about: { route: 'about', part: '' },
  pairing: { route: 'web-clients', part: '' },
  logs: { route: 'diagnostics', part: '' },
  quality: { route: 'hdr-enhanced', part: '' },
  rtss: { route: 'rtss', part: '' },
  vdd: { route: 'vdd-settings', part: '' }, // 虚拟显示器 (toolbar, tray)
  'desktop-scale': { route: 'web-display-output', part: '' }, // 调整 DPI -> 显示与视频 › 显示器 › 显示模式 (the scaling slider)
  'live-session': { route: 'live-session', part: '' }, // 码率调整 -> 串流 › 当前串流
  shortcuts: { route: 'help', part: 'shortcuts' }, // 快捷键手册 -> 帮助与关于 › 快捷键手册
  pet: { route: 'panel-settings', part: 'pet' }, // 桌宠设置 -> 面板设置 › 桌宠
  // the rows "在 Panel 中打开" of the big screen (desktop/panelLinks.js): what a sofa does not need often lives here
  'tool-paths': { route: 'panel-settings', part: 'helpers' }, // 启动助手的工具路径 -> 面板设置 › 启动助手
  'file-sharing': { route: 'panel-settings', part: 'sharing' }, // 文件共享 -> 面板设置 › 文件共享
  'panel-advanced': { route: 'panel-settings', part: 'expert' }, // 开发者模式, 文本上下文, 日志级别 -> 面板设置 › 高级
  'display-modes': { route: 'web-display-modes', part: '' }, // 输出显示器, 分辨率与刷新率调整 -> 显示与视频
  'input-drivers': { route: 'component-status', part: '' }, // 虚拟鼠标驱动的安装与修复 -> 组件状态
})

// Pages that live in the Web frame need their address changed as well as the
// panel route. Native tray and command entries use these names so they keep a
// single main window and do not create a second HTML document.
export const PANEL_PAGE_PATHS = Object.freeze({ pairing: '/pin#pairing', logs: '/troubleshooting' })

/**
 * Take the page the toolbar or the tray left for this window (src-tauri panel_page.rs) and open it. Called on its `panel-page-pending` ping and
 * once at mount, after the listeners are registered: the host empties its slot on take, so the page opens once whichever comes first.
 */
export async function deliverPendingPanelPage(invoke, open) {
  let page = null
  try { page = await invoke('take_pending_panel_page') } catch { return null }
  if (typeof page !== 'string' || !PANEL_PAGES[page]) return null
  open(page)
  return page
}

const wanted = ref('')
/** The route of a page name, with its part requested; null for a name that is no page. */
export function requestPanelPage(page) {
  const target = PANEL_PAGES[page]
  if (!target) return null
  if (page === 'rtss' && rtssInstalled.value === false) return null // third-party component that is not installed: no page (unknown yet: the page bounces back once the read says so)
  wanted.value = target.part
  return target.route
}
/** A page takes its part when it is asked for (now or later); `take` returns false to leave it for another page. */
export function usePanelPart(part, take) {
  watch(wanted, (value) => {
    if (value !== part) return
    void nextTick(() => { if (wanted.value === part && take() !== false) wanted.value = '' })
  }, { immediate: true })
}
export const panelPartWanted = wanted
