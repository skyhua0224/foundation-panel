import { createReachability } from 'foundation-ui/vue/guide.js'
import { sunshine } from '../tauri-adapter.js'

// The reachability check of the host (GET /api/network/reachability) through the panel's proxy: one state for the home tile, the welcome flow and the remote access tutorial.
// The host does the check itself; a host without the API answers 404 and the screens say so.
let shared = null
export function useHostReach() {
  if (!shared) {
    shared = createReachability({
      request: async (path) => {
        const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
        const response = await fetch(`${base}${path}`)
        return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
      },
    })
  }
  return shared
}
