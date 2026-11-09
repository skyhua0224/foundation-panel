// The side of the menu window's page that talks to the native side (src-tauri/src/ctx_popup.rs). Kept apart from menuPopup.js so that the page of the window loads nothing of the main window's code.

/** The transport of the menu window's page (foundation-ui MMenuPopup): events in from the native side, commands out. */
export function popupTransport({ invoke, listen }) {
  const quiet = (promise) => Promise.resolve(promise).catch(() => {})
  return {
    onOpen: (cb) => listen('ctx-popup:open', (event) => cb(event.payload)),
    onClose: (cb) => listen('ctx-popup:close', (event) => cb(event.payload)),
    ready: (token, size) => quiet(invoke('ctx_popup_ready', { token, width: size.w, height: size.h })),
    pick: (token, path) => quiet(invoke('ctx_popup_pick', { token, path })),
    cancel: (token) => quiet(invoke('ctx_popup_cancel', { token })),
    loaded: () => quiet(invoke('ctx_popup_loaded')),
  }
}

/**
 * The transport of the tray menu's page (foundation-ui MTrayPopup): the tray icon opens a menu with the variant and the host state (`tray-popup:open`), the page reports the card's size
 * (`ctx_popup_ready`, which sizes and places the window against the taskbar), and an item goes back by its id (`tray_popup_act`: the native menu's handler runs it).
 */
export function trayTransport({ invoke, listen }) {
  const quiet = (promise) => Promise.resolve(promise).catch(() => {})
  return {
    onOpen: (cb) => listen('tray-popup:open', (event) => cb(event.payload)),
    onState: (cb) => listen('tray-popup:state', (event) => cb(event.payload)),
    onClose: (cb) => listen('ctx-popup:close', (event) => cb(event.payload)),
    ready: (token, size) => quiet(invoke('ctx_popup_ready', { token, width: size.w, height: size.h })),
    act: (token, id) => quiet(invoke('tray_popup_act', { token, id })),
    // a slider was released: unlike the others this one's answer matters (the page shows the busy state and the failure), so the rejection is not swallowed
    set: (token, key, value) => invoke('tray_popup_set', { token, key, value }),
    // the quick panel was edited (the pencil): the record { tiles, sliders } (null = the default); the host stores it and sends the state back
    layout: (token, record) => quiet(invoke('tray_popup_layout', { token, tiles: record?.tiles ?? null, sliders: record?.sliders ?? null })),
    cancel: (token) => quiet(invoke('ctx_popup_cancel', { token })),
    loaded: () => {}, // the context menu page reports the window as loaded
  }
}
