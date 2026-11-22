// 当前串流: the old 码率调整 and 主机性能 tool windows on one page of 串流. Pure helpers, shared with the tests.

export const BITRATE = Object.freeze({ min: 1000, max: 800000, step: 1000 })
export const BITRATE_PRESETS = Object.freeze([5000, 10000, 20000, 50000, 100000, 200000])
export const DEFAULT_BITRATE = 20000

export const formatBitrate = (kbps) => (kbps >= 1000 ? `${(kbps / 1000).toFixed(0)} Mbps` : `${kbps} Kbps`)
export const validBitrate = (kbps) => Number.isFinite(kbps) && kbps >= BITRATE.min && kbps <= BITRATE.max

/** The sessions that stream (a stopped or stopping session cannot take a bitrate). */
export const liveSessions = (sessions) => (Array.isArray(sessions) ? sessions.filter((s) => s && s.state !== 'STOPPED' && s.state !== 'STOPPING') : [])

/** The session of /api/perf/current that the panel shows: the latest active one, else the first active one. */
export function perfSessionOf(snapshot) {
  const active = snapshot?.sessions?.filter((s) => s.active) ?? []
  if (!active.length) return null
  return active.find((s) => s.session_id === snapshot.latest_session_id) ?? active[0]
}

export const frameBudgetMs = (session) => 1000 / (session?.fps || 60)
export function budgetUsage(session) {
  const p95 = session?.host_latency?.p95_ms
  return p95 == null ? null : (p95 / frameBudgetMs(session)) * 100
}
/** The health word of the tool window: idle, waiting, good, warning, slow, error (the same thresholds). */
export function perfStatus(session, failed = false) {
  if (failed) return 'error'
  if (!session) return 'idle'
  const latency = session.host_latency || {}
  const usage = budgetUsage(session)
  if (!latency.samples || usage == null) return 'waiting'
  const pressure = Math.max(latency.avg_ms == null ? 0 : (latency.avg_ms / frameBudgetMs(session)) * 100, usage * 0.7)
  return pressure <= 90 ? 'good' : pressure <= 170 ? 'warning' : 'slow'
}

export const PIPELINE_SEGMENTS = Object.freeze(['capture_to_convert', 'convert', 'encode_queue', 'encode', 'packet_to_broadcast', 'total'])
/** The pipeline stages that have samples, in pipeline order. */
export const pipelineSegments = (session) => PIPELINE_SEGMENTS.map((key) => ({ key, stats: session?.pipeline?.[key] ?? {} })).filter((s) => s.stats.samples)

// The overview reads the stages and the extras of a perf session through foundation-ui (homePerfInputs.js), the same functions as the Web UI.
export { PIPELINE_FIELDS, pipelineAverages, perfExtras } from 'foundation-ui/home/perf'

export const formatMs = (v) => (v == null ? '--' : `${Number(v).toFixed(2)} ms`)
export const formatNumber = (v) => (v == null ? '--' : Number(v).toFixed(1))
export const formatPercent = (v) => (v == null ? '--' : `${Math.round(v)}%`)
export const formatInteger = (v) => (v == null ? '--' : Number(v).toLocaleString())
export function formatUptime(ms) {
  if (!ms) return '--'
  const total = Math.floor(ms / 1000)
  return `${Math.floor(total / 60)}m ${total % 60}s`
}
