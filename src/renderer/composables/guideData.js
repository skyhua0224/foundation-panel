import { reactive } from 'vue'
import { isSetupCompleted, mergeSetupConfig, normalizeClients, normalizeSessions, setupStateFromConfig, shouldAutoOpen } from 'foundation-ui/vue/guide.js'
import { endpointsOfConfig } from 'foundation-ui/settings'

export const GUIDE_STORAGE_KEY = 'foundation.guide.panel'
export const GUIDE_DISMISSED_KEY = 'foundation.guide.dismissed'

const okStatus = (value) => value === true || value === 'true' || value === 1 || value === '1'

/**
 * State and actions of the guided first run in the control panel. The host API is reached through the panel's local proxy; every value
 * in `state` comes from it (or from the panel's own commands). A source that cannot be read leaves its field empty, never a made-up
 * value. All outside access is injected, so the rules can be tested without a window.
 *
 * deps: { proxyUrl(): Promise<string>, fetchImpl, vddStatus(): Promise<object>, activeSessions(): Promise<array>,
 *         openUrl(url), copy(text), openApps(mode), reach (the shared reachability check, composables/useHostReach.js) }
 */
export function createGuideData(deps) {
  const fetchImpl = deps.fetchImpl || ((...args) => globalThis.fetch(...args))
  const state = reactive({
    account: { exists: null },
    setup: { completed: false, locale: '', showLanguage: true, adapters: [], displayDevices: [], adapter: '', display: '', prep: 'ensure_only_display', virtual: true },
    host: { platform: '', addresses: null },
    network: { port: 47989, upnpActive: null, upnpSaved: null, loaded: false, endpoints: '' },
    reach: deps.reach ? deps.reach.view : { report: null, status: 'idle' },
    clients: null,
    sessions: null,
    pair: { name: '' },
    apps: { scanAvailable: true },
  })

  const url = async (path) => `${await deps.proxyUrl()}${path}`
  const getJson = async (path) => {
    const response = await fetchImpl(await url(path))
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    return response.json()
  }
  const postJson = async (path, body) => {
    const response = await fetchImpl(await url(path), { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
    return { response, data: await response.json().catch(() => ({})) }
  }

  /** A host with an account answers the config with JSON; a host without one sends the page for creating it. */
  async function probeAccount() {
    try {
      const config = await getJson('/api/config')
      state.account.exists = !!config && typeof config === 'object'
      return config
    } catch {
      state.account.exists = false
      return null
    }
  }

  async function loadSetup() {
    try {
      const config = await getJson('/api/config')
      state.host.platform = config.platform || ''
      state.pair.name = config.pair_name || ''
      Object.assign(state.setup, setupStateFromConfig(config))
    } catch (error) {
      console.error('Failed to read the host configuration:', error)
    }
  }

  async function saveSetup({ locale, adapter, display, prep }) {
    if (display === 'ZakoHDR' && state.setup.virtual) {
      let status = null
      try { status = await deps.vddStatus() } catch { /* not ready */ }
      const ready = !!status && status.installed && status.running && ['ready', 'degraded'].includes(status.state)
      if (!ready) return { ok: false, vdd: false }
    }
    try {
      const config = mergeSetupConfig(await getJson('/api/config'), { locale, adapter, display, prep: prep ?? state.setup.prep })
      const { response } = await postJson('/api/config', config)
      if (!response.ok) return { ok: false, error: await response.text().catch(() => '') }
      state.setup.completed = true
      if (locale) state.setup.locale = locale
      return { ok: true }
    } catch (error) {
      return { ok: false, error: error?.message || '' }
    }
  }

  async function skipSetup() {
    const config = mergeSetupConfig(await getJson('/api/config'), {})
    const { response } = await postJson('/api/config', config)
    if (response.ok) state.setup.completed = true
  }

  async function refreshAddresses() {
    try {
      const data = await getJson('/api/network/listeners')
      state.host.addresses = okStatus(data.status) && Array.isArray(data.addresses) ? data.addresses : null
    } catch {
      state.host.addresses = null
    }
  }

  /** The listener settings: the base port and UPnP as it runs now and as it is saved (a restart applies the saved one). */
  async function loadNetwork() {
    try {
      const data = await getJson('/api/network/listeners')
      if (!okStatus(data.status)) throw new Error('listeners')
      Object.assign(state.network, { port: Number(data.saved?.port || data.active?.port) || 47989, upnpActive: !!data.active?.upnp, upnpSaved: !!data.saved?.upnp, loaded: true })
      state.host.addresses = Array.isArray(data.addresses) ? data.addresses : state.host.addresses
      try { state.network.endpoints = endpointsOfConfig(await getJson('/api/config')).join(', ') } catch { /* the field stays as it was */ }
    } catch {
      state.network.loaded = false
    }
  }

  /** UPnP on or off in the saved configuration (the host applies it after a restart). */
  async function setUpnp(on) {
    try {
      const config = { ...(await getJson('/api/config')), upnp: on ? 'enabled' : 'disabled' }
      const { response } = await postJson('/api/config', config)
      return response.ok ? { ok: true } : { ok: false, error: await response.text().catch(() => '') }
    } catch (error) {
      return { ok: false, error: error?.message || '' }
    }
  }

  /** The public endpoints (`external_endpoints`: `host[:port]` entries, comma separated) in the saved configuration; the old single `external_ip` is cleared with the first save. */
  async function setPublicEndpoints(value) {
    try {
      const config = { ...(await getJson('/api/config')), external_endpoints: value, external_ip: '' }
      const { response } = await postJson('/api/config', config)
      if (!response.ok) return { ok: false, error: await response.text().catch(() => '') }
      state.network.endpoints = value
      return { ok: true }
    } catch (error) {
      return { ok: false, error: error?.message || '' }
    }
  }

  async function restartHost() {
    try {
      const { response } = await postJson('/api/restart', {})
      return { ok: response.ok }
    } catch {
      return { ok: true } // the connection drops while the host restarts
    }
  }

  /** The QR window of the host: opened by the pair step and given back when the step goes away. */
  const pairQr = {
    start: async () => { const { data } = await postJson('/api/qr-pair', {}); if (!okStatus(data.status)) throw new Error(data.error || 'qr'); return data },
    poll: () => getJson('/api/qr-pair'),
    cancel: () => postJson('/api/qr-pair/cancel', {}),
  }

  async function refreshClients() {
    try {
      state.clients = normalizeClients(await getJson('/api/clients/list'))
    } catch {
      state.clients = null
    }
  }

  async function refreshSessions() {
    let list = null
    try { list = normalizeSessions(await getJson('/api/runtime/sessions')) } catch { /* the panel's own command below */ }
    if (!list && deps.activeSessions) {
      try { list = normalizeSessions({ sessions: await deps.activeSessions() }) } catch { list = null }
    }
    state.sessions = list
  }

  async function pair(pin, name) {
    try {
      const { data } = await postJson('/api/pin', { pin, name })
      return okStatus(data.status) ? { ok: true } : { ok: false, error: data.error || '' }
    } catch (error) {
      return { ok: false, error: error?.message || '' }
    }
  }

  /**
   * The account step: the host's own password route, as the welcome page did. The host trusts this computer without a sign-in, so the panel goes on with the
   * next step at once (the Web UI page of the host, which has to reload for the sign-in, leaves).
   */
  const createAccount = (translate) => async (form) => {
    try {
      const { data } = await postJson('/api/password', { newUsername: form.username, newPassword: form.password, confirmNewPassword: form.confirm })
      if (!okStatus(data.status)) return { ok: false, error: `${translate('error')} ${data.error || translate('server_error')}` }
      return { ok: true }
    } catch (error) {
      return { ok: false, error: `${translate('error')} ${error?.message || translate('network_error')}` }
    }
  }

  const callbacks = {
    openLink: (link) => deps.openUrl(link),
    copy: (text) => deps.copy(text),
    loadSetup,
    saveSetup,
    skipSetup,
    refreshAddresses,
    refreshClients,
    refreshSessions,
    loadNetwork,
    checkReach: (force) => deps.reach?.refresh(force),
    setUpnp,
    setPublicEndpoints,
    restartHost,
    pairQr,
    pair,
    openApps: (mode) => deps.openApps(mode),
  }
  return { state, callbacks, probeAccount, createAccount }
}

/**
 * First run on this panel: the host reports setup not completed, nobody is paired, the guide is not finished and not closed this
 * session; or the guide was started and not finished (it opens again where it was). Reads the host through the same proxy; an unreadable answer never opens the guide.
 */
export async function shouldOpenGuide({ proxyUrl, fetchImpl, progress, dismissed = false }) {
  if (dismissed || progress?.finished) return false
  const get = fetchImpl || ((...args) => globalThis.fetch(...args))
  try {
    const base = await proxyUrl()
    const configResponse = await get(`${base}/api/config`)
    if (!configResponse.ok) return false
    // a host that has no account yet answers with the page for creating it (a web page, not the configuration): the first run starts with the account step
    if (/^text\/html/i.test(configResponse.headers?.get?.('content-type') || '')) return true
    const config = await configResponse.json()
    const completed = isSetupCompleted(config)
    const resuming = !!progress && !progress.finished && progress.visited?.length > 0
    if (completed && !resuming) return false
    const clientsResponse = await get(`${base}/api/clients/list`)
    if (!clientsResponse.ok) return false
    const clients = normalizeClients(await clientsResponse.json())
    return shouldAutoOpen({ setupCompleted: completed, pairedCount: clients ? clients.length : null, finished: false, resuming })
  } catch {
    return false
  }
}
