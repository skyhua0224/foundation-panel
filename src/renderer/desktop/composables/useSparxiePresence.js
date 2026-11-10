import { reactive } from 'vue'
import { sunshine } from '../../tauri-adapter.js'
import { hostBase } from '../../composables/hostBase.js'

// Whether the host has the Sparxie component installed. Absent (no route, an older host, not installed) shows nothing of it on the big screen.
const state = reactive({ loaded: false, installed: false })
let inflight = null

/** Reads /api/sparxie/status through the proxy; never throws */
export function refreshSparxiePresence() {
  if (!inflight) {
    inflight = (async () => {
      try {
        const base = await hostBase(undefined, () => sunshine.getProxyUrl())
        const response = await fetch(`${base}/api/sparxie/status`, { signal: AbortSignal.timeout(4000) })
        const body = response.ok ? await response.json() : null
        state.installed = !!body && body.installed !== false
      } catch { state.installed = false } finally { state.loaded = true; inflight = null }
    })()
  }
  return inflight
}
export const useSparxiePresence = () => state
