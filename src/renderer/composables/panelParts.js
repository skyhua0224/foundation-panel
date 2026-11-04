import { nextTick, ref, watch } from 'vue'

// The pages another window opens in the main window (the floating toolbar's menu, toolbar.rs emits `open-panel-page`): the route, and the part
// of the page to show there (route ids of useRouter.js ROUTES, as strings: this module is loaded by tests without the UI framework). SidebarMenu navigates; the page takes its part once it shows (a section scrolls into view, a sheet opens).
export const PANEL_PAGES = Object.freeze({
  vdd: { route: 'vdd-settings', part: '' }, // 虚拟显示器 (toolbar, tray)
  'desktop-scale': { route: 'vdd-settings', part: 'desktop-scale' }, // 调整 DPI -> 显示与视频 › 虚拟显示器 › 桌面缩放
  'live-session': { route: 'live-session', part: '' }, // 码率调整 -> 串流 › 当前串流
  shortcuts: { route: 'help', part: 'shortcuts' }, // 快捷键手册 -> 帮助与关于 › 快捷键手册
  pet: { route: 'panel-settings', part: 'pet' }, // 桌宠设置 -> 面板设置 › 桌宠
})

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
