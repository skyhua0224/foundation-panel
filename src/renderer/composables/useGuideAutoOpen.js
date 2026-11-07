import { onBeforeUnmount, onMounted } from 'vue'
import { readProgress } from 'foundation-ui/vue/guide.js'
import { sunshine } from '../tauri-adapter.js'
import { panelVersion } from './panelVersion.js'
import { GUIDE_DISMISSED_KEY, GUIDE_STORAGE_KEY, shouldOpenGuide } from './guideData.js'

/**
 * Opens the guide on a first run (see shouldOpenGuide). The host may still be starting when the panel appears, so it asks once
 * now and once more after a few seconds; `open` is only called when the answer is a clear yes.
 */
export function useGuideAutoOpen(open, { retryMs = 8000 } = {}) {
  let timer = 0
  let opened = false
  const ask = async () => {
    if (opened) return
    let dismissed = false
    try { dismissed = window.sessionStorage.getItem(GUIDE_DISMISSED_KEY) === '1' } catch { /* not remembered */ }
    if (await shouldOpenGuide({ proxyUrl: () => sunshine.getProxyUrl(), progress: readProgress(null, GUIDE_STORAGE_KEY), dismissed, version: await panelVersion() })) {
      opened = true
      open()
    }
  }
  onMounted(() => { void ask(); timer = setTimeout(ask, retryMs) })
  onBeforeUnmount(() => clearTimeout(timer))
}
