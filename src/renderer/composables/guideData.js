import { reactive } from 'vue'
import { checkPortsViaDoctor, createHostSaver, deviceId as guideDeviceId, guideEntry, hostProgressOf, isSetupCompleted, mergeSetupConfig, normalizeClients, normalizeSessions, setupStateFromConfig } from 'foundation-ui/vue/guide.js'
import { endpointsOfConfig } from 'foundation-ui/settings'
import { restoreDefaultBuiltInApps } from 'foundation-ui/apps'

export const GUIDE_STORAGE_KEY = 'foundation.guide.panel'
export const GUIDE_DISMISSED_KEY = 'foundation.guide.dismissed'

const okStatus = (value) => value === true || value === 'true' || value === 1 || value === '1'

/**
 * State and actions of the guided first run in the control panel. The host API is reached through the panel's local proxy; every value
 * in `state` comes from it (or from the panel's own commands). A source that cannot be read leaves its field empty, never a made-up
 * value. All outside access is injected, so the rules can be tested without a window.
 *
 * deps: { proxyUrl(): Promise<string>, fetchImpl, vddStatus(): Promise<object>, activeSessions(): Promise<array>,
 *         openUrl(url), copy(text), locale() (the language of the guide: the 体检 words its answer in it), openApps(mode), openDiagnostics() (the address step's link to 体检), reach (the shared reachability check, composables/useHostReach.js) }
 */
