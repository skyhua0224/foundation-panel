import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { appendTail, createLogByteDecoder, decodeLogBytes, findSessions, parseLog, sessionSummary } from 'foundation-ui/diagnostics'
import { sunshine } from '../tauri-adapter.js'
import { combineDiagnosticsRecords } from './diagnosticsRecords.js'
import { SAVE_CANCELLED, saveThroughDialog } from '../shared/saveDialog.js'
import { setVisibleInterval, clearVisibleInterval } from './pageVisibility.js'

// The data of the diagnostics page of the control panel: the host log and configuration through the local proxy, and what only the panel
// can read (system, graphics cards, monitors, device status). Reading only: `save` writes a file the person chose in the system dialog.

const osLine = (text) => /^\[[^\]]+\]:\s*Info:\s*Running with (.+)$/m.exec(String(text || ''))?.[1]?.trim() || ''
const base64 = (bytes) => {
  let out = ''
  for (let i = 0; i < bytes.length; i += 0x8000) out += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  return btoa(out)
}
const proxy = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const getJson = async (path) => {
  const response = await fetch(`${await proxy()}${path}`, { signal: AbortSignal.timeout(6000) })
  if (!response.ok) throw new Error(`HTTP ${response.status}`)
  return response.json()
}
const safe = async (fn, fallback = undefined) => { try { return await fn() } catch { return fallback } }

export async function saveBundle(name, bytes) {
  const saved = await saveThroughDialog(invoke, 'save_binary_file', { contentBase64: base64(bytes), defaultName: name, filterName: 'Zip', extensions: ['zip'] })
  if (saved === null) throw new Error(SAVE_CANCELLED)
  return saved
}

/** `status` = { usb, dualsense, mic, vdd } readers (cached status of the device hub) for the devices part of the bundle. */
export function useDiagnostics({ update, devices = async () => ({}) } = {}) {
  const text = ref('')
  const panelEntries = ref([])
  const live = ref(true)
  const config = ref(null)
  const system = ref(null)
  const gpus = ref([])
  const liveSession = ref(null)
  const clientNames = ref([])
  let offset = 0
  let reading = false
  let timer = 0
  let logBytes = createLogByteDecoder()

  // One viewer, two sources: the host's text log and the panel's own LogEntry
  // list from get_all_logs. Both become the same records; source.id keeps the
  // two apart for the filter.
  const records = computed(() => combineDiagnosticsRecords(text.value, panelEntries.value))

  const readLogChunk = async (response, { final = false } = {}) => {
    if (typeof response.arrayBuffer !== 'function') return response.text()
    return logBytes.push(new Uint8Array(await response.arrayBuffer()), { final })
  }

  async function refresh() {
    if (reading) return
    reading = true
    try {
      panelEntries.value = (await safe(() => invoke('get_all_logs'), [])) || []
      const response = await fetch(`${await proxy()}/api/logs`, { headers: { 'X-Log-Offset': String(offset) }, signal: AbortSignal.timeout(6000) })
      if (response.status === 304) return
      if (!response.ok) return
      const size = Number.parseInt(response.headers.get('X-Log-Size') || '0', 10)
      const incremental = (response.headers.get('X-Log-Range') || '').trim().toLowerCase() === 'incremental'
      if (!incremental) logBytes = createLogByteDecoder()
      const chunk = await readLogChunk(response, { final: !incremental })
      if (incremental && !chunk) { offset = Number.isNaN(size) ? offset : size; return }
      text.value = appendTail(text.value, chunk, { incremental })
      offset = Number.isNaN(size) || size < 0 ? 0 : size
    } catch { /* the viewer keeps what it has */ } finally { reading = false }
  }
  async function pollLive() {
    liveSession.value = (await safe(async () => (await getJson('/api/runtime/sessions'))?.sessions?.[0], null)) || null
  }
  async function loadFacts() {
    config.value = await safe(() => getJson('/api/config'), null)
    system.value = await safe(() => invoke('get_system_info'), null)
    gpus.value = (await safe(() => invoke('get_gpus'), [])) || []
    clientNames.value = ((await safe(() => getJson('/api/clients/list'), null))?.named_certs || []).map((c) => c?.name).filter(Boolean)
  }
  const start = () => { stop(); void refresh(); void pollLive(); timer = setVisibleInterval(() => { void refresh(); void pollLive() }, 5000, { resume: true }) }
  const stop = () => { clearVisibleInterval(timer); timer = 0 }
  watch(live, (on) => (on ? start() : stop()))
  onBeforeUnmount(stop)

  const facts = computed(() => ({
    version: config.value?.version || '',
    channel: update?.channel?.() || '',
    panelVersion: system.value?.app_version || '',
    os: system.value?.os_version || osLine(text.value),
    gpu: gpus.value,
    encoder: config.value?.active_encoder || '',
  }))
  const hostNames = computed(() => [config.value?.sunshine_name].filter(Boolean))
  const statsFor = (summary) => {
    const s = summary.live ? liveSession.value : null
    if (!s) return null
    return [
      s.client_name ? { label: 'Client', value: s.client_name } : null,
      s.width && s.height ? { label: 'Resolution', value: `${s.width}×${s.height}` } : null,
      s.fps ? { label: 'FPS', value: String(s.fps) } : null,
      s.bitrate ? { label: 'Bitrate', value: `${Math.round(s.bitrate / 1000)} Mbps` } : null,
    ].filter(Boolean)
  }

  async function collect() {
    const [log, conf, runtime, clients, sys, gpuList, monitors, dev, panel] = await Promise.all([
      safe(async () => {
        const r = await fetch(`${await proxy()}/api/logs`)
        if (!r.ok) return ''
        return typeof r.arrayBuffer === 'function' ? decodeLogBytes(new Uint8Array(await r.arrayBuffer())) : r.text()
      }, ''),
      safe(() => getJson('/api/config')),
      safe(() => getJson('/api/runtime/sessions')),
      safe(() => getJson('/api/clients/list')),
      safe(() => invoke('get_system_info')),
      safe(() => invoke('get_gpus'), []),
      safe(() => invoke('get_monitors'), []),
      safe(devices, {}),
      safe(() => invoke('get_all_logs'), []),
    ])
    const out = {}
    if (log) out.log = log
    if (panel?.length) out.panelRecords = panel
    if (conf) { out.config = conf }
    out.versions = { host: conf?.version || '', controlPanel: sys?.app_version || '' }
    out.os = sys ? { name: sys.os_version, cpu: sys.cpu_model, memoryBytes: sys.memory_total, fromLog: osLine(log) } : { fromLog: osLine(log) }
    if (gpuList?.length) out.gpu = { adapters: gpuList, encoder: conf?.active_encoder || '', note: 'The driver version is not reported by the control panel.' }
    if (monitors?.length) out.displays = monitors
    out.devices = { clients: (clients?.named_certs || []).map((c) => ({ name: c?.name })), ...dev }
    const rows = parseLog(log)
    out.sessions = {
      fromLog: findSessions(rows).map((s) => sessionSummary(rows, s)).map(({ firstError, ...rest }) => ({ ...rest, firstError: firstError?.message || '' })),
      running: runtime?.sessions || [],
    }
    return out
  }

  const platform = computed(() => String(config.value?.platform || ''))
  return { text, records, live, facts, platform, hostNames, clientNames, statsFor, collect, save: saveBundle, refresh, start, stop, loadFacts }
}
