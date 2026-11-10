import { sunshine } from '../tauri-adapter.js'

// The ONE place the panel's own renderer reads the host from (owner 2026-11-10). Today: a native probe of the host, and the host's HTTP API through the panel's proxy, with the last answer of
// the configuration kept so the shell draws from it at once. The dedicated channel (card panel-host-channel: hostChannel.subscribe/call) replaces the inside of this file when it lands.
const KEEP = Object.freeze({ config: 'foundation.panel.host-config' })
const storage = () => { try { return globalThis.localStorage ?? null } catch { return null } }

export const hostData = {
  /** One probe of the host by the panel's own process: it waits for the host inside the call (readiness handshake) and answers the HTTP status. */
  probe: (timeoutMs) => sunshine.probeHost(timeoutMs),
  /** The kept answer of a topic, synchronously, or null. */
  peek(topic) {
    try { const value = JSON.parse(storage()?.getItem(KEEP[topic]) || 'null'); return value && typeof value === 'object' ? value : null } catch { return null }
  },
  /** The fresh configuration of the host; kept for the next start. Rejects when the host does not answer. */
  async refreshConfig() {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const response = await fetch(`${base}/api/config`)
    if (!response.ok) throw new Error(`config ${response.status}`)
    const value = await response.json()
    try { if (value && typeof value === 'object') storage()?.setItem(KEEP.config, JSON.stringify(value)) } catch { /* storage unavailable or full */ }
    return value
  },
}
