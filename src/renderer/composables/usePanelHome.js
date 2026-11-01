import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { sunshine } from '@/tauri-adapter.js'
import { pickHardware } from './hardwareReadings.js'
import { parseSessionLog, parseErrorLog } from 'foundation-ui/vue'

const HISTORY = 60
const safe = async (fn, fallback = null) => { try { return await fn() } catch { return fallback } }

/**
 * Live data of the overview tiles. Every value comes from a real source (host API through the local proxy, Tauri commands);
 * a source that fails leaves its field empty and the tile hides or shows its unavailable state. Samples for the sparklines
 * are collected here while the page is open.
 */
export function usePanelHome() {
  const state = reactive({
    version: '', startMs: 0, now: Date.now(), sessions: [], clientsTotal: null, appsTotal: null,
    perf: null, config: null, memory: null, gpus: [], vdd: null, logs: null, loaded: false,
    // CPU and GPU load come from native Windows counters (system_load); temperatures only exist through HWiNFO, if it is running
    sys: { cpu: null, gpu: null },
    hw: { available: false, cpuTemp: null, gpuTemp: null },
    series: { bitrate: [], fps: [], latency: [], cpu: [], gpu: [], memory: [] },
    clients: null, apps: null, recent: null, errorLog: null,
  })
  const proxy = ref('')
  const fetchJson = async (path, init) => {
    const r = await fetch(`${proxy.value}${path}`, init)
    if (!r.ok) throw new Error(String(r.status))
    return r.json()
  }
  const fetchText = async (path, init) => {
    const r = await fetch(`${proxy.value}${path}`, init)
    if (!r.ok) throw new Error(String(r.status))
    return r.text()
  }
  const push = (list, v) => { if (typeof v === 'number' && Number.isFinite(v)) { list.push(v); if (list.length > HISTORY) list.shift() } }
  const memoryShare = (m) => (m?.system_total_memory ? Math.min(1, (m.system_total_memory - (m.system_available_memory || 0)) / m.system_total_memory) : null)

  async function fast() {
    state.now = Date.now()
    const body = await safe(() => fetchJson('/api/runtime/sessions'))
    state.sessions = (body && Array.isArray(body.sessions) ? body.sessions : (await safe(() => sunshine.getActiveSessions(), []))) || []
    const s = state.sessions[0]
    if (s) {
      push(state.series.bitrate, s.bitrate)
      const perf = await safe(() => fetchJson('/api/perf/current'))
      state.perf = perf
      const ps = perf?.sessions?.find((x) => x.active) || perf?.sessions?.[0]
      push(state.series.latency, ps?.host_latency?.avg_ms)
      push(state.series.fps, ps?.frame_rates?.encoded?.fps ?? ps?.host_latency?.recent_fps ?? s.fps)
    } else { state.perf = null; state.series.bitrate = []; state.series.fps = []; state.series.latency = [] }
    const load = await safe(() => invoke('system_load'))
    if (load) {
      state.sys.cpu = typeof load.cpu_percent === 'number' ? load.cpu_percent : null
      state.sys.gpu = typeof load.gpu_percent === 'number' ? load.gpu_percent : null
      push(state.series.cpu, state.sys.cpu); push(state.series.gpu, state.sys.gpu)
    }
    push(state.series.memory, memoryShare(state.memory))
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
    if (typeof logs === 'string') {
      const lines = logs.split('\n')
      const err = lines.filter((l) => /\]\s*(Error|Fatal):/.test(l))
      state.recent = parseSessionLog(logs)
      state.errorLog = parseErrorLog(logs)
      state.logs = { errors: err.length, warnings: lines.filter((l) => /\]\s*Warning:/.test(l)).length, last: err[err.length - 1] || '' }
    }
    if (state.hw.available) {
      const data = await safe(() => invoke('hwinfo_get_sensors'))
      const hw = pickHardware(data?.readings)
      state.hw.cpuTemp = hw.cpuTemp
      state.hw.gpuTemp = hw.gpuTemp
    }
    state.loaded = true
  }
  let t1 = 0, t2 = 0
  onMounted(async () => {
    proxy.value = (await safe(() => sunshine.getProxyUrl(), '')) || ''
    state.version = (await safe(() => sunshine.getVersion(), '')) || ''
    const start = await safe(() => invoke('get_sunshine_start_time'))
    state.startMs = start?.start_time_ms || 0
    const gpus = await safe(() => invoke('get_gpus'), [])
    state.gpus = Array.isArray(gpus) ? gpus : []
    state.hw.available = (await safe(() => invoke('hwinfo_check_available'), false)) === true
    await slow()
    await fast()
    t1 = setInterval(fast, 3000)
    t2 = setInterval(slow, 15000)
  })
  onBeforeUnmount(() => { clearInterval(t1); clearInterval(t2) })

  const session = computed(() => state.sessions[0] || null)
  const uptimeMs = computed(() => (state.startMs ? Math.max(0, state.now - state.startMs) : 0))
  const perfSession = computed(() => state.perf?.sessions?.find((x) => x.active) || state.perf?.sessions?.[0] || null)
  const share = computed(() => memoryShare(state.memory))
  const proxyUrl = computed(() => proxy.value)
  return { state, proxyUrl, session, uptimeMs, perfSession, memoryShare: share, restart: () => invoke('restart_sunshine_service'), stopAll: () => sunshine.stopAllSessions(), openTool: (name) => invoke('open_tool_window', { toolName: name }) }
}

export function formatUptime(ms) {
  const m = Math.floor(ms / 60000)
  const d = Math.floor(m / 1440)
  const h = Math.floor((m % 1440) / 60)
  return d ? `${d}d ${h}h` : h ? `${h}h ${m % 60}m` : `${m % 60}m`
}
