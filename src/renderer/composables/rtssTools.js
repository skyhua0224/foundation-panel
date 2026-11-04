// RTSS and HWiNFO on the panel's RTSS 叠加 page: the pure parts of the old tool window (tool-window/tools/rtss/*), shared with its tests.
// The commands are the old ones (get_rtss_status, rtss_*, hwinfo_*); the monitoring style stays in the same localStorage key.

export const FPS_LIMITS = Object.freeze({ min: 15, max: 360 })
export const FPS_PRESETS = Object.freeze([30, 60, 90, 120, 144, 165, 240])
export const FONT_SIZES = Object.freeze([0, 12, 16, 20, 24, 32])
export const ZOOMS = Object.freeze([1, 2, 3, 4])
export const INTERVALS = Object.freeze([500, 1000, 2000, 5000])
export const OSD_OWNER = 'Sunshine'

export const MONITORING_KEY = 'rtss-monitoring-config'
export const MONITORING_DEFAULTS = Object.freeze({
  interval_ms: 1000,
  metrics: ['session_state', 'stream_fps', 'stream_bitrate', 'pipeline_fps', 'host_perf_p95'],
  title_color: 'FFD700',
  label_color: 'AAAAAA',
  value_color: '00FF00',
  font_size: 0,
  header_text: '☀ Foundation Sunshine',
})

/** The monitoring style the tool window saved (or the defaults). */
export function loadMonitoringConfig(storage = globalThis.localStorage) {
  try {
    const raw = storage?.getItem(MONITORING_KEY)
    if (raw) return { ...MONITORING_DEFAULTS, metrics: [...MONITORING_DEFAULTS.metrics], ...JSON.parse(raw) }
  } catch { /* a broken entry is replaced by the defaults */ }
  return { ...MONITORING_DEFAULTS, metrics: [...MONITORING_DEFAULTS.metrics] }
}
export function saveMonitoringConfig(config, storage = globalThis.localStorage) {
  try { storage?.setItem(MONITORING_KEY, JSON.stringify(config)) } catch { /* storage unavailable */ }
}

/** Insert `insertion` at the caret (replacing the selection): the new text and where the caret goes. */
export function insertText(text, start, end, insertion) {
  const s = Math.max(0, Math.min(start ?? text.length, text.length)), e = Math.max(s, Math.min(end ?? s, text.length))
  return { text: text.slice(0, s) + insertion + text.slice(e), caret: s + insertion.length }
}
/** Wrap the selection in an RTSS pair tag: <B>…<B0>, <I>…<I0>; the caret goes after the selection, inside the pair. */
export function wrapTag(text, start, end, tag) {
  const s = Math.max(0, Math.min(start ?? text.length, text.length)), e = Math.max(s, Math.min(end ?? s, text.length))
  const selected = text.slice(s, e)
  return { text: text.slice(0, s) + `<${tag}>${selected}<${tag}0>` + text.slice(e), caret: s + tag.length + 2 + selected.length }
}
export const colorTag = (hex) => `<C=${String(hex || 'FFFFFF').replace('#', '').toUpperCase()}>`
export const sizeTag = (px) => (px > 0 ? `<S=${px}>` : '')

/** The metrics the host offers, in the three groups of the tool window (empty groups left out). */
export function metricGroups(available, zh) {
  const groups = [
    { id: 'session', label: zh ? '串流会话' : 'Streaming', metrics: [] },
    { id: 'host_perf', label: zh ? '主机性能' : 'Host Performance', metrics: [] },
    { id: 'process', label: zh ? '进程性能' : 'Process', metrics: [] },
  ]
  for (const m of Array.isArray(available) ? available : []) {
    const group = groups.find((g) => g.id === m.group)
    if (group) group.metrics.push({ value: m.id, label: zh ? m.label_zh : m.label_en })
  }
  return groups.filter((g) => g.metrics.length)
}

/** HWiNFO's shared memory as sensor groups with their readings (index = the reading's position, the id hwinfo_get_readings takes). */
export function sensorGroups(data) {
  if (!data?.sensors || !data?.readings) return []
  return data.sensors.map((sensor, si) => ({
    name: sensor.name_user || sensor.name_original,
    readings: data.readings.map((r, index) => ({ r, index })).filter(({ r }) => r.sensor_index === si)
      .map(({ r, index }) => ({ index, label: r.label_user || r.label_original, unit: r.unit, value: r.value })),
  })).filter((g) => g.readings.length)
}
/** The groups that match a search (a group name or a reading label); a match opens its group. */
export function filterSensors(groups, query) {
  const q = String(query || '').trim().toLowerCase()
  if (!q) return groups.map((g) => ({ ...g, match: false }))
  return groups.map((g) => ({ ...g, match: true, readings: g.readings.filter((r) => r.label.toLowerCase().includes(q) || g.name.toLowerCase().includes(q)) })).filter((g) => g.readings.length)
}
/** The OSD text of the chosen HWiNFO readings: one `label: value unit` per line. */
export const hwinfoOsdText = (readings) => (readings || []).map((r) => `${r.label_user || r.label_original}: ${Number(r.value).toFixed(1)} ${r.unit}`).join('\n')
