import { nextTick, ref, watch } from 'vue'

// The pages another window opens in the main window (the floating toolbar's menu, toolbar.rs emits `open-panel-page`): the route, and the part
// of the page to show there (route ids of useRouter.js ROUTES, as strings: this module is loaded by tests without the UI framework). SidebarMenu navigates; the page takes its part once it shows (a section scrolls into view, a sheet opens).
export const PANEL_PAGES = Object.freeze({
  'desktop-scale': { route: 'vdd-settings', part: 'desktop-scale' }, // 调整 DPI -> 显示与视频 › 虚拟显示器 › 桌面缩放
  'live-session': { route: 'live-session', part: '' }, // 码率调整 -> 串流 › 当前串流
  shortcuts: { route: 'help', part: 'shortcuts' }, // 快捷键手册 -> 帮助与关于 › 快捷键手册
  pet: { route: 'panel-settings', part: 'pet' }, // 桌宠设置 -> 面板设置 › 桌宠
})

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
