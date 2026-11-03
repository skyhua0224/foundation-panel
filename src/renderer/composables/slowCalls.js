// Calls of the renderer that take long are written to the panel's log (kind, name, milliseconds): a fetch to the host through the proxy, a native command,
// a frame that was blocked, a timer that came late. They are what a stall of the window is read from: whether it was the host, a native probe, the
// page's own script or the machine (a late timer is late for everything on it). Nothing is written for calls under the limits.

export const LIMITS = { call: 500, frame: 300, timer: 1500 }

/** `report(kind, name, ms, detail)`; returns the wrapped function and a `pending()` list of the calls that have not returned. */
export function trackCalls(call, { report, now = () => performance.now(), kind, limit = LIMITS.call, nameOf = (args) => String(args[0]) }) {
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
      if (ms >= limit) report(kind, name, ms)
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

/** Starts the reports in a page; `report` writes one line. Returns a stop function. */
export function startSlowCallLog(report, win = window) {
  const stops = []
  const watch = []
  if (typeof win.fetch === 'function') {
    const { wrapped, pending } = trackCalls(win.fetch.bind(win), { report, kind: 'fetch', nameOf: (args) => pathOf(args[0]) })
    win.fetch = wrapped
    watch.push(pending)
  }
  const internals = win.__TAURI_INTERNALS__
  if (internals && typeof internals.invoke === 'function') {
    const original = internals.invoke.bind(internals)
    const { wrapped, pending } = trackCalls(original, { report, kind: 'invoke', nameOf: (args) => String(args[0]) })
    internals.invoke = wrapped
    watch.push(pending)
  }
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
      const open = watch.flatMap((p) => p()).filter((c) => c.ms >= 1000).map((c) => `${c.name} ${c.ms | 0}ms`).join(', ')
      report('timer', 'late', late, open ? `pending: ${open}` : 'nothing pending')
    }
  }, 250)
  stops.push(() => win.clearInterval(timer))
  return () => stops.forEach((stop) => stop())
}
