import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp } from 'vue'
import 'foundation-ui/all.css'
import '../styles/global.less'
import './desktop.less'
import App from './DesktopApp.vue'
import { i18nReady } from './i18n/index.js'

const app = createApp(App)
i18nReady.then(() => app.mount('#app'))

