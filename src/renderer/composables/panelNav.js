import { ROUTES, RAILS } from './useRouter.js'

/**
 * What the list pane of the panel shows: the pages of a category with the icon, tone and a second line (`runtime.lines`, from foundation-ui's rowLines: the live
 * state where the page has one, otherwise what is inside; never blank).
 */
export function railItems(t) {
  const n = t.m3e.nav
  const icons = { overview: 'home', stream: 'cast_connected', config: 'tune', input: 'sports_esports', audio: 'volume_up', display: 'desktop_windows', system: 'build', assistant: 'smart_toy', settings: 'settings', help: 'help' }
  return RAILS.map((id) => ({ id, icon: icons[id], label: n[id] }))
}

/** The row id of foundation-ui's rowLines for a page of the list panes (the second line and the search words of the page). */
export const ROW_OF_ROUTE = {
  [ROUTES.WEB_CLIENTS]: 'clients', [ROUTES.WEB_APPS]: 'apps', [ROUTES.WEB_DOWNLOADS]: 'downloads', [ROUTES.GAMEPAD]: 'gamepad', [ROUTES.WEB_INPUT_KEYBOARD]: 'keyboard', [ROUTES.WEB_INPUT_MOUSE]: 'mouse', [ROUTES.MICROPHONE]: 'microphone', [ROUTES.USB]: 'usb',
  [ROUTES.WEB_DISPLAY_OUTPUT]: 'display-output', [ROUTES.WEB_DISPLAY_MODES]: 'display-modes', [ROUTES.WEB_DISPLAY_ENCODERS]: 'display-encoders', [ROUTES.WEB_DISPLAY_VIDEO]: 'display-video', [ROUTES.WEB_DISPLAY_HDR]: 'display-hdr', [ROUTES.WEB_AUDIO_SOUND]: 'audio-output',
  [ROUTES.VDD_SETTINGS]: 'vdd', [ROUTES.HDR_ENHANCED]: 'quality', [ROUTES.COMPONENT_STATUS]: 'components', [ROUTES.DIAGNOSTICS]: 'diagnostics', [ROUTES.NETWORK_OVERVIEW]: 'network', [ROUTES.TOOLBOX]: 'toolbox',
  [ROUTES.LATENCY_CLOCK]: 'latency-clock', [ROUTES.TEST_TOOLS]: 'test-tools', [ROUTES.STREAM_TIMER]: 'stream-timer',
  [ROUTES.PANEL_SETTINGS]: 'panel', [ROUTES.HELP]: 'help', [ROUTES.AI_ASSISTANT]: 'assistant',
  [ROUTES.LIVE_SESSION]: 'session', [ROUTES.RTSS]: 'rtss', [ROUTES.SUNSHINE_SERVICE]: 'service',
}

