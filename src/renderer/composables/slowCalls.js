// Calls of the renderer that take long are written to the panel's log (kind, name, milliseconds): a fetch to the host through the proxy, a native command,
// a frame that was blocked, a timer that came late. They are what a stall of the window is read from: whether it was the host, a native probe, the
// page's own script or the machine (a late timer is late for everything on it). Nothing is written for calls under the limits.

export const LIMITS = { call: 500, frame: 300, timer: 1500 }
import { faultReport, isLongPoll } from './frameReport.js'

/** `report(kind, name, ms, detail)`; returns the wrapped function and a `pending()` list of the calls that have not returned. */
export function trackCalls(call, { report, now = () => performance.now(), kind, limit = LIMITS.call, nameOf = (args) => String(args[0]), ignore = () => false }) {
  const open = new Map()
  let next = 0
  const wrapped = function (...args) {
    const id = ++next
    const started = now()
    const name = nameOf(args)
    open.set(id, { name, started })
    const done = () => {
      open.delete(id)
      const ms = now() - started
      if (ms >= limit && !ignore(args)) report(kind, name, ms)
    }
    let result
    try { result = call.apply(this, args) } catch (error) { done(); throw error }
    if (result && typeof result.then === 'function') result.then(done, done)
    else done()
    return result
  }
  return { wrapped, pending: () => [...open.values()].map((c) => ({ name: c.name, ms: now() - c.started })) }
}

const pathOf = (input) => {
  const raw = typeof input === 'string' ? input : input?.url || String(input)
  try { return new URL(raw, 'http://x').pathname } catch { return raw.slice(0, 80) }
}

/** Starts the reports in a page; `report` writes one line, `extraPending` lists the commands that have not returned. Returns a stop function. */
export function startSlowCallLog(report, win = window, extraPending = null) {
  const stops = []
  const watch = []
  if (typeof win.fetch === 'function') {
    // the long poll of the pairing list takes 25 s by design: it is neither reported as slow nor listed as what a late timer was waiting for
    const { wrapped, pending } = trackCalls(win.fetch.bind(win), { report, kind: 'fetch', nameOf: (args) => pathOf(args[0]), ignore: (args) => isLongPoll(String(args[0]?.href ?? args[0]?.url ?? args[0])) })
    win.fetch = wrapped
    watch.push(pending)
  }
  // the commands of the renderer are timed by the import they go through (tauri-core-timed.js); their pending list is passed in
  if (typeof extraPending === 'function') watch.push(extraPending)
  // a frame that was blocked: its scripts say by what
  try {
    const observer = new win.PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration < LIMITS.frame) continue
        const scripts = (entry.scripts || []).filter((s) => s.duration >= 20).map((s) => `${s.invokerType}:${(s.invoker || '').slice(0, 40)}:${s.duration | 0}ms@${String(s.sourceURL || '').split('/').pop()}:${s.sourceCharPosition}`).join(' ')
        report('frame', 'long-animation-frame', entry.duration, `blocking ${entry.blockingDuration | 0} ms ${scripts}`)
      }
    })
    observer.observe({ type: 'long-animation-frame', buffered: false })
    stops.push(() => observer.disconnect())
  } catch { /* an engine without the entry type */ }
  // a late timer, and what had not come back by then
  let last = performance.now()
  const timer = win.setInterval(() => {
    const t = performance.now()
    const late = t - last - 250
    last = t
    if (late >= LIMITS.timer) {
      const open = watch.flatMap((p) => p()).filter((c) => c.ms >= 1000 && !isLongPoll(c.name)).map((c) => `${c.name} ${c.ms | 0}ms`).join(', ')
      report('timer', 'late', late, open ? `pending: ${open}` : 'nothing pending')
    }
  }, 250)
  stops.push(() => win.clearInterval(timer))
  return () => stops.forEach((stop) => stop())
}

/** The errors and the rejections of the panel's own page are lines of the log too: a blank page or a dead button leaves a message and a place. Returns a stop function. */
export function installFaultLog(report, win = window) {
  const BENIGN = /ResizeObserver loop|^Script error\.?$/
  const on = (event) => {
    const r = faultReport(event)
    if (BENIGN.test(r.name)) return
    report(r.kind, r.name, r.ms, r.detail)
  }
  win.addEventListener('error', on)
  win.addEventListener('unhandledrejection', on)
  return () => { win.removeEventListener('error', on); win.removeEventListener('unhandledrejection', on) }
}
