import { ROUTES, RAILS } from './useRouter.js'

/**
 * What the list pane of the panel shows: the pages of a category with the icon, tone and one line of live data.
 * `live` text comes only from real sources (the device runtime); a page without one shows no second line.
 */
export function railItems(t) {
  const n = t.m3e.nav
  const icons = { overview: 'home', stream: 'cast_connected', config: 'tune', input: 'sports_esports', display: 'desktop_windows', system: 'build', settings: 'settings', help: 'help' }
  return RAILS.map((id) => ({ id, icon: icons[id], label: n[id] }))
}

export function pageItems(t, rail, runtime = {}) {
  const c = t.m3e
  const ds = runtime.ds?.state || ''
  const dsReady = ['ready', 'active'].includes(ds)
  const pages = {
    stream: [
      { id: ROUTES.WEB_CLIENTS, icon: 'devices', tone: 'tertiary', label: c.stream.clients[0] },
      { id: ROUTES.WEB_APPS, icon: 'apps', tone: 'secondary', label: c.stream.apps[0] },
      { id: ROUTES.WEB_DOWNLOADS, icon: 'download', tone: 'success', label: c.stream.downloads[0] },
    ],
    input: [
      { id: ROUTES.GAMEPAD, icon: 'sports_esports', tone: 'primary', label: c.input.gamepad, text: dsReady ? (t.dualSense.status.states[ds] || t.deviceHub.available) : '' },
      { id: ROUTES.MICROPHONE, icon: 'mic', tone: 'tertiary', label: c.input.microphone, text: runtime.micLabel || '' },
      { id: ROUTES.USB, icon: 'usb', tone: 'success', label: c.input.usb, text: runtime.usb?.ready ? t.deviceHub.available : '' },
    ],
    display: [
      { id: ROUTES.VDD_SETTINGS, icon: 'desktop_windows', tone: 'primary', label: c.display.vdd },
      { id: ROUTES.HDR_ENHANCED, icon: 'wand_stars', tone: 'tertiary', label: c.display.quality },
    ],
    system: [
      { id: ROUTES.COMPONENT_STATUS, icon: 'monitor_heart', tone: 'success', label: runtime.statusTitle || 'Components' },
      { id: ROUTES.DIAGNOSTICS, icon: 'stethoscope', tone: 'warn', label: runtime.diagTitle || c.system.diagnostics },
      { id: ROUTES.TOOLBOX, icon: 'build', tone: 'secondary', label: t.sidebar.toolbox },
      { id: ROUTES.AI_ASSISTANT, icon: 'smart_toy', tone: 'tertiary', label: c.system.assistant },
    ],
  }
  return pages[rail] || []
}
