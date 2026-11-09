// The page of the context menu window (src-tauri/src/ctx_popup.rs): one Material menu (foundation-ui MMenuPopup, the same component as the in-page menu) in a transparent window.
import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import 'foundation-ui/all.css'  // Material 3 Expressive tokens and components
import { MMenuPopup } from 'foundation-ui/popup'
import { popupTransport } from '../composables/menuPopupTransport.js'

createApp({ render: () => h(MMenuPopup, { transport: popupTransport({ invoke, listen }) }) }).mount('#app')