export function pageItems(t, rail, runtime = {}) {
  const c = t.m3e
  const lines = runtime.lines || {}
  const developer = runtime.developer ? [{ id: ROUTES.DEVELOPER, icon: 'code', tone: 'tertiary', label: runtime.developerTitle || 'Developer options', text: runtime.developerLine || '' }] : []
  const pages = {
    stream: [
      { id: ROUTES.WEB_CLIENTS, icon: 'devices', tone: 'tertiary', label: c.stream.clients[0], text: lines.clients },
      { id: ROUTES.WEB_APPS, icon: 'apps', tone: 'secondary', label: c.stream.apps[0], text: lines.apps },
      { id: ROUTES.WEB_DOWNLOADS, icon: 'download', tone: 'success', label: c.stream.downloads[0], text: lines.downloads },
      { id: ROUTES.LIVE_SESSION, icon: 'cast', tone: 'warn', label: c.live.title, text: lines.session },
    ],
    input: [
      { id: ROUTES.GAMEPAD, icon: 'sports_esports', tone: 'primary', label: c.input.gamepad, text: lines.gamepad },
      { id: ROUTES.WEB_INPUT_KEYBOARD, icon: 'keyboard', tone: 'secondary', label: c.input.keyboard, text: lines.keyboard },
      { id: ROUTES.WEB_INPUT_MOUSE, icon: 'mouse', tone: 'tertiary', label: c.input.mouse, text: lines.mouse },
      { id: ROUTES.USB, icon: 'usb', tone: 'success', label: c.input.usb, text: lines.usb },
    ],
    audio: [
      { id: ROUTES.WEB_AUDIO_SOUND, icon: 'speaker', tone: 'primary', label: c.audio.sound, text: lines['audio-output'] },
      { id: ROUTES.MICROPHONE, icon: 'mic', tone: 'tertiary', label: c.input.microphone, text: lines.microphone },
    ],
    display: [
      { id: ROUTES.WEB_DISPLAY_MODES, icon: 'aspect_ratio', tone: 'secondary', label: c.display.modes, text: lines['display-modes'] },
      { id: ROUTES.WEB_DISPLAY_OUTPUT, icon: 'monitor', tone: 'primary', label: c.display.output, text: lines['display-output'] },
      { id: ROUTES.WEB_DISPLAY_ENCODERS, icon: 'memory', tone: 'secondary', label: c.display.encoders, text: lines['display-encoders'] },
      { id: ROUTES.WEB_DISPLAY_VIDEO, icon: 'videocam', tone: 'primary', label: c.display.video, text: lines['display-video'] },
      { id: ROUTES.WEB_DISPLAY_HDR, icon: 'hdr_on', tone: 'warn', label: c.display.hdr, text: lines['display-hdr'] },
      { id: ROUTES.VDD_SETTINGS, icon: 'desktop_windows', tone: 'success', label: c.display.vdd, text: lines.vdd },
      { id: ROUTES.HDR_ENHANCED, icon: 'wand_stars', tone: 'tertiary', label: c.display.quality, text: lines.quality },
    ],
    system: [
      { id: ROUTES.SUNSHINE_SERVICE, icon: 'dns', tone: 'primary', label: runtime.serviceTitle || 'Sunshine', text: lines.service },
      { id: ROUTES.COMPONENT_STATUS, icon: 'monitor_heart', tone: 'success', label: runtime.statusTitle || 'Components', text: lines.components },
      { id: ROUTES.NETWORK_OVERVIEW, icon: 'lan', tone: 'primary', label: c.system.network, text: lines.network },
      { id: ROUTES.DIAGNOSTICS, icon: 'stethoscope', tone: 'warn', label: runtime.diagTitle || c.system.diagnostics, text: lines.diagnostics },
      { id: ROUTES.DOCTOR, icon: 'health_and_safety', tone: 'success', label: runtime.doctorTitle || 'Health check', text: lines.doctor },
      { id: ROUTES.TOOLBOX, icon: 'build', tone: 'secondary', label: t.sidebar.toolbox, text: lines.toolbox },
      // the tools people come for have a row of their own: 系统 then the tool is two presses (inside the toolbox it was three)
      { id: ROUTES.LATENCY_CLOCK, icon: 'speed', tone: 'primary', label: t.sidebar.latencyClock, text: lines['latency-clock'] },
      { id: ROUTES.TEST_TOOLS, icon: 'science', tone: 'secondary', label: t.sidebar.testTools, text: lines['test-tools'] },
      { id: ROUTES.STREAM_TIMER, icon: 'timer', tone: 'tertiary', label: t.sidebar.streamTimer, text: lines['stream-timer'] },
      // RivaTuner Statistics Server is a third-party component: the row exists only where it is installed (rtssInstall.js)
      ...(runtime.rtssInstalled ? [{ id: ROUTES.RTSS, icon: 'speed', tone: 'tertiary', label: c.rtss.title, text: lines.rtss }] : []),
      ...developer,
    ],
  }
  return pages[rail] || []
}

// The bottom bar of a narrow window (the Material navigation bar takes three to five destinations; nine crammed their labels): the overview, the stream,
// ONE entry for the four settings categories (it opens a list of them), 米塔 AI, and 更多 (panel settings, help and about, search, sponsor).
export const BAR_GROUP_OF = { overview: 'overview', stream: 'stream', config: 'settings-hub', input: 'settings-hub', audio: 'settings-hub', display: 'settings-hub', system: 'settings-hub', assistant: 'assistant', settings: 'more', help: 'more', appearance: 'more' }
export function barItems(t) {
  const n = t.m3e.nav
  return [
    { id: 'overview', icon: 'home', label: n.overview },
    { id: 'stream', icon: 'cast_connected', label: n.stream },
    { id: 'settings-hub', icon: 'tune', label: n.barSettings },
    { id: 'assistant', icon: 'smart_toy', label: n.assistant },
    { id: 'more', icon: 'more_horiz', label: n.barMore },
  ]
}
/** The bar entry that is current for a rail id. */
export const barCurrent = (rail) => BAR_GROUP_OF[rail] || 'overview'
