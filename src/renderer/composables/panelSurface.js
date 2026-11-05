// The colour the native window and WebView2 show before a frame is painted (first start, restore, resize) is the panel's own surface colour.
// The panel reads its surface role whenever the root element changes and tells the native side, which keeps the last one for the next start.
// WebView2: ICoreWebView2Controller2::DefaultBackgroundColor
// https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2controller2#get_defaultbackgroundcolor

/**
 * Reports `read()` (an [r, g, b] array) through `send` after the root settled; the same colour twice is sent once.
 * `schedule(fn)` runs fn later (idle time) and returns a cancel function.
 */
export function createSurfaceReporter({ read, send, schedule }) {
  let last = ''
  let cancel = null
  const run = () => {
    cancel = null
    const rgb = read()
    if (!rgb) return
    const key = rgb.join(',')
    if (key === last) return
    last = key
    send(rgb)
  }
  return {
    request() {
      if (cancel) cancel()
      cancel = schedule(run)
    },
    flush: run,
  }
}

function rgbOfSurface(doc) {
  const probe = doc.createElement('i')
  probe.style.cssText = 'position:fixed;width:0;height:0;pointer-events:none;background-color:var(--md-sys-color-surface)'
  doc.body.appendChild(probe)
  const css = doc.defaultView.getComputedStyle(probe).backgroundColor
  probe.remove()
  const canvas = doc.createElement('canvas')
  canvas.width = canvas.height = 1
  const ctx = canvas.getContext('2d', { willReadFrequently: true })
  if (!ctx) return null
  ctx.clearRect(0, 0, 1, 1)
  ctx.fillStyle = css
  ctx.fillRect(0, 0, 1, 1)
  const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data
  return a === 255 ? [r, g, b] : null
}

/** Starts reporting the document's surface colour; `invoke` is Tauri's invoke. Returns a stop function. */
export function followPanelSurface(invoke, doc = document) {
  const idle = (fn) => {
    const id = setTimeout(fn, 200)
    return () => clearTimeout(id)
  }
  const reporter = createSurfaceReporter({
    read: () => rgbOfSurface(doc),
    send: ([r, g, b]) => invoke('panel_set_surface', { r, g, b }).catch(() => {}),
    schedule: idle,
  })
  const observer = new MutationObserver(() => reporter.request())
  observer.observe(doc.documentElement, { attributes: true, attributeFilter: ['style', 'class', 'data-bs-theme'] })
  reporter.request()
  return () => observer.disconnect()
}
