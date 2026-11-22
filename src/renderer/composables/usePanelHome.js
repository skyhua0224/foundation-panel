import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { sunshine } from '@/tauri-adapter.js'
import { pickHardware } from './hardwareReadings.js'
import { parseSessionLog, parseErrorLog, realSession } from 'foundation-ui/vue'
import { readLogResponse } from 'foundation-ui/diagnostics'
import { pollDelay, boostUntilFor, hasStartingSession } from './pollPlan.js'
import { setVisibleInterval, clearVisibleInterval } from './pageVisibility.js'

const HISTORY = 60
const safe = async (fn, fallback = null) => { try { return await fn() } catch { return fallback } }

// The last data of the overview (stale-while-revalidate): a visit after the first starts from it, so the tiles show their numbers at once and the
// reads below replace them in place. The first visit waited for the slowest of its reads (the driver probe, 0.5 s) before anything showed.
// The live state is part of it: the session (who streams, with which client), the perf reading its clock counts from, the curves and the CPU / GPU load. A visit while a stream runs draws
// the streaming overview in its first frame; the tiles were once drawn idle and then flipped, because the session was the one thing this list did not keep.
let remembered = null
const REMEMBER = ['version', 'startMs', 'gpus', 'config', 'memory', 'vdd', 'vddSettings', 'logs', 'logText', 'clients', 'clientsTotal', 'apps', 'appsTotal', 'recent', 'errorLog', 'sessions', 'perf', 'perfAt']
const SERIES_KEYS = ['bitrate', 'fps', 'latency', 'cpu', 'gpu', 'memory', 'loss', 'dropped']
// the first reads of a visit that has nothing kept may take this long before the overview is drawn anyway (its skeleton is up until then)
export const SETTLE_CAP_MS = 1500
// what the overview's own readers keep between visits (the system information of the host, the DualSense keys of the stream): the same rule, read at the start of a visit
export const seen = { nics: null, ds5Keys: null }

/**
 * Live data of the overview tiles. Every value comes from a real source (host API through the local proxy, Tauri commands);
 * a source that fails leaves its field empty and the tile hides or shows its unavailable state. Samples for the sparklines
 * are collected here while the page is open.
 */
