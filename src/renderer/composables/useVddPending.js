import { watch } from 'vue'
import { usePending } from 'foundation-ui/pending'
import { vdd } from '../tauri-adapter.js'

// The virtual display settings as pending changes: one entry for every value of the driver's settings that differs from what is saved (a list, such as the
// resolutions, is one value). The page reports them, takes them back and applies the ones kept from before; the save works without the page.
const LABELS = {
  zh: { 'monitors.count': '显示器数量', 'gpu.friendlyname': '显卡', 'global.g_refresh_rate': '刷新率列表', 'resolutions.resolution': '分辨率列表', 'colour.SDR10bit': '10 位 SDR', 'colour.HDRPlus': 'HDR', 'colour.ColourFormat': '色彩格式', 'cursor.CursorMaxX': '光标最大宽度', 'cursor.CursorMaxY': '光标最大高度', 'cursor.AlphaCursorSupport': 'Alpha 光标', 'cursor.XorCursorSupportLevel': 'XOR 光标等级', 'cursor.HardwareCursor': '硬件光标', 'edid.CustomEdid': '自定义 EDID', 'edid.DolbyVision': '杜比视界', 'edid.PreventSpoof': '保留制造商信息', 'edid.EdidCeaOverride': 'CEA 扩展块', 'edid.EdidProfile': '内置 EDID 版本', 'edid.Vrr': '可变刷新率' },
  en: { 'monitors.count': 'Display count', 'gpu.friendlyname': 'GPU', 'global.g_refresh_rate': 'Refresh rates', 'resolutions.resolution': 'Resolutions', 'colour.SDR10bit': '10-bit SDR', 'colour.HDRPlus': 'HDR', 'colour.ColourFormat': 'Colour format', 'cursor.CursorMaxX': 'Cursor max width', 'cursor.CursorMaxY': 'Cursor max height', 'cursor.AlphaCursorSupport': 'Alpha cursor', 'cursor.XorCursorSupportLevel': 'XOR cursor level', 'cursor.HardwareCursor': 'Hardware cursor', 'edid.CustomEdid': 'Custom EDID', 'edid.DolbyVision': 'Dolby Vision', 'edid.PreventSpoof': 'Keep manufacturer', 'edid.EdidCeaOverride': 'CEA block', 'edid.EdidProfile': 'Built-in EDID version', 'edid.Vrr': 'Variable refresh rate' },
}
const clone = (v) => JSON.parse(JSON.stringify(v))
const isLeaf = (v) => v === null || typeof v !== 'object' || Array.isArray(v)

export function leaves(obj, prefix = '', out = {}) {
  for (const [k, v] of Object.entries(obj || {})) {
    const path = prefix ? `${prefix}.${k}` : k
    if (isLeaf(v)) out[path] = v
    else leaves(v, path, out)
  }
  return out
}
export function setPath(obj, path, value) {
  const keys = path.split('.')
  let at = obj
  for (const k of keys.slice(0, -1)) at = at[k] ||= {}
  at[keys.at(-1)] = clone(value)
}
export function vddEntries(saved, current, locale) {
  const names = LABELS[String(locale).startsWith('zh') ? 'zh' : 'en']
  const a = leaves(saved)
  const b = leaves(current)
  const out = []
  for (const path of new Set([...Object.keys(a), ...Object.keys(b)])) {
    if (JSON.stringify(a[path]) === JSON.stringify(b[path])) continue
    const list = (v) => (path === 'resolutions.resolution' ? (v || []).map((r) => `${r.width}×${r.height}`).join(', ') : Array.isArray(v) ? v.join(', ') : undefined)
    out.push({ key: path, label: names[path] || path, before: a[path], after: b[path], beforeText: list(a[path]), afterText: list(b[path]) })
  }
  return out
}
export const applyVddEntries = (payload, entries, side = 'after') => { const next = clone(payload); for (const e of entries) setPath(next, e.key, e[side]); return next }

async function writeEntries(entries, side, failure) {
  const loaded = await vdd.loadSettings()
  if (!loaded?.success) return { ok: false, error: failure }
  const result = await vdd.saveSettings(applyVddEntries(loaded.data, entries, side))
  return result?.success ? { ok: true } : { ok: false, error: result?.message || failure }
}

/** Once, at start: the settings of the virtual display as a source that can be saved from any page. */
export function defineVddSource({ pageLabel, failure = 'save' }) {
  usePending().define('vdd', {
    page: 'vdd', pageLabel, restart: () => false,
    save: (entries) => writeEntries(entries, 'after', failure),
    revertSaved: (entries) => writeEntries(entries, 'before', failure),
  })
}

/** On the page: `payload()` the settings in the form that is saved, `saved()` the saved form, `ready` a ref, `set(path, value)` puts a value in the page's model. */
export function useVddPending({ payload, saved, ready, set, locale, onSaved, onReverted }) {
  const pending = usePending()
  let restored = false
  const detach = pending.attach('vdd', {
    revert: (key) => { const s = leaves(saved())[key]; if (s !== undefined) set(key, s) },
    revertAll: () => { for (const [k, v] of Object.entries(leaves(saved()))) set(k, v) },
    saved: () => onSaved?.(),
    reverted: () => onReverted?.(),
  })
  const stop = watch(() => (ready.value ? JSON.stringify([payload(), saved()]) : null), (json) => {
    if (json === null) return
    if (!restored) {
      restored = true
      const kept = pending.entries('vdd')
      if (kept.length) { for (const e of kept) set(e.key, e.after); return }
    }
    pending.sync('vdd', vddEntries(saved(), payload(), locale()))
  }, { immediate: true, flush: 'post' })
  return () => { stop(); detach() }
}
