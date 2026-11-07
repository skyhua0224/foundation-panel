import { ref } from 'vue'

// RivaTuner Statistics Server is an optional third-party component: its page (row, search entry, hub row, tile text) exists only where it is installed.
// get_rtss_status carries `installed` (src-tauri/src/rtss.rs detect_rtss_install_dir: the registry key or the default folder). It is read once at start;
// the RTSS page reads it again on its refresh and tells this module, so an uninstall closes the page. null = not read yet.
export const rtssInstalled = ref(null)

/** installed of one get_rtss_status answer; a build whose answer has no such field is judged by what it found (running, rtss-cli, the hooks dll). */
export const installedOf = (status) => {
  if (!status || typeof status !== 'object') return false
  if (typeof status.installed === 'boolean') return status.installed
  return !!(status.running || status.cli_path || status.hooks_dll_path)
}

/** Record an answer of get_rtss_status (the RTSS page calls it after each refresh). */
export const noteRtssStatus = (status) => { rtssInstalled.value = installedOf(status) }

/** The one read at start. A failed read (not in the app window, no such command) is "not installed": the page is not offered. */
export async function readRtssInstall(invoke) {
  try { noteRtssStatus(await invoke('get_rtss_status')) } catch { rtssInstalled.value = false }
  return rtssInstalled.value
}
