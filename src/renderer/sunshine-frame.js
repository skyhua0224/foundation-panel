import '../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp } from 'vue'
import SunshineFrame from './components/SunshineFrame.vue'
import { invoke } from '@tauri-apps/api/core'
import { followPanelSurface } from './composables/panelSurface.js'
import { startWebPrewarm, createHiddenFrameLoader } from './composables/webPrewarm.js'
import { startSlowCallLog, installFaultLog } from './composables/slowCalls.js'
import { followDpiCover } from './composables/dpiSettle.js'
import { listen } from '@tauri-apps/api/event'
import { configureInvokeTiming, invokePending } from './tauri-core-timed.js'
import 'foundation-ui/all.css'  // Material 3 Expressive tokens and components
import './styles/pressed.css'  // pressed state at pointerdown for the controls that had none
import './styles/m3e-compat.less'  // restyles the pages that still use their own markup
// 导入 Tauri polyfill
import './tauri-polyfill.js'
import { getDefaultLocale, setLocale } from '../i18n/index.js'
import { i18nReady } from './desktop/i18n/index.js'
import { configureDevBackend } from 'foundation-ui/vue'
import { applyDevBackend } from './devBackendConfig.js'

// 初始化语言
setLocale(getDefaultLocale())

// the GitHub flows of developer mode (sign-in, Issue 与 PR, feedback under the person's name) have their backend from here on
applyDevBackend(configureDevBackend)

const app = createApp(SunshineFrame)
// the language of the window is read before the first view is drawn (a language other than English and Chinese is a file of its own)

// The first view is in the DOM: the native side may show the window now (WebView2 paints over the window's surface colour; it need not wait for
// the load event, which waits for every frame inside the page) and learns the theme's surface colour for its first frame
const afterFirstView = () => {
  invoke('panel_content_ready').catch(() => {})
  // the window may be cloaked until the first frame is painted: tell the host when it is
  try {
    new PerformanceObserver((list, observer) => {
      if (list.getEntries().some((entry) => entry.name === 'first-contentful-paint')) { observer.disconnect(); invoke('panel_first_paint').catch(() => {}) }
    }).observe({ type: 'paint', buffered: true })
  } catch { invoke('panel_first_paint').catch(() => {}) }
  followPanelSurface(invoke)
  followPageActivity(invoke)
  followDpiCover({ listen, invoke })  // the native cover after a change of scale fades out when the layout matches the window
  prewarmWebPages()
  // the calls, frames and timers that take long are in the panel's log, with what was waiting when a timer came late
  const report = (kind, name, ms, detail) => { if (name !== 'frontend_slow') invoke('frontend_slow', { kind, name, ms, detail }).catch(() => {}) }
  configureInvokeTiming({ report })
  startSlowCallLog(report, window, invokePending)
  installFaultLog(report, window)
}
i18nReady.then(() => { app.mount('#app'); queueMicrotask(afterFirstView) })

// The page is being used: the native side takes no snapshot of it then (at most one report in 250 ms)
function followPageActivity(invoke) {
  let last = 0
  const note = () => {
    const now = performance.now()
    if (now - last < 250) return
    last = now
    invoke('panel_activity').catch(() => {})
  }
  for (const type of ['pointerdown', 'pointermove', 'keydown', 'wheel']) window.addEventListener(type, note, { capture: true, passive: true })
}

// The Web UI pages load their scripts on the first visit (up to 1.6 s on the Windows test panel): load them once in a hidden frame while nobody uses the window
function prewarmWebPages() {
  let lastInput = performance.now()
  for (const type of ['pointerdown', 'pointermove', 'keydown', 'wheel']) window.addEventListener(type, () => { lastInput = performance.now() }, { capture: true, passive: true })
  invoke('wait_for_proxy_ready').then((base) => {
    const frame = createHiddenFrameLoader()
    startWebPrewarm({
      base: String(base).replace(/\/$/, ''),
      load: frame.load,
      visible: () => document.visibilityState === 'visible',
      idleFor: () => performance.now() - lastInput,
      done: frame.remove,
      sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
    })
  }).catch(() => {})
}
