import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
// The launch splash's own window (src-tauri/src/launch_splash): a click-through window on the display the stream captures. Rust sends the frame (the launch's
// name, stage, notice and hero) and the end; this draws MLaunchSplash from it. A page that loads after the first frame asks for the current one.
import { createApp, defineComponent, h, onBeforeUnmount, onMounted, ref } from 'vue'
import 'foundation-ui/all.css'
import { MLaunchSplash } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// the Panel's language when it has one, else the system's; the splash only tells Chinese from the rest (foundation-ui/launchsplash/texts.js)
const localeOf = () => { try { return localStorage.getItem('language') || navigator.language || 'zh' } catch { return navigator.language || 'zh' } }
const FADE_MS = 200

const Splash = defineComponent({
  setup() {
    const frame = ref(null)
    const leaving = ref(false)
    let timer = 0
    const set = (next) => { clearTimeout(timer); leaving.value = false; frame.value = next }
    const leave = () => { leaving.value = true; clearTimeout(timer); timer = setTimeout(() => { if (leaving.value) frame.value = null }, FADE_MS) }
    // for the harness and for a person at the devtools: the same two calls Rust makes
    window.__launchSplash = { set, leave }
    const off = []
    onMounted(async () => {
      try {
        off.push(await listen('launch-splash:frame', (event) => set(event.payload)))
        off.push(await listen('launch-splash:leave', leave))
        const current = await invoke('launch_splash_frame')
        if (current && !frame.value) set(current)
      } catch { /* not inside the Panel: the harness drives window.__launchSplash */ }
    })
    onBeforeUnmount(() => { clearTimeout(timer); off.forEach((fn) => fn()) })
    return () => (frame.value
      ? h(MLaunchSplash, { key: frame.value.id, name: frame.value.appName, art: frame.value.art, stage: frame.value.stage, notice: frame.value.notice, locale: localeOf(), leaving: leaving.value })
      : null)
  },
})

createApp(Splash).mount('#app')
