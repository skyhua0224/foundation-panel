import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp } from 'vue'
import '../styles/global.less'
import App from './index.vue'

const app = createApp(App);

app.mount('#app');
