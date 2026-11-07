import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp } from 'vue'
import 'foundation-ui/all.css'
import '../styles/global.less'
import './desktop.less'
import App from './DesktopApp.vue'

const app = createApp(App)
app.mount('#app')

