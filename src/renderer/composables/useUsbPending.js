import { watch } from 'vue'
import { usePending } from 'foundation-ui/pending'
import { usbip } from '../tauri-adapter.js'

// The USB forwarding settings (allow clients, port) as pending changes of the window's one unsaved state: the page reports them, the bar saves them from any page.
const LABELS = { zh: { enabled: '允许客户端转发 USB', port: '转发端口' }, en: { enabled: 'Allow clients to forward USB', port: 'Forwarding port' } }
const lang = (locale) => (String(locale).startsWith('zh') ? 'zh' : 'en')

export function usbEntries(baseline, now, locale) {
  const names = LABELS[lang(locale)]
  return ['enabled', 'port'].filter((k) => baseline[k] !== now[k]).map((k) => ({ key: k, label: names[k], before: baseline[k], after: now[k] }))
}

async function write(entries, side) {
  const read = await usbip.getForwardingConfig()
  if (!read?.success) return { ok: false, error: 'read' }
  const next = { enabled: read.data.enabled === true, port: read.data.port }
  for (const e of entries) next[e.key] = e[side]
  const result = await usbip.saveForwardingConfig(next.enabled, next.port)
  return result?.success ? { ok: true } : { ok: false, error: 'save' }
}

/** Once, at start. */
export function defineUsbSource({ pageLabel }) {
  // the host reads the forwarding settings at start: the page's own toast said to restart Sunshine
  usePending().define('usb', { page: 'usb', pageLabel, restart: () => true, save: (entries) => write(entries, 'after'), revertSaved: (entries) => write(entries, 'before') })
}

/** On the page: `state()` = { enabled, port }, `baseline()` the saved values, `ready` a ref, `set(key, value)` puts a value in the page. */
export function useUsbPending({ state, baseline, ready, set, locale, valid }) {
  const pending = usePending()
  let restored = false
  const detach = pending.attach('usb', {
    revert: (key) => set(key, baseline()[key]),
    revertAll: () => { for (const k of ['enabled', 'port']) set(k, baseline()[k]) },
    saved: () => set('baseline'),
    reverted: () => set('reload'),
  })
  const stop = watch(() => (ready.value ? JSON.stringify([state(), baseline()]) : null), (json) => {
    if (json === null) return
    if (!restored) {
      restored = true
      const kept = pending.entries('usb')
      if (kept.length) { for (const e of kept) set(e.key, e.after); return }
    }
    // an invalid port is not offered for saving
    pending.sync('usb', valid() ? usbEntries(baseline(), state(), locale()) : usbEntries(baseline(), { ...state(), port: baseline().port }, locale()))
  }, { immediate: true, flush: 'post' })
  return () => { stop(); detach() }
}
