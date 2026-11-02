import { createComponentStatus, microphoneState, registerComponentStatus } from 'foundation-ui/vue'
import { sunshine, vdd, vigem } from '../tauri-adapter.js'
import { dualsenseStatusChannel, micStatusChannel, usbipStatusChannel, vigemStatusChannel } from './deviceStatusStore.js'

// The component status of the control panel: the sources are the ones the panel already reads (the status channels of the device hub, the
// VDD status, the host configuration through the local proxy). The page and the home cards read the one controller registered here;
// a check asks the source again (`force`), the cheap read uses the channel caches.
let controller = null

const data = (result) => (result?.success ? result.data : null)
const channel = (ch) => ({ read: async () => data(await ch.fetch({ force: false })), fresh: async () => data(await ch.fetch({ force: true })) })
const signedOf = (usb) => (usb?.signature === 'signed' ? true : usb?.signature === 'unsigned' || usb?.signature === 'invalid' ? false : null)

async function hostConfig() {
  const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
  const response = await fetch(`${base}/api/config`, { signal: AbortSignal.timeout(4000) })
  if (!response.ok) throw new Error(`HTTP ${response.status}`)
  return response.json()
}

/**
 * `go` = { vdd, components, microphone, gamepad, page } navigation callbacks; `update()` returns { state, version, next };
 * `check()` runs the panel's update check and opens its update dialog.
 */
export function useComponentStatus({ locale, go = {}, update = () => ({}), check = () => {} } = {}) {
  if (controller) return controller
  const open = (fn, label) => ({ run: () => fn?.(), ...(label ? { label } : {}) })
  const both = (fn) => ({ update: open(fn, (T) => T.updateRepair), repair: undefined })
  controller = registerComponentStatus(createComponentStatus({
    locale,
    openPage: go.page,
    sources: {
      vdd: async () => data(await vdd.getStatus()),
      usb: channel(usbipStatusChannel),
      dualsense: channel(dualsenseStatusChannel),
      drivers: {
        read: async () => driversRaw(false),
        fresh: async () => driversRaw(true),
      },
      mic: async () => { const s = data(await micStatusChannel.fetch({ force: false })); return s ? { state: microphoneState(s), backend: s.active_backend || s.configured_backend } : null },
      encoder: async () => { const c = await hostConfig(); return { active: c.active_encoder, hevc: c.hevc_mode, av1: c.av1_mode } },
      host: async () => {
        let version = ''
        try { version = (await sunshine.getVersion()) || '' } catch { /* the version stays empty */ }
        let reachable = true
        try { await hostConfig() } catch { reachable = false }
        const u = update() || {}
        return { version: version === 'Unknown' ? '' : version, reachable, update: u.state || '', next: u.next || '' }
      },
    },
    actions: {
      vdd: both(go.vdd),
      usb: both(go.components),
      dualsense: both(go.components),
      drivers: both(go.components),
      mic: { open: open(go.microphone) },
      host: { update: open(check) },
    },
  }))
  return controller
}

// The drivers the panel can see: the USB/IP transport (signed by the signer it reports), ViGEmBus, the virtual display driver
async function driversRaw(force) {
  const [usb, pad, display] = await Promise.all([
    force ? usbipStatusChannel.fetch({ force: true }) : usbipStatusChannel.fetch({ force: false }),
    force ? vigemStatusChannel.fetch({ force: true }) : vigemStatusChannel.fetch({ force: false }),
    vdd.getStatus(),
  ])
  const u = data(usb)
  const g = data(pad)
  const v = data(display)
  const items = []
  if (u && u.supported !== false) items.push({ id: 'usbip', name: 'USB/IP', installed: !!u.installed, signed: signedOf(u), signer: u.signer || '', version: u.version || '' })
  if (g) items.push({ id: 'vigem', name: 'ViGEmBus', installed: !!g.installed, signed: null, version: g.version || '' })
  if (v) items.push({ id: 'vdd', name: 'ZakoDisplay', installed: !!v.installed, signed: null, version: v.installed_version || v.version || '' })
  return { items }
}
