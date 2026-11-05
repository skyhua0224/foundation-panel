import { createApp } from 'vue'
import 'foundation-ui/all.css'
import '../styles/global.less'
import './desktop.less'
import App from './DesktopApp.vue'

const app = createApp(App)
app.mount('#app')

