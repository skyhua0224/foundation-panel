import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { KEEP_ON_RESET, resetStorage } from './developerTools.js'
import { STORAGE_KEY as AI_CONFIG_KEY } from './aiProviders.js'
import { CONSENT_KEY, ERRORS_KEY, ERRORS_NOTICE_KEY } from 'foundation-ui/telemetry'
import { LOCALE_KEY } from 'foundation-ui/locale/localePref.js'

// 恢复默认偏好 of the panel: what the panel itself keeps goes back to the defaults, the host's settings are not touched.
// Kept: the developer mode and the choices of its overlays, the privacy choices, the connection settings of the AI assistant (an account the person entered, not a preference), and the
// language: the host and the tray follow the language of the panel, so clearing it would change the language of the host on the next start.
export const PRIVACY_KEYS = [CONSENT_KEY, ERRORS_KEY, ERRORS_NOTICE_KEY]
export const LANGUAGE_KEYS = [LOCALE_KEY, 'language']
export const KEEP_ON_PREFERENCES_RESET = Object.freeze([...KEEP_ON_RESET, ...PRIVACY_KEYS, ...LANGUAGE_KEYS, AI_CONFIG_KEY])

/**
 * The native files (the window's settings, the tray icon's record, the developer overrides) go back to their defaults through `reset_panel_preferences`; then what the window
 * stores itself (theme, overview layout, pins, the first-run guide) is cleared, except the kept keys; then the window loads again. Rejects, with nothing cleared, when the native side refuses.
 */
export async function resetPanelPreferences({ invoke = tauriInvoke, local = globalThis.localStorage, session = globalThis.sessionStorage, reload = () => globalThis.location?.reload(), keep = KEEP_ON_PREFERENCES_RESET } = {}) {
  await invoke('reset_panel_preferences')
  const cleared = resetStorage(local, keep)
  resetStorage(session, [])
  reload()
  return cleared
}
