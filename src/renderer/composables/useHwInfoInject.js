import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { hwinfoOsdText, sensorGroups } from './rtssTools.js'

// HWiNFO readings written into the RTSS OSD every 2 s (the tool window's 注入 OSD). The loop belongs to the panel, not to the page:
// it keeps writing while the person looks at another page, and stops when switched off or when a read fails.
const POLL_MS = 2000
const state = reactive({ available: false, loading: false, loaded: false, groups: [], selected: [], injecting: false, error: '' })
let timer = null

async function load() {
  state.loading = true
  try {
    state.available = (await invoke('hwinfo_check_available')) === true
    state.groups = state.available ? sensorGroups(await invoke('hwinfo_get_sensors')) : []
  } catch { state.available = false; state.groups = [] }
  state.loading = false
  state.loaded = true
}
async function tick() {
  try {
    const readings = await invoke('hwinfo_get_readings', { readingIds: [...state.selected] })
    await invoke('rtss_set_osd', { text: hwinfoOsdText(readings) })
    try { state.groups = sensorGroups(await invoke('hwinfo_get_sensors')) } catch { /* the values on the page wait for the next round */ }
  } catch (error) { state.error = String(error); stop() }
}
function start() {
  if (!state.selected.length) return false
  state.error = ''
  state.injecting = true
  clearInterval(timer)
  void tick()
  timer = setInterval(tick, POLL_MS)
  return true
}
function stop() {
  state.injecting = false
  clearInterval(timer); timer = null
  // the HWiNFO lines leave the OSD with the loop
  invoke('rtss_set_osd', { text: '' }).catch(() => {})
}
function toggleReading(index) {
  state.selected = state.selected.includes(index) ? state.selected.filter((i) => i !== index) : [...state.selected, index]
}

export function useHwInfoInject() {
  return { state, load, start, stop, toggleReading }
}
