// The tools behind the 开发者选项 page: what the page asks for (foundation-ui's MDeveloperPage `tools`) answered from this panel. Everything the panel does not own is handed in
// (`invoke`, the settings, the telemetry kit, the host), so the pieces can be tested without a window.

export const DEV_FLAGS_KEY = 'foundation-ui.dev.flags'
export const PALETTE_KEY = 'm-picture-read-v2'
export const OVERVIEW_PREFIX = 'foundation-ui.dash2.'
/** The first-run guide, the tour that follows it, and the per-client recommendations that were dismissed. */
export const ONBOARDING_KEYS = ['foundation.coach.panel', 'foundation.guide.panel', 'foundation.guide.dismissed']
export const ONBOARDING_PREFIXES = ['foundation-ui.clientrec.']
/** What a full reset keeps: the developer mode itself (the page the person is on) and the choices of the overlays. */
export const KEEP_ON_RESET = ['foundation-ui.dev', DEV_FLAGS_KEY]

const keysOf = (storage) => { const out = []; try { for (let i = 0; i < storage.length; i += 1) out.push(storage.key(i)) } catch { /* no storage */ } return out }
const remove = (storage, keys) => { for (const key of keys) { try { storage.removeItem(key) } catch { /* not removed */ } } return keys.length }

/** How many wallpapers the palette cache holds. */
export function paletteCount(storage) {
  try { return Object.keys(JSON.parse(storage.getItem(PALETTE_KEY) || '{}')).length } catch { return 0 }
}
export const clearPalette = (storage) => remove(storage, [PALETTE_KEY])
export function resetOnboarding(local, session) {
  const keys = keysOf(local).filter((k) => ONBOARDING_KEYS.includes(k) || ONBOARDING_PREFIXES.some((p) => k.startsWith(p)))
  return remove(local, keys) + remove(session, ONBOARDING_KEYS.filter((k) => { try { return session.getItem(k) !== null } catch { return false } }))
}
export const resetOverview = (storage) => remove(storage, keysOf(storage).filter((k) => k.startsWith(OVERVIEW_PREFIX)))
export const resetStorage = (storage, keep = KEEP_ON_RESET) => remove(storage, keysOf(storage).filter((k) => !keep.includes(k)))

/** One entry of the panel log as the line the file has (the page parses these lines). */
export function logLine(entry) {
  const where = entry.file ? ` [${entry.file}${entry.line ? `:${entry.line}` : ''}]` : ''
  return `[${entry.timestamp}] [${String(entry.level || '').toUpperCase()}]${where} ${entry.message}`
}

/** The percentage the page asks for as a number the native side takes (100 = back to the real scale). */
export const percentOf = (value) => (value ? Math.round(Number(value)) : 100)

/** The text of a display: physical pixels and the scale of this window. */
export const displayText = (win = globalThis) => {
  const ratio = win.devicePixelRatio || 1
  return `${Math.round((win.screen?.width || 0) * ratio)}x${Math.round((win.screen?.height || 0) * ratio)} @${Math.round(ratio * 100)}%`
}

export function createDeveloperTools({ invoke, settings, locale = 'zh-CN', version = '', hostVersion = null, base = null, kit = null, errorContext = null, simulateHost = null,
  local = globalThis.localStorage, session = globalThis.sessionStorage, reload = () => globalThis.location?.reload(), save = null, win = globalThis } = {}) {
  const call = (name, args) => invoke(name, args)
  const apiBase = async () => String((await base?.()) || '').replace(/\/$/, '')
  const saveSettings = async (patch) => settings.save({ ...settings.get(), ...patch })
  const tools = {
    environment: async () => {
      const [info, gpus, dev, host, components] = await Promise.all([call('get_system_info').catch(() => ({})), call('get_gpus').catch(() => []), call('dev_environment').catch(() => ({})), Promise.resolve(hostVersion?.()).catch(() => null), call('get_component_inventory').catch(() => [])])
      return {
        product: 'Foundation Sunshine panel', version: version || info.app_version || '', channel: (await tools.update.channel.get().catch(() => '')) || '',
        builds: [{ name: 'panel', sha: info.app_version || version || '' }], components: Array.isArray(components) ? components : [], host: host || '', os: info.os_version || '', arch: info.arch || dev.target || '', gpu: Array.isArray(gpus) ? gpus.join(', ') : '',
        webview: dev.webview || '', display: displayText(win), locale,
      }
    },
    debugOutput: { get: async () => Boolean(settings.get().devMode), set: (value) => saveSettings({ devMode: Boolean(value) }) },
    logLevel: { get: async () => settings.get().logLevel || 'info', set: (value) => saveSettings({ logLevel: String(value) }) },
    logText: async () => (await call('get_all_logs')).map(logLine).join('\n'),
    window: {
      get: async () => { const state = await call('dev_get_state'); return { ...state, simulated: 0 } },
      setCover: (mode) => call('dev_set_cover_mode', { mode }),
      setCloak: (on) => call('dev_set_cold_cloak', { on: Boolean(on) }),
      simulateDpi: (percent) => call('dev_simulate_dpi', { percent: percentOf(percent) }),
    },
    webview: {
      devtools: () => call('dev_open_devtools'),
      reload: () => reload(),
      clearCache: () => call('dev_clear_webview_cache'),
    },
    data: {
      paletteCount: async () => paletteCount(local),
      clearPalette: async () => { clearPalette(local) },
      resetOnboarding: async () => { resetOnboarding(local, session) },
      resetOverview: async () => { resetOverview(local); reload() },
      // the settings file, the storage of this window (except the developer mode), and the choices of the developer page; then the window starts again
      resetAll: async () => { await call('dev_reset_desktop_settings'); resetStorage(local); resetStorage(session, []); reload() },
    },
    host: {
      reach: async () => {
        const started = Date.now()
        const response = await fetch(`${await apiBase()}/api/network/reachability`, { cache: 'no-store' })
        const text = await response.text()
        let body = text
        try { body = JSON.parse(text) } catch { /* shown as text */ }
        return { status: response.status, ms: Date.now() - started, body }
      },
      simulate: async (kind) => simulateHost?.(kind || ''),
    },
    telemetry: {
      state: async () => ({ consent: kit?.consent?.() === 'on', installId: '', queue: kit?.queue?.() ?? 0 }),
      // the document the report sends, built the way a real one is, with the error handlers' context: personal data is removed by the same code
      preview: async () => kit.document(kit.report(new Error('Developer page: preview of an error report'), { kind: 'manual', fatal: false, context: errorContext?.() || {} }), { title: 'Preview', description: 'Preview from the developer page. Not sent.' }),
      test: async () => {
        const doc = kit.document(kit.report(new Error('Developer page: test event'), { kind: 'manual', fatal: false, context: errorContext?.() || {} }), { title: 'Test event', description: 'Sent from the developer page.' })
        const answer = await kit.send(doc)
        if (answer.state !== 'sent') throw new Error(answer.reason || answer.state)
        return answer
      },
    },
    update: {
      channel: { get: async () => ((await call('get_include_prerelease_preference')) ? 'prerelease' : 'stable'), set: (value) => call('set_include_prerelease_preference', { include: value === 'prerelease' }) },
      check: async () => { const channel = await tools.update.channel.get(); const info = await call('check_for_updates_for_channel', { channel }); return info ? { latest: Boolean(info.is_latest), version: info.version || '' } : { latest: true, version } },
    },
  }
  if (save) tools.exportText = save
  return tools
}
