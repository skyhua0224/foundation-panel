// The panel talks to the Web UI through its own local proxy, and checks from the WebView that the proxy answers (a proxy or TUN tool can break exactly that). The check
// shares the WebView's connection pool with the pages' own requests (six at a time to one origin): when the first requests of a start are slow, a healthy proxy answers
// the check late and one 5 s try fails. One failed try is not a verdict. The check is tried again with a short backoff before the page says anything, and keeps being
// tried in the background while the page does, so the page clears by itself.
// A window opened right after the computer started has the same cold-start slowness (the proxy binds its port a moment after the window, the first answers take seconds): the first round
// is about 25 s of tries (quick at first, then every 5 s) before the page says anything, and the page is a neutral loading state meanwhile (the frame's cover), never the error.
export const FIRST_ROUND_DELAYS = Object.freeze([0, 500, 1000, 2000, 3000, 5000, 5000, 5000, 5000]) // ms before each of the first tries
export const BACKGROUND_EVERY_MS = 5000

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

/**
 * `verify()` resolves true when the proxy answers. `run()` resolves true once it has (possibly long after `onShown`), false after `stop()`.
 * onShown() is called once, when the first round has failed; onCleared() when a later try succeeds; onAttempt(n) before every try.
 */
export function createProxyGate({ verify, onShown = () => {}, onCleared = () => {}, onAttempt = () => {}, delays = FIRST_ROUND_DELAYS, backgroundMs = BACKGROUND_EVERY_MS, sleep = wait } = {}) {
  let stopped = false
  let shown = false
  let attempts = 0
  let nudge = null // resolves the wait before the next background try (the button)
  const attempt = async () => {
    attempts += 1
    onAttempt(attempts)
    try { return (await verify()) === true } catch { return false }
  }
  const ok = () => { if (shown) { shown = false; onCleared() } return true }
  const run = async () => {
    for (const delay of delays) {
      if (delay > 0) await sleep(delay)
      if (stopped) return false
      if (await attempt()) return true
    }
    if (stopped) return false
    shown = true
    onShown()
    for (;;) {
      await Promise.race([sleep(backgroundMs), new Promise((resolve) => { nudge = resolve })])
      nudge = null
      if (stopped) return false
      if (await attempt()) return ok()
    }
  }
  return { run, stop: () => { stopped = true; nudge?.() }, retryNow: () => nudge?.(), get attempts() { return attempts }, get shown() { return shown } }
}
