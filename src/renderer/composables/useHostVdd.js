import { ref } from 'vue'
import { createHttpVddAdapter, createVddStore, vddStatusOf } from 'foundation-ui/vue'
import { sunshine, vdd as vddApi } from '../tauri-adapter.js'

// The host's virtual display keys (display handling, shared display, always on, zero-copy handoff, Vulkan HDR bridge ...) for the control panel's own page: the shared store of foundation-ui
// over the host API through the proxy, saving each change at once, with the tray-owned keys written by the panel's commands. One store for the page.
let shared = null
export function useHostVdd() {
  if (shared) return shared
  const base = ref('')
  const urlOf = async () => { if (!base.value) base.value = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); return base.value }
  const fetchJson = async (path, init) => (await fetch(`${await urlOf()}${path}`, init)).json()
  const adapter = createHttpVddAdapter({
    fetchJson,
    setProtected: async (key, on) => { const r = key === 'vdd_keep_enabled' ? await vddApi.setKeepEnabled(on) : await vddApi.setHeadlessCreateEnabled(on); if (r && r.success === false) throw new Error(r.error || 'failed') },
    status: async () => vddStatusOf(await fetchJson('/api/vdd/status')),
  })
  const store = createVddStore(adapter, { autosave: true })
  shared = { store, adapter, fetchJson, load: async () => { try { await adapter.load() } catch { /* the page shows the defaults */ } await store.refresh() } }
  return shared
}
