import { invoke } from '@tauri-apps/api/core'

/** The version of this panel (the build's own), '' where it cannot be read (a browser, a test): the guide then opens on a first run only. */
export async function panelVersion() {
  try { return String((await invoke('get_system_info'))?.app_version || '') } catch { return '' }
}
