import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp, watchEffect } from 'vue';
import './icons.css';
import { useI18n, i18nReady } from '../desktop/i18n/index.js';

const { t } = useI18n();
watchEffect(() => { document.title = t.value.windowTitles.toolWindow; });
import ToolWindow from './ToolWindow.vue';

const app = createApp(ToolWindow);
i18nReady.then(() => app.mount('#app'));

