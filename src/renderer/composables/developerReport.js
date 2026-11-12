import { invoke } from '@tauri-apps/api/core'
import { registerDevReport } from 'foundation-ui/vue'
import { environmentText } from 'foundation-ui/developer'
import { sunshine } from '../tauri-adapter.js'
import { desktopSettings, saveDesktopSettings } from '../desktop/composables/useDesktopSettings.js'
import { createDeveloperTools, logLine } from './developerTools.js'

/** base64 -> bytes (the window capture comes from the native side as a PNG in base64). */
export const bytesOfBase64 = (text) => {
  const raw = atob(String(text || ''))
  const out = new Uint8Array(raw.length)
  for (let i = 0; i < raw.length; i += 1) out[i] = raw.charCodeAt(i)
  return out
}

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
    // the native side captures the whole window (frames included); where it cannot, there is no screenshot and the sheet says so
    capture: async () => bytesOfBase64(await invoke('dev_capture_window')),
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
    // the report sheet sends privately to the developers (a user_report event through the host) or saves the zip: nothing public, no address to open
    product: 'panel',
  })
}
