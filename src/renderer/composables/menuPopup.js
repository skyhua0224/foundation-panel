// The context menu in a window of its own (src-tauri/src/ctx_popup.rs): the side of the page that owns the menu, and the side of the menu window's page.
//
// A menu built in the main window (foundation-ui MContextProvider / MContextMenu) is handed to `present`, which sends it as data to the native side together with the theme of this window and where
// it was opened. The native side shows a borderless window at the pointer, which may extend beyond the main window; when the menu ends the main window gets the path of the chosen item
// (`ctx-popup:result`) and the page runs that item's own action. If there is no menu window, or it fails, `present` answers `{ handled: false }` and the page draws the menu itself.

import { handleFrameMenuMessage, setMenuPresenter, toMenuModel } from 'foundation-ui/contextmenu/presenter.js'
import { themeSnapshot } from 'foundation-ui/contextmenu/popup.js'

const TIMINGS_KEPT = 60

/**
 * Installs the presenter of this window. Returns `{ present, frameMessage, stop }`:
 *   present(req)       the presenter itself (also what the frames' menus go through)
 *   frameMessage(event, frame)  answers the menu messages of a frame (foundation-ui handleFrameMenuMessage); true when the message was one
 *   timings            window.__ctxPopupTimings: per menu, the milliseconds from the request to the shown window (read by the measurement runs)
 */
export function installMenuPopup({ invoke, listen, backLabel = () => 'Back', theme = () => themeSnapshot(globalThis.document), win = globalThis, now = () => globalThis.performance.now() }) {
  const pending = new Map()
  const base = Math.floor(Math.random() * 1e9) * 1000 + 1000
  let seq = 0
  let stopListening = () => {}
  let stopped = false
  const timings = (win.__ctxPopupTimings = win.__ctxPopupTimings || [])
  Promise.resolve(listen('ctx-popup:result', (event) => {
    const token = event?.payload?.token
    const entry = pending.get(token)
    if (!entry) return
    pending.delete(token)
    const path = event.payload.path
    entry({ handled: true, path: Array.isArray(path) && path.length ? path : null })
  })).then((off) => { if (stopped) off?.(); else stopListening = off || (() => {}) }).catch(() => {})

  async function present({ items, point, pointer }) {
    const token = base + ++seq
    const started = now()
    const done = new Promise((resolve) => pending.set(token, resolve))
    try {
      await invoke('ctx_popup_open', {
        request: {
          token, items: toMenuModel(items), back: backLabel(), theme: theme(), pointer: pointer === true,
          clientX: point?.x ?? 0, clientY: point?.y ?? 0, ratio: win.devicePixelRatio || 1,
        },
      })
    } catch {
      // no menu window (not created yet, switched off, a failure): the page draws the menu
      pending.delete(token)
      return { handled: false }
    }
    timings.push({ token, ms: Math.round((now() - started) * 10) / 10 })
    if (timings.length > TIMINGS_KEPT) timings.splice(0, timings.length - TIMINGS_KEPT)
    return done
  }
  const stopPresenter = setMenuPresenter(present)
  return {
    present,
    timings,
    frameMessage: (event, frame) => handleFrameMenuMessage(event, { frame, present, reply: (message) => frame?.contentWindow?.postMessage(message, event.origin && event.origin !== 'null' ? event.origin : '*') }),
    stop() { stopped = true; stopPresenter(); stopListening(); for (const resolve of pending.values()) resolve({ handled: false }); pending.clear() },
  }
}
