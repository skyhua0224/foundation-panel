import { tauriInvoke } from './composables/useTauri.js'

// The rows "在 Panel 中打开": what a sofa does not need often lives in the main panel. Each row opens one page of it through the host command
// open_panel_page, which asks the one main window for the page (src-tauri windows.rs open_main_panel_page; the names are PANEL_PAGES of
// composables/panelParts.js, and a test keeps the Rust list, that list and this table together). The texts are i18n panel.links.<key>.
export const PANEL_LINKS = Object.freeze({
  toolPaths: { page: 'tool-paths', icon: 'bolt', tone: 'primary' },
  fileSharing: { page: 'file-sharing', icon: 'folder_shared', tone: 'tertiary' },
  pet: { page: 'pet', icon: 'pets', tone: 'secondary' },
  advanced: { page: 'panel-advanced', icon: 'tune', tone: 'neutral' },
  displayModes: { page: 'display-modes', icon: 'monitor', tone: 'primary' },
  vdd: { page: 'vdd', icon: 'tv_gen', tone: 'tertiary' },
  vmouse: { page: 'input-drivers', icon: 'mouse', tone: 'secondary' },
  liveSession: { page: 'live-session', icon: 'monitor_heart', tone: 'primary' },
  dpi: { page: 'desktop-scale', icon: 'aspect_ratio', tone: 'tertiary' },
  shortcuts: { page: 'shortcuts', icon: 'keyboard', tone: 'secondary' },
  logs: { page: 'logs', icon: 'stethoscope', tone: 'neutral' },
})

/** Ask the host to open a page of the main panel; resolves true when it did, false when there is no host (a browser) or it refused. */
export async function openInPanel(page) {
  try {
    await tauriInvoke('open_panel_page', { page })
    return true
  } catch (error) {
    console.error('Failed to open the panel page:', page, error)
    return false
  }
}
