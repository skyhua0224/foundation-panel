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

/** base64 -> bytes (for a native side that still answers the capture as text). */
const bytesOfBase64 = (text) => {
  const raw = atob(String(text || ''))
  const out = new Uint8Array(raw.length)
  for (let i = 0; i < raw.length; i += 1) out[i] = raw.charCodeAt(i)
  return out
}
/** The window capture as the native side answers it: the PNG itself (a binary IPC answer arrives as an ArrayBuffer), or text in base64. */
export const bytesOfCapture = (answer) => {
  if (typeof answer === 'string') return bytesOfBase64(answer)
  if (answer instanceof Uint8Array) return answer
  if (answer instanceof ArrayBuffer) return new Uint8Array(answer)
  return Array.isArray(answer) ? Uint8Array.from(answer) : new Uint8Array(0)
}

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

/** `promise`, or `fallback` when it takes longer than `ms`. The promise is left running: a late answer still lands in whatever keeps it (`remembered`). */
export function within(promise, ms, fallback) {
  let timer
  const late = new Promise((resolve) => { timer = setTimeout(() => resolve(fallback), ms) })
  return Promise.race([Promise.resolve(promise).catch(() => fallback), late]).finally(() => clearTimeout(timer))
}

/** Probes answered once and kept for `ttl` ms (Infinity: the session). A probe that failed is not kept, so the next question asks again. */
export function remembered(now = Date.now) {
  const kept = new Map()
  return (key, ttl, read) => {
    const hit = kept.get(key)
    if (hit && now() - hit.at < ttl) return hit.promise
    const entry = { at: now(), promise: Promise.resolve().then(read) }
    kept.set(key, entry)
    entry.promise.catch(() => { if (kept.get(key) === entry) kept.delete(key) })
    return entry.promise
  }
}
/** How long the environment waits for each probe before it goes without that line (ms). The slow ones run on and are kept when they arrive. */
export const ENV_WAIT = Object.freeze({ system: 5000, host: 1500, components: 4000, channel: 2000 })
const MINUTE = 60_000

export function createDeveloperTools({ invoke, settings, locale = 'zh-CN', version = '', hostVersion = null, base = null, kit = null, errorContext = null, simulateHost = null,
  local = globalThis.localStorage, session = globalThis.sessionStorage, reload = () => globalThis.location?.reload(), save = null, win = globalThis, wait = {} } = {}) {
  const call = (name, args) => invoke(name, args)
  const waits = { ...ENV_WAIT, ...wait }
  const keep = remembered()
  const apiBase = async () => String((await base?.()) || '').replace(/\/$/, '')
  const saveSettings = async (patch) => settings.save({ ...settings.get(), ...patch })
  const tools = {
    // What the report and the developer page show about this computer. The system, the GPUs and the WebView do not change while the panel runs: asked once. The host and the components change (an update, a driver
    // installed): kept for a minute. Nothing here may hold a window open: a probe that is slow is left out of this answer and kept when it arrives (the host version never blocks).
    environment: async () => {
      const [info, gpus, dev, host, components, channel] = await Promise.all([
        within(keep('system', Infinity, () => call('get_system_info')), waits.system, {}),
        within(keep('gpus', Infinity, () => call('get_gpus')), waits.system, []),
        within(keep('webview', Infinity, () => call('dev_environment')), waits.system, {}),
        within(keep('host', MINUTE, async () => { const v = await hostVersion?.(); if (!v) throw new Error('no host version'); return v }), waits.host, null),
        within(keep('components', MINUTE, () => call('get_component_inventory')), waits.components, []),
        within(tools.update.channel.get(), waits.channel, ''),
      ])
      return {
        product: 'Foundation Sunshine panel', version: version || info.app_version || '', channel: channel || '',
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
