// The page of the context menu window (src-tauri/src/ctx_popup.rs): one Material menu (foundation-ui MMenuPopup, the same component as the in-page menu) in a transparent window.
import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import 'foundation-ui/all.css'  // Material 3 Expressive tokens and components
import { MMenuPopup, MTrayPopup } from 'foundation-ui/popup'
import { popupTransport, trayTransport } from '../composables/menuPopupTransport.js'

// one window, two pages in it: the context menus of the main window (MMenuPopup) and the tray icon's menus (MTrayPopup, src-tauri/src/tray/popup_input.rs); each answers only its own events
const menus = popupTransport({ invoke, listen })
const tray = trayTransport({ invoke, listen })
createApp({ render: () => [h(MMenuPopup, { transport: menus }), h(MTrayPopup, { transport: tray })] }).mount('#app')
