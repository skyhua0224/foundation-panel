// After a change of the display scale the native side covers the page while WebView2 settles (SUNSHINE_GUI_WEBVIEW_DPI_COVER = adaptive | snapshot) and tells
// it the size of the window in pixels. WebView2 lays the page out right, then once more for the old size at the new scale, then right again: the cover goes
// when the layout has matched the window for a while, not at the first match.

export const SETTLE = { stableMs: 130, giveUpMs: 700 }

/** Judges layout samples against the window's size in pixels; `sample` is true once the layout has matched for `stableMs`. */
export function createSettleJudge({ width, height, stableMs = SETTLE.stableMs, stableFrames = 0 }) {
  let since = null
  let frames = 0
  return {
    sample(innerWidth, innerHeight, ratio, now) {
      const slack = Math.ceil(ratio) + 1
      const matches = Math.abs(innerWidth * ratio - width) <= slack && Math.abs(innerHeight * ratio - height) <= slack
      if (!matches) {
        since = null
        frames = 0
        return false
      }
      if (since === null) since = now
      frames += 1
      // stableFrames: that many frames in a row (one sample per frame); otherwise the time
      return stableFrames > 0 ? frames >= stableFrames : now - since >= stableMs
    },
  }
}

/** Follows the native cover: on each `panel-dpi-cover` event it watches the layout every frame and reports with `panel_dpi_settled`. Returns the unlisten promise. */
export function followDpiCover({ listen, invoke, raf = (fn) => requestAnimationFrame(fn), now = () => performance.now(), win = window }) {
  let current = 0
  return listen('panel-dpi-cover', (event) => {
    const { width, height, token, stable_ms: stableMs, stable_frames: stableFrames } = event.payload || {}
    if (!width || !height || !token) return
    current = token
    const judge = createSettleJudge({ width, height, stableMs: stableMs || SETTLE.stableMs, stableFrames: stableFrames || 0 })
    const started = now()
    const tick = () => {
      if (current !== token) return
      const t = now()
      if (judge.sample(win.innerWidth, win.innerHeight, win.devicePixelRatio, t)) {
        invoke('panel_dpi_settled', { token }).catch(() => {})
        return
      }
      if (t - started < SETTLE.giveUpMs) raf(tick)
    }
    raf(tick)
  })
}