export function usePanelHome() {
  const state = reactive({
    version: '', startMs: 0, now: Date.now(), sessions: [], clientsTotal: null, appsTotal: null,
    perf: null, perfAt: 0, config: null, memory: null, gpus: [], vdd: null, vddSettings: null, logs: null, loaded: false,
    // CPU and GPU load come from native Windows counters (system_load); temperatures only exist through HWiNFO, if it is running
    sys: { cpu: null, gpu: null },
    hw: { available: false, cpuTemp: null, gpuTemp: null },
    series: { bitrate: [], fps: [], latency: [], cpu: [], gpu: [], memory: [], loss: [], dropped: [] },
    logText: '',
    clients: null, apps: null, recent: null, errorLog: null,
    // false until what the overview is drawn from is known: the session, the paired clients and the host's reads. From the kept data of an earlier visit it is true at once.
    settled: false,
  })
  const proxy = ref('')
  if (remembered) {
    for (const key of REMEMBER) state[key] = remembered.state[key]
    for (const key of SERIES_KEYS) state.series[key] = [...remembered.series[key]]
    state.sys.cpu = remembered.sys.cpu
    state.sys.gpu = remembered.sys.gpu
    state.hw.available = remembered.hwAvailable
    state.hw.cpuTemp = remembered.cpuTemp
    state.hw.gpuTemp = remembered.gpuTemp
    proxy.value = remembered.proxy
    state.loaded = true
    state.settled = true
  }
  const remember = () => {
    remembered = {
      state: Object.fromEntries(REMEMBER.map((key) => [key, state[key]])), series: Object.fromEntries(SERIES_KEYS.map((key) => [key, [...state.series[key]]])), sys: { cpu: state.sys.cpu, gpu: state.sys.gpu },
      hwAvailable: state.hw.available, cpuTemp: state.hw.cpuTemp, gpuTemp: state.hw.gpuTemp, proxy: proxy.value,
    }
  }
  const fetchJson = async (path, init) => {
    const r = await fetch(`${proxy.value}${path}`, init)
    if (!r.ok) throw new Error(String(r.status))
    return r.json()
  }
  // the host log is read as bytes and decoded line by line (UTF-8, GBK of an older log, UTF-16), never as r.text(), which turns legacy bytes into U+FFFD
  const fetchText = async (path, init) => {
    const r = await fetch(`${proxy.value}${path}`, init)
    if (!r.ok) throw new Error(String(r.status))
    return readLogResponse(r)
  }
  const push = (list, v) => { if (typeof v === 'number' && Number.isFinite(v)) { list.push(v); if (list.length > HISTORY) list.shift() } }
  const memoryShare = (m) => (m?.system_total_memory ? Math.min(1, (m.system_total_memory - (m.system_available_memory || 0)) / m.system_total_memory) : null)

  async function fast() {
    state.now = Date.now()
    const body = await safe(() => fetchJson('/api/runtime/sessions'))
    const sessions = (body && Array.isArray(body.sessions) ? body.sessions : (await safe(() => sunshine.getActiveSessions(), []))) || []
    if (hasStartingSession(sessions)) boostUntil = boostUntilFor(Date.now(), 'starting', boostUntil)
    const s = sessions[0]
    if (s) {
      // the session and its perf reading are taken together: a stream that starts is one change of the page, not the session first and its numbers a moment later
      const perf = await safe(() => fetchJson('/api/perf/current'))
      push(state.series.loss, s.packet_loss); push(state.series.dropped, s.dropped_frames)
      const psess = perf?.sessions?.find((x) => x.active) || perf?.sessions?.[0]
      // the measured bitrate comes in sessions[] or in perf/current (a newer host); `bitrate` stays the session's budget
      if (!Number.isFinite(s.measured_bitrate_kbps) && Number.isFinite(psess?.measured_bitrate_kbps)) s.measured_bitrate_kbps = psess.measured_bitrate_kbps
      push(state.series.bitrate, Number.isFinite(s.measured_bitrate_kbps) ? s.measured_bitrate_kbps : s.bitrate)
      const ps = perf?.sessions?.find((x) => x.active) || perf?.sessions?.[0]
      push(state.series.latency, ps?.host_latency?.avg_ms)
      push(state.series.fps, ps?.frame_rates?.encoded?.fps ?? ps?.host_latency?.recent_fps ?? s.fps)
      state.sessions = sessions
      state.perf = perf
      state.perfAt = Date.now() // the instant the host's uptime_ms was read: the session clock counts from it (useSessionClock)
    } else { state.sessions = sessions; state.perf = null; state.series.bitrate = []; state.series.fps = []; state.series.latency = []; state.series.loss = []; state.series.dropped = [] }
    const load = await safe(() => invoke('system_load'))
    if (load) {
      state.sys.cpu = typeof load.cpu_percent === 'number' ? load.cpu_percent : null
      state.sys.gpu = typeof load.gpu_percent === 'number' ? load.gpu_percent : null
      push(state.series.cpu, state.sys.cpu); push(state.series.gpu, state.sys.gpu)
    }
    push(state.series.memory, memoryShare(state.memory))
    remember()
  }
  async function slow() {
    const [clients, apps, config, memory, vdd, logs] = await Promise.all([
      safe(() => fetchJson('/api/clients/list')),
      safe(() => fetchJson('/api/apps')),
      safe(() => fetchJson('/api/config')),
      safe(() => invoke('get_process_memory_info')),
      safe(() => invoke('get_vdd_status')),
      safe(() => fetchText('/api/logs', { headers: { 'X-Log-Offset': '0' } })),
    ])
    if (clients) { state.clients = clients.named_certs || []; state.clientsTotal = state.clients.length }
    state.apps = Array.isArray(apps) ? apps : Array.isArray(apps?.apps) ? apps.apps : null
    state.appsTotal = state.apps ? state.apps.length : null
    state.config = config
    state.memory = memory
    state.vdd = vdd
    if (vdd && vdd.installed !== false) state.vddSettings = await safe(() => invoke('load_vdd_settings'))
    if (typeof logs === 'string') {
      const lines = logs.split('\n')
      const err = lines.filter((l) => /\]\s*(Error|Fatal):/.test(l))
      state.logText = logs
      state.recent = parseSessionLog(logs)
      state.errorLog = parseErrorLog(logs)
      state.logs = { errors: err.length, warnings: lines.filter((l) => /\]\s*Warning:/.test(l)).length, last: err[err.length - 1] || '' }
    }
    state.loaded = true
    remember()
  }
  async function hardware() {
    if (!state.hw.available) return
    const data = await safe(() => invoke('hwinfo_get_sensors'))
    const hw = pickHardware(data?.readings)
    state.hw.cpuTemp = hw.cpuTemp
    state.hw.gpuTemp = hw.gpuTemp
    remember()
  }
  // what does not change while the page is open, read together (these were five reads one after the other: 20 + 63 + 70 + 25 ms before the first tile)
  async function facts() {
    const [version, start, gpus, hwAvailable] = await Promise.all([
      safe(() => sunshine.getVersion(), ''),
      safe(() => invoke('get_sunshine_start_time')),
      safe(() => invoke('get_gpus'), []),
      safe(() => invoke('hwinfo_check_available'), false),
    ])
    state.version = version || ''
    state.startMs = start?.start_time_ms || 0
    state.gpus = Array.isArray(gpus) ? gpus : []
    state.hw.available = hwAvailable === true
    remember()
  }
  let timer = 0, t2 = 0, capTimer = 0, boostUntil = 0, nudge = () => {}
  const onSeen = () => { if (document.visibilityState !== 'hidden') nudge('returned') }
  onMounted(async () => {
    proxy.value = (await safe(() => sunshine.getProxyUrl(), '')) || proxy.value
    // The session and the paired clients decide what the overview draws (idle or streaming, the star of the client in use), so the first read of the session goes out with the others, not after the slowest
    // of them (it used to wait for the log and the driver probe). What is drawn from them waits for all of them (state.settled) instead of being drawn idle and redrawn.
    capTimer = setTimeout(() => { state.settled = true }, SETTLE_CAP_MS)
    await Promise.all([facts().then(() => hardware()), slow(), fast()])
    clearTimeout(capTimer)
    state.settled = true
    // the poll follows pollPlan.js: three seconds, one second for a few seconds after the panel is looked at again and while a session is starting
    // A hidden page (covered, minimized, in the tray) does not poll: the loop ends, and the page being shown again restarts it at once (onSeen).
    const loop = async () => { if (document.hidden) return; await fast(); timer = setTimeout(loop, pollDelay(Date.now(), boostUntil)) }
    nudge = (reason) => { boostUntil = boostUntilFor(Date.now(), reason, boostUntil); clearTimeout(timer); loop() }
    timer = setTimeout(loop, pollDelay(Date.now(), boostUntil))
    document.addEventListener('visibilitychange', onSeen); window.addEventListener('focus', onSeen)
    // the slow reads (history, hardware) also rest while hidden and are read once when the page is shown again
    t2 = setVisibleInterval(() => { slow(); hardware() }, 15000, { resume: true })
  })
  onBeforeUnmount(() => { clearTimeout(timer); clearTimeout(capTimer); clearVisibleInterval(t2); document.removeEventListener('visibilitychange', onSeen); window.removeEventListener('focus', onSeen) })

  // the session with the real name of its client (realSession: the placeholder 「Named Zako」 becomes the paired device's name, else the client family), the same rule in every tile
  const session = computed(() => realSession(state.sessions[0] || null, state.clients))
  const uptimeMs = computed(() => (state.startMs ? Math.max(0, state.now - state.startMs) : 0))
  const perfSession = computed(() => state.perf?.sessions?.find((x) => x.active) || state.perf?.sessions?.[0] || null)
  const share = computed(() => memoryShare(state.memory))
  const proxyUrl = computed(() => proxy.value)
  return { state, proxyUrl, session, uptimeMs, perfSession, memoryShare: share, restart: () => invoke('restart_sunshine_service', { acknowledgeLiveStream: true }) /* PanelHome confirms first: 串流会话会中断 */, stopAll: () => sunshine.stopAllSessions(), openTool: (name) => invoke('open_tool_window', { toolName: name }) }
}

export function formatUptime(ms) {
  const m = Math.floor(ms / 60000)
  const d = Math.floor(m / 1440)
  const h = Math.floor((m % 1440) / 60)
  return d ? `${d}d ${h}h` : h ? `${h}h ${m % 60}m` : `${m % 60}m`
}
