import { createReachability } from 'foundation-ui/vue/guide.js'
import { sunshine } from '../tauri-adapter.js'

// The reachability check of the host (POST /api/network/check, GET /api/network/reachability) through the panel's proxy: one state for the home tile, the welcome flow and the remote access tutorial.
// The host does the check itself; a host without the API answers 404 and the screens say so.
let shared = null
export function useHostReach() {
  if (!shared) {
    shared = createReachability({
      request: async (path, init = {}) => {
        const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
        const response = await fetch(`${base}${path}`, { method: init.method || 'GET', headers: init.body ? { 'content-type': 'application/json' } : undefined, body: init.body ? JSON.stringify(init.body) : undefined, signal: init.signal })
        return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
      },
    })
  }
  return shared
}
