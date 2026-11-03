import { createApp } from 'vue'
import SunshineFrame from './components/SunshineFrame.vue'
import 'element-plus/theme-chalk/el-loading.css'
import 'element-plus/theme-chalk/el-message.css'
import 'element-plus/theme-chalk/el-overlay.css'
import 'element-plus/theme-chalk/el-message-box.css'
import 'element-plus/theme-chalk/el-notification.css'
import 'foundation-ui/all.css'  // Material 3 Expressive tokens, components and the Element Plus skin
import './styles/m3e-compat.less'  // restyles the pages that still use their own markup
// 导入 Tauri polyfill
import './tauri-polyfill.js'
import { getDefaultLocale, setLocale } from '../i18n/index.js'

// 初始化语言
setLocale(getDefaultLocale())

const app = createApp(SunshineFrame)
app.mount('#app')
