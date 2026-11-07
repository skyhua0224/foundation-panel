import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
// The latency test clock's own window, full screen on the display the stream captures (useTools.openLatencyClockOnStream): the face alone, ?mode=dark|light,
// ?locale=zh|en; Esc closes the window.
import { createApp, h } from 'vue'
import 'foundation-ui/all.css'
import { MLatencyClock } from 'foundation-ui/vue'

const q = new URLSearchParams(location.search)
const close = async () => {
  try { const { getCurrentWindow } = await import('@tauri-apps/api/window'); await getCurrentWindow().close() } catch { window.close() }
}
createApp({ render: () => h(MLatencyClock, { standalone: true, mode: q.get('mode') || 'dark', locale: q.get('locale') || 'zh', onClose: close }) }).mount('#app')
