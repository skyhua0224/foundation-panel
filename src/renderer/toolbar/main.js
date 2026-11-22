import '../../../src-tauri/inject-traces.js'  // no trace of a browser: drag ghosts, browser keys and menus, zoom, selected chrome
import { createApp, watchEffect } from 'vue';
import 'foundation-ui/all.css';
import { useI18n, i18nReady } from '../desktop/i18n/index.js';

const { t } = useI18n();
watchEffect(() => { document.title = t.value.windowTitles.toolbar; });
import ToolbarApp from './ToolbarApp.vue';

const app = createApp(ToolbarApp);
i18nReady.then(() => app.mount('#app'));

