import { invoke } from '@tauri-apps/api/core'
import { registerDevReport } from 'foundation-ui/vue'
import { environmentText } from 'foundation-ui/developer'
import { sunshine, openExternalUrl } from '../tauri-adapter.js'
import { desktopSettings, saveDesktopSettings } from '../desktop/composables/useDesktopSettings.js'
import { bytesOfCapture, createDeveloperTools, logLine } from './developerTools.js'

/** Gives foundation-ui's report and sign-in sheets what only the panel can: the capture of its window, the log, the environment, saving a file. */
export function registerPanelReport({ locale, version, errorContext, hostNames = () => [] }) {
  const tools = createDeveloperTools({
    invoke,
    settings: { get: () => desktopSettings.value, save: saveDesktopSettings },
    locale: locale(),
    version: version(),
    hostVersion: () => sunshine.getVersion(),
    base: () => sunshine.getProxyUrl(),
  })
  registerDevReport({
    // the native side captures the whole window (frames included) and answers the PNG as bytes, not as text; where it cannot, there is no screenshot and the sheet says so
    capture: async () => bytesOfCapture(await invoke('dev_capture_window')),
    logs: async () => (await invoke('get_all_logs')).map(logLine).join('\n'),
    env: async () => environmentText(await tools.environment()),
    hostNames,
    save: (name, bytes, mime) => {
      const link = document.createElement('a')
      link.href = URL.createObjectURL(new Blob([bytes], { type: mime }))
      link.download = name
      document.body.appendChild(link)
      link.click()
      link.remove()
      setTimeout(() => URL.revokeObjectURL(link.href), 4000)
    },
    // the sign-in sheet's button opens github.com/login/device through the native side (the system browser, also from an elevated panel); it answers false when it could not, and the sheet then says what to do by hand
    openUrl: (url) => openExternalUrl(url),
    // the report sheet sends privately to the developers (a user_report event through the host) or saves the zip. The host takes a screenshot with the report (uploads it like the run log), so the sheet
    // offers one: the capture above (covered before the sheet exists, 重新截取 takes it again) or a picture chosen or pasted (a window of this panel does not get HTML drops: the native side owns file drops).
    shots: true,
    product: 'panel',
  })
}
