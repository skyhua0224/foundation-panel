import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp } from 'vue'
import 'foundation-ui/all.css'
import { registerViviAssets } from 'foundation-ui/brand-assets'  // 薇薇 and her wallpapers: the library draws none until a product registers them
import '../styles/global.less'
import './desktop.less'
import App from './DesktopApp.vue'
import { i18nReady } from './i18n/index.js'

registerViviAssets()
const app = createApp(App)
i18nReady.then(() => app.mount('#app'))

