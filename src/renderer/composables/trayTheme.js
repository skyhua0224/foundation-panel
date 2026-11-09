// The tray icon's menus open without a main window to ask for the colours (the app may be in the tray only): this window tells the native side its theme whenever the appearance changes
// (src-tauri/src/ctx_popup.rs `tray_popup_theme_set`), the same snapshot the context menus carry (foundation-ui themeSnapshot).

import { themeSnapshot } from 'foundation-ui/contextmenu/popup.js'

/**
 * Starts reporting. Returns a function that stops it. `invoke` is Tauri's; `doc` the document to watch; `delay` how long a burst of changes is collected (the appearance sets many attributes at once).
 */
export function installTrayTheme({ invoke, doc = globalThis.document, delay = 400, snapshot = themeSnapshot, setTimer = setTimeout, clearTimer = clearTimeout, Observer = globalThis.MutationObserver } = {}) {
  if (!doc?.documentElement || typeof Observer !== 'function') return () => {}
  let timer = 0
  let last = ''
  const send = () => {
    timer = 0
    const snap = snapshot(doc)
    if (!snap) return
    const key = JSON.stringify(snap)
    if (key === last) return
    last = key
    Promise.resolve(invoke('tray_popup_theme_set', { theme: snap })).catch(() => {})
  }
  const schedule = () => { if (!timer) timer = setTimer(send, delay) }
  const observer = new Observer(schedule)
  observer.observe(doc.documentElement, { attributes: true })
  if (doc.head) observer.observe(doc.head, { childList: true, subtree: true, characterData: true })
  schedule()
  return () => { observer.disconnect(); if (timer) clearTimer(timer) }
}
