// The monitor the stream captures, for the latency test clock's 「在串流显示器上全屏打开」. Sunshine's output_name (/api/config) is the device id of one of its
// display_devices, whose description names the Windows display (\\.\DISPLAYn); the window system's monitor of that name is the one. With no output_name Sunshine
// captures the primary display. Pure: the caller reads the config and the monitors (Tauri's availableMonitors, primaryMonitor).

const displayOf = (text) => String(text || '').match(/DISPLAY\d+/i)?.[0].toUpperCase() || ''

const squash = (text) => String(text || '').toLowerCase().replace(/[^a-z0-9]/g, '')
const friendlyOf = (data) => String(data || '').match(/FRIENDLY NAME:[ \t]*([^\n]*)/i)?.[1] || ''

/**
 * The Windows display (DISPLAYn) Sunshine captures, or '' (auto, or a device it does not describe). output_name is a device id, or the name of a display the host made itself
 * (the virtual display writes 「ZakoHDR」 for the device whose friendly name is 「Zako HDR」): a name that equals no id is matched with the friendly names, ignoring case and spaces.
 */
export function capturedDisplay(outputName, displayDevices = []) {
  if (!outputName) return ''
  const list = Array.isArray(displayDevices) ? displayDevices : []
  const idOf = (d) => (typeof d === 'string' ? d : d?.device_id || d?.id)
  const dataOf = (d) => (typeof d === 'string' ? d : d?.data || d?.name || '')
  const dev = list.find((d) => idOf(d) === outputName) || list.find((d) => squash(friendlyOf(dataOf(d))) === squash(outputName))
  return displayOf(dataOf(dev)) || displayOf(outputName)
}

/** { monitor, found }: the monitor of the captured display; else the primary one, found false when Sunshine named a display that is not there. */
export function pickStreamMonitor({ outputName = '', displayDevices = [], monitors = [], primary = null } = {}) {
  const fallback = primary || monitors[0] || null
  if (!outputName) return { monitor: fallback, found: !!fallback }
  const want = capturedDisplay(outputName, displayDevices)
  const monitor = want ? monitors.find((m) => displayOf(m?.name) === want) : null
  return monitor ? { monitor, found: true } : { monitor: fallback, found: false }
}

/** A window of width × height centred on a monitor, in logical pixels (a Tauri monitor is in physical pixels). */
export function centredOn(monitor, width, height) {
  const s = monitor?.scaleFactor || 1
  const x = monitor.position.x / s, y = monitor.position.y / s, w = monitor.size.width / s, h = monitor.size.height / s
  return { x: Math.round(x + Math.max(0, (w - width) / 2)), y: Math.round(y + Math.max(0, (h - height) / 2)), width: Math.min(width, w), height: Math.min(height, h) }
}
