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
