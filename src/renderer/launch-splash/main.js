import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
// The launch splash's own window (src-tauri/src/launch_splash): a click-through window on the display the stream captures. Rust sends the frame (the launch's
// name, stage, notice and hero) and the end; this draws MLaunchSplash from it. A page that loads after the first frame asks for the current one.
import { createApp, defineComponent, h, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import 'foundation-ui/all.css'
import MLaunchSplash from 'foundation-ui/vue/MLaunchSplash.vue'  // the one component, not the package's index: the page loads in the tray state
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
    // Rust leaves the picture (megabytes) out of the events after the first one of a launch: the page keeps the one it has
    const set = (next) => {
      clearTimeout(timer); leaving.value = false
      if (!next.art && frame.value && frame.value.id === next.id) next = { ...next, art: frame.value.art }
      const first = !frame.value || frame.value.id !== next.id
      frame.value = next
      if (first) ready(next)
    }
    // the window is ready for a launch when its page is drawn and its picture decoded: the first show then paints at once
    const ready = async (next) => {
      await nextTick()
      if (next.art) { try { const img = new Image(); img.src = next.art; await img.decode() } catch { /* the picture is optional */ } }
      invoke('launch_splash_prepared', { id: next.id }).catch(() => {})
    }
    // the window is on the display: two animation frames later the first frame is painted there, and the host starts the game
    const present = async (id) => {
      await nextTick()
      requestAnimationFrame(() => requestAnimationFrame(() => { invoke('launch_splash_painted', { id }).catch(() => {}) }))
    }
    const leave = () => { leaving.value = true; clearTimeout(timer); timer = setTimeout(() => { if (leaving.value) frame.value = null }, FADE_MS) }
    // for the harness and for a person at the devtools: the same two calls Rust makes
    window.__launchSplash = { set, leave, present }
    const off = []
    onMounted(async () => {
      try {
        off.push(await listen('launch-splash:frame', (event) => set(event.payload)))
        off.push(await listen('launch-splash:leave', leave))
        off.push(await listen('launch-splash:present', (event) => present(event.payload)))
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
