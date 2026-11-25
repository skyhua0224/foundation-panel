// The installer's setup UI page (installer-ui/): foundation-ui's installer pages on the bridge to the installer
// shell (foundation-ui src/vue/installer/bridge.js). Outside the setup UI the bridge is the scripted mock.
import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom
import { createApp } from 'vue'
import 'foundation-ui/all.css'
import { registerViviAssets } from 'foundation-ui/brand-assets'  // the logo of the rail
import { createBridge } from 'foundation-ui/vue/installer/bridge.js'
import MInstaller from 'installer-root'

registerViviAssets()
createApp(MInstaller, { bridge: createBridge(), windowed: true }).mount('#app')
