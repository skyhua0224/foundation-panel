// The mode lists of the ZakoDisplay driver (vdd_settings.xml): <resolutions><resolution><width/><height/></resolution>… and <global><g_refresh_rate/>….
// Source of the format: https://github.com/qiin2333/zako-vdd (README, "配置文件说明", and the shipped vdd_settings.xml).
// Pure functions on plain arrays of strings ('1920x1080', '59.94') so that the page and the tests share one set of rules.
// The order of a list is the person's order and is written as it is; nothing here sorts unless asked to.

/** Sane limits of our editor (the driver itself names no limit: its README says the maximum depends on the GPU and the system). */
export const RESOLUTION_LIMITS = { min: 320, max: 16384 }
export const RATE_LIMITS = { min: 1, max: 480 }
export const MAX_ENTRIES = 64

/** '1920x1080', '1920×1080', '1920 X 1080' -> { width, height } or null */
export function parseResolution(text) {
  const m = /^\s*(\d{1,5})\s*[x×X*]\s*(\d{1,5})\s*$/.exec(String(text ?? ''))
  return m ? { width: Number(m[1]), height: Number(m[2]) } : null
}

/** The canonical text of a resolution, or an error code: 'invalid' (not width x height) | 'range' */
export function normalizeResolution(text) {
  const r = parseResolution(text)
  if (!r) return { error: 'invalid' }
  const { min, max } = RESOLUTION_LIMITS
  if (r.width < min || r.width > max || r.height < min || r.height > max) return { error: 'range' }
  return { value: `${r.width}x${r.height}` }
}

/** '60', '59.94', '119,88' -> canonical text ('60', '59.94') or an error code: 'invalid' | 'range' */
export function normalizeRate(text) {
  const raw = String(text ?? '').trim().replace(',', '.').replace(/\s*hz$/i, '')
  if (!/^\d{1,3}(\.\d{1,3})?$/.test(raw)) return { error: 'invalid' }
  const n = Number(raw)
  if (!Number.isFinite(n) || n < RATE_LIMITS.min || n > RATE_LIMITS.max) return { error: 'range' }
  return { value: String(n) }
}

export const normalizers = { resolution: normalizeResolution, rate: normalizeRate }

/** What a loaded list becomes: canonical, duplicates dropped, order kept. A value the rules refuse is kept as it is (nothing the person already has is lost). */
export function cleanList(kind, values) {
  const out = []
  for (const v of values || []) {
    const n = normalizers[kind](v)
    const value = n.value ?? String(v).trim()
    if (value && !out.includes(value)) out.push(value)
  }
  return out
}

/** Append a value. -> { list, value } or { list (unchanged), error: 'invalid' | 'range' | 'exists' | 'full' } */
export function addEntry(kind, list, text) {
  const n = normalizers[kind](text)
  if (n.error) return { list, error: n.error }
  if (list.includes(n.value)) return { list, error: 'exists' }
  if (list.length >= MAX_ENTRIES) return { list, error: 'full' }
  return { list: [...list, n.value], value: n.value }
}

/** Replace the value at an index (the position stays). Writing the same value back is no change and no error. */
export function editEntry(kind, list, index, text) {
  if (index < 0 || index >= list.length) return { list, error: 'invalid' }
  const n = normalizers[kind](text)
  if (n.error) return { list, error: n.error }
  if (n.value === list[index]) return { list, value: n.value }
  if (list.includes(n.value)) return { list, error: 'exists' }
  const next = list.slice()
  next[index] = n.value
  return { list: next, value: n.value }
}

/** Remove the value at an index; the last one stays (the driver needs at least one mode). */
export function removeEntry(list, index) {
  if (list.length <= 1) return { list, error: 'minOne' }
  if (index < 0 || index >= list.length) return { list, error: 'invalid' }
  return { list: list.filter((_, i) => i !== index) }
}

/** Move the value at an index by delta places (-1 up, +1 down); at an end nothing moves. */
export function moveEntry(list, index, delta) {
  const to = index + delta
  if (index < 0 || index >= list.length || to < 0 || to >= list.length || to === index) return list
  const next = list.slice()
  const [item] = next.splice(index, 1)
  next.splice(to, 0, item)
  return next
}

const area = (v) => { const r = parseResolution(v); return r ? r.width * r.height : 0 }
/** Ascending (a chosen action, never done on its own). */
export function sortList(kind, list) {
  const key = kind === 'resolution' ? (v) => [area(v), parseResolution(v)?.width ?? 0] : (v) => [parseFloat(v), 0]
  return list.slice().sort((a, b) => { const x = key(a), y = key(b); return x[0] - y[0] || x[1] - y[1] })
}

/** The driver's resolution nodes from the list (what vdd_settings.xml carries), and back. */
export const toResolutionNodes = (list) => list.map(parseResolution).filter(Boolean)
export const fromResolutionNodes = (nodes) => cleanList('resolution', (nodes || []).filter((r) => r && r.width && r.height).map((r) => `${r.width}x${r.height}`))
