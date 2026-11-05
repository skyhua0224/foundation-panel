import { createApp } from 'vue';
import LogConsoleApp from './LogConsoleApp.vue';
import 'foundation-ui/all.css';
import { applyTheme } from 'foundation-ui';

// The log console is a window of its own and always dark: it gets the Material 3 colours of the dark scheme, so its dialogs and snackbars (LogConsoleApp
// mounts its own hosts) are the ones of the control panel in the same scheme.
applyTheme(document.documentElement);
document.documentElement.dataset.theme = 'dark';
document.documentElement.dataset.bsTheme = 'dark';

const app = createApp(LogConsoleApp);
app.mount('#app');