export function createGuideData(deps) {
  const fetchImpl = deps.fetchImpl || ((...args) => globalThis.fetch(...args))
  const state = reactive({
    account: { exists: null },
    setup: { completed: false, locale: '', showLanguage: true, adapters: [], displayDevices: [], adapter: '', display: '', prep: 'ensure_only_display', virtual: true },
    host: { platform: '', name: '', addresses: null },
    network: { port: 47989, activePort: 0, upnpActive: null, upnpSaved: null, loaded: false, endpoints: '' },
    webAccess: { scope: 'lan', bind: '', active: null, saved: null },
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

  /** What the host remembers of the guide (the steps done on any device, finished, closed, the device running it). */
  async function loadGuideHost() {
    try { return hostProgressOf(await getJson('/api/config')) } catch { return hostProgressOf(null) }
  }
  const saveGuideProgress = createHostSaver({ getJson, postJson: async (path, body) => (await postJson(path, body)).data })

  async function saveSetup({ locale, adapter, display, prep, hdr }) {
    if (display === 'ZakoHDR' && state.setup.virtual) {
      let status = null
      try { status = await deps.vddStatus() } catch { /* not ready */ }
      const ready = !!status && status.installed && status.running && ['ready', 'degraded'].includes(status.state)
      if (!ready) return { ok: false, vdd: false }
    }
    try {
      const config = mergeSetupConfig(await getJson('/api/config'), { locale, adapter, display, prep: prep ?? state.setup.prep, hdr })
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
      Object.assign(state.network, { port: Number(data.saved?.port || data.active?.port) || 47989, activePort: Number(data.active?.port) || 0, upnpActive: !!data.active?.upnp, upnpSaved: !!data.saved?.upnp, loaded: true })
      state.host.addresses = Array.isArray(data.addresses) ? data.addresses : state.host.addresses
      try {
        const config = await getJson('/api/config')
        state.network.endpoints = endpointsOfConfig(config).join(', ')
        state.host.name = String(config.sunshine_name || '') // the name clients list (the address step)
      } catch { /* the fields stay as they were */ }
    } catch {
      state.network.loaded = false
    }
  }

  /** Who may open the Web UI and the bind address as saved, and what the host runs now (the choice is applied after a restart). */
  async function loadWebAccess() {
    try {
      const [listeners, config] = await Promise.all([getJson('/api/network/listeners'), getJson('/api/config')])
      Object.assign(state.webAccess, { scope: config.origin_web_ui_allowed || 'lan', bind: config.bind_address || '', active: listeners.active || null, saved: listeners.saved || null })
    } catch { /* the step shows the default and the save reports what the host says */ }
  }

  /** origin_web_ui_allowed and/or bind_address in the saved configuration. */
  async function saveWebAccess(extra) {
    try {
      const { response } = await postJson('/api/config', { ...(await getJson('/api/config')), ...extra })
      if (!response.ok) return { ok: false, error: await response.text().catch(() => '') }
      Object.assign(state.webAccess, { scope: extra.origin_web_ui_allowed ?? state.webAccess.scope, bind: extra.bind_address ?? state.webAccess.bind })
      return { ok: true }
    } catch (error) {
      return { ok: false, error: error?.message || '' }
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

  /** The base port (`port`) in the saved configuration; the host listens on it after a restart. */
  async function setPort(port) {
    try {
      const { response, data } = await postJson('/api/config', { ...(await getJson('/api/config')), port })
      if (!response.ok) return { ok: false, error: typeof data.error === 'string' ? data.error : '' } // postJson has read the body
      state.network.port = Number(port)
      return { ok: true }
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

  /** A cheap read of the host, the proof of a restart: the start time of its process (`/api/version`); a host without it answers 404/444, then any answer of the listener route is "it is up". The proxy answers 503 while the core is away. */
  async function probeHost() {
    try {
      return { ok: true, startedAt: String((await getJson('/api/version'))?.process_started_at || '') }
    } catch (error) {
      if (/HTTP (404|444)$/.test(String(error?.message))) { try { await getJson('/api/network/listeners'); return { ok: true, startedAt: '' } } catch { return { ok: false } } }
      return { ok: false }
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

  /** The scan of the game libraries on the host (POST /api/apps/scan): { ok, games: [{ key, name, platform, cmd, 'working-dir' }] }, or why not. */
  async function scanGames(sources) {
    try {
      const { response, data } = await postJson('/api/apps/scan', { sources })
      if (response.status === 404 || response.status === 501) return { ok: false, unsupported: true }
      if (response.status === 409) return { ok: false, busy: true }
      if (!response.ok || !Array.isArray(data.games)) return { ok: false, error: typeof data.error === 'string' ? data.error : '' }
      return { ok: true, warnings: Array.isArray(data.warnings) ? data.warnings : [], games: data.games.map((g, i) => {
        const launches = Array.isArray(g.launches) ? g.launches : []
        const mode = g.platform === 'sparxie' && launches.some((choice) => choice.id === 'touch') ? 'touch' : (launches.find((choice) => choice.launch)?.id || '')
        return { key: g.install_dir || g.id || `${g.name}|${i}`, name: g.name, platform: g.platform || '', cmd: g.launch || g.exe || '', 'working-dir': g.install_dir || '', exe: g.exe || '', profile: g.profile || '', region: g.region || '', launches, mode, 'image-path': g.cover_hint || '' }
      }) }
    } catch { return { ok: false } }
  }

  /** Adds the chosen games (or the one typed by hand) to the apps list: a name, the program and its folder. */
  async function addGames(games) {
    try {
      const sparxie = games.filter((g) => g.profile && (g.mode === 'touch' || g.mode === 'pc' || g.platform === 'sparxie'))
      const plain = games.filter((g) => !sparxie.includes(g))
      for (const game of sparxie) {
        const touch = game.mode !== 'pc'
        const { response, data } = touch
          ? await postJson('/api/sparxie/import', { ids: [game.profile] })
          : await postJson('/api/sparxie/apps', { path: game.exe || game.cmd, name: game.name, mode: 'pc', region: game.region || '' })
        if (!response.ok || data?.ok === false) return { ok: false, error: data?.error || '' }
      }
      if (plain.length) {
        const list = (await getJson('/api/apps')).apps || []
        const added = plain.map((g) => ({ name: g.name, cmd: g.cmd || '', 'working-dir': g['working-dir'] || '', 'image-path': g['image-path'] || '' }))
        const { response } = await postJson('/api/apps', { apps: [...list, ...added], editApp: null })
        if (!response.ok) return { ok: false }
      }
      return { ok: true, added: games.length }
    } catch (error) { return { ok: false, error: error?.message || '' } }
  }

  /**
   * 「还原内置应用」 on the apps step, in the flow: the same restore as the Web UI's apps page (foundation-ui/apps) over GET and POST /api/apps; the guide shows the result where the button is.
   * { ok, result: 'done' | 'none' } or { ok: false, error }. The names of the default menu commands stay English (the panel has no Web UI language pack).
   */
  async function restoreApps() {
    try {
      const [{ apps = [] }, config] = await Promise.all([getJson('/api/apps'), getJson('/api/config')])
      const result = restoreDefaultBuiltInApps(apps, config.platform || 'windows')
      if (result.changed === 0) return { ok: true, result: 'none' }
      const { response } = await postJson('/api/apps', { apps: result.apps, editApp: null })
      return response.ok ? { ok: true, result: 'done' } : { ok: false }
    } catch (error) { return { ok: false, error: error?.message || '' } }
  }

  const callbacks = {
    scanGames,
    restoreApps,
    addGames,
    openTools: () => deps.openTools?.(),
    openLink: (link) => deps.openUrl(link),
    copy: (text) => deps.copy(text),
    loadSetup,
    saveGuideProgress,
    saveSetup,
    skipSetup,
    refreshAddresses,
    refreshClients,
    refreshSessions,
    loadNetwork,
    loadWebAccess,
    saveWebAccess,
    checkReach: (force, options) => deps.reach?.refresh(force, options),
    setUpnp,
    setPort,
    openDiagnostics: () => deps.openDiagnostics?.(),
    // the port check of the address step is the host's own 体检 (network group), the answer of the diagnostics page
    checkPorts: () => checkPortsViaDoctor(async (path) => {
      try { const response = await fetchImpl(await url(path)); return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) } } catch { return { ok: false, status: 0, json: null } }
    }, /^zh/i.test(String(deps.locale?.() || '')) ? 'zh' : 'en'),
    setPublicEndpoints,
    restartHost,
    probeHost,
    pairQr,
    pair,
    openApps: (mode) => deps.openApps(mode),
  }
  return { state, callbacks, probeAccount, createAccount, loadGuideHost }
}

/**
 * What the guide does when the panel starts: a first run (the host reports setup not completed, nobody is paired, the guide is not finished and not closed this session, or it was
 * started and not finished: it opens again where it was), or, for a host that is set up, once per new version of the panel (`version`; the version is stored when the guide is
 * closed, skipped or finished). Reads the host through the same proxy; an unreadable answer never opens the guide. Without a version only a first run opens it.
 */
export async function shouldOpenGuide({ proxyUrl, fetchImpl, progress, dismissed = false, version = '' }) {
  if (dismissed || (progress?.finished && !version)) return false
  const get = fetchImpl || ((...args) => globalThis.fetch(...args))
  try {
    const base = await proxyUrl()
    const configResponse = await get(`${base}/api/config`)
    if (!configResponse.ok) return false
    // a host that has no account yet answers with the page for creating it (a web page, not the configuration): the first run starts with the account step
    if (/^text\/html/i.test(configResponse.headers?.get?.('content-type') || '')) return true
    const config = await configResponse.json()
    const completed = isSetupCompleted(config)
    const host = hostProgressOf(config)
    const resuming = !!progress && !progress.finished && !host.finished && (progress.visited?.length > 0 || progress.done?.length > 0 || host.done.length > 0)
    if (completed && !resuming && !version) return false
    const clientsResponse = await get(`${base}/api/clients/list`)
    if (!clientsResponse.ok) return false
    const clients = normalizeClients(await clientsResponse.json())
    return !!guideEntry({ progress, version, setupCompleted: completed, pairedCount: clients ? clients.length : null, configExists: true, host, device: guideDeviceId() })
  } catch {
    return false
  }
}
