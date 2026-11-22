// Polling that follows the page. `document.hidden` is true when the window is hidden to the tray, minimized or covered by other windows
// (src-tauri/src/windows/lifecycle.js folds the panel's own state and the engine's occlusion into it); focus does not matter: a window that
// is only behind a click is still looked at. A timer made here does not tick while the page is hidden and, with `resume`, ticks once at
// once when the page is shown again, so what is on screen is current the moment it is seen. While the page is visible it is a plain interval.

const timers = new Map()
let nextToken = 1

const hidden = () => typeof document !== 'undefined' && document.hidden === true

/** setInterval that pauses while the page is hidden. Returns a token for `clearVisibleInterval`. */
export function setVisibleInterval(fn, ms, { resume = false } = {}) {
  const token = nextToken++
  const entry = { id: 0, off: () => {} }
  const start = () => { if (!entry.id) entry.id = setInterval(fn, ms) }
  const stop = () => { if (entry.id) { clearInterval(entry.id); entry.id = 0 } }
  const onChange = () => {
    if (hidden()) { stop(); return }
    if (entry.id) return
    start()
    if (resume) fn()
  }
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', onChange)
    entry.off = () => document.removeEventListener('visibilitychange', onChange)
  }
  if (!hidden()) start()
  entry.stop = stop
  timers.set(token, entry)
  return token
}

export function clearVisibleInterval(token) {
  const entry = timers.get(token)
  if (!entry) return
  entry.stop()
  entry.off()
  timers.delete(token)
}

/** Calls `fn` when the page becomes visible again (not at the first mount). Returns the function that stops it. */
export function onPageShown(fn) {
  if (typeof document === 'undefined') return () => {}
  const handler = () => { if (!hidden()) fn() }
  document.addEventListener('visibilitychange', handler)
  return () => document.removeEventListener('visibilitychange', handler)
}
