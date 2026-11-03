// The Web UI pages the panel shows in its frame (/config, /apps, ...) are whole documents: the first visit of each one loads and compiles its scripts
// (measured on the Skyindows panel: /config first visit 1.6 s to its first paint, 0.23 s once its files were loaded before). The panel therefore loads them once,
// one at a time, in a hidden frame, while nobody uses the window; the frame is removed afterwards. Nothing is shown and nothing is written.

export const PREWARM_PATHS = ['/config', '/apps', '/pin', '/downloads', '/troubleshooting']

/**
 * `base` the proxy's address; `load(url)` loads one page in the hidden frame (resolves when it loaded or gave up); `visible()` whether the window is
 * shown; `idleFor()` ms since the last input; `done()` called once at the end (the frame is removed); `sleep(ms)` waits.
 * Returns `stop()`.
 */
export function startWebPrewarm({ base, paths = PREWARM_PATHS, load, visible, idleFor, done, sleep, startAfterMs = 4000, idleMs = 1500, pauseMs = 400 }) {
  let stopped = false
  void (async () => {
    try {
      await sleep(startAfterMs)
      for (const path of paths) {
        while (!stopped && !(visible() && idleFor() >= idleMs)) await sleep(1000)
        if (stopped) return
        try { await load(base + path) } catch { /* a page that does not load is not prewarmed */ }
        await sleep(pauseMs)
      }
    } finally {
      done()
    }
  })()
  return () => { stopped = true }
}

/** The browser side: one hidden frame that is moved from page to page. */
export function createHiddenFrameLoader(doc = document, timeoutMs = 8000) {
  let frame = null
  const load = (url) => new Promise((resolve) => {
    if (!frame) {
      frame = doc.createElement('iframe')
      frame.setAttribute('aria-hidden', 'true')
      frame.tabIndex = -1
      frame.style.cssText = 'position:fixed;left:-9999px;top:0;width:1024px;height:768px;border:0;visibility:hidden;pointer-events:none'
      doc.body.appendChild(frame)
    }
    const timer = setTimeout(finish, timeoutMs)
    function finish() { clearTimeout(timer); frame.removeEventListener('load', finish); resolve() }
    frame.addEventListener('load', finish)
    frame.src = url
  })
  const remove = () => { frame?.remove(); frame = null }
  return { load, remove }
}
