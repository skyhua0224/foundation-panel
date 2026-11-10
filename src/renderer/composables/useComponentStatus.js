import { createComponentStatus, microphoneState, registerComponentStatus } from 'foundation-ui/vue'
import { ElMessage, ElMessageBox } from './notifyMessage.js'
import { sunshine, vdd, vigem, vmouse } from '../tauri-adapter.js'
import { enhancementActions, enhancementSources } from './useEnhancementComponents.js'
import { sparxieActions } from './sparxieInstall.js'
import { dualsenseStatusChannel, micStatusChannel, usbipStatusChannel, vigemStatusChannel } from './deviceStatusStore.js'

// The component status of the control panel: the sources are the ones the panel already reads (the status channels of the device hub, the
// VDD status, the host configuration through the local proxy). The page and the home cards read the one controller registered here;
// a check asks the source again (`force`), the cheap read uses the channel caches.
let controller = null

const data = (result) => (result?.success ? result.data : null)
const channel = (ch) => ({ read: async () => data(await ch.fetch({ force: false })), fresh: async () => data(await ch.fetch({ force: true })) })

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
// Uninstalling a driver is tier 3: it says what stops working and asks first; the card is read again afterwards.
function removeAction(id, uninstall) {
  return {
    label: (S) => S.uninstall.yes,
    run: async () => {
      const S = controller.text()
      const name = S.names[id]
      try { await ElMessageBox.confirm(S.uninstall.text.replaceAll('{name}', name), S.uninstall.title.replace('{name}', name), { type: 'warning', danger: true, confirmButtonText: S.uninstall.yes }) } catch { return }
      try {
        const result = await uninstall()
        if (result && result.success === false) throw new Error(result.message || '')
        ElMessage.success(S.uninstall.done.replace('{name}', name))
      } catch (error) {
        ElMessage.error(`${S.uninstall.failed.replace('{name}', name)} ${String(error?.message || '')}`.trim())
      }
      await controller.check(id)
    },
  }
}

export function useComponentStatus({ locale, go = {}, update = () => ({}), check = () => {} } = {}) {
  if (controller) return controller
  const open = (fn, label) => ({ run: () => fn?.(), ...(label ? { label } : {}) })
  const both = (fn) => ({ update: open(fn, (T) => T.updateRepair), repair: undefined })
  // to the maintenance panels under the cards: the page scrolls there once it is open
  const maintenance = () => { go.components?.(); setTimeout(() => controller?.hooks?.scrollToMaintenance?.(), 350) }
  controller = registerComponentStatus(createComponentStatus({
    locale,
    openPage: go.page,
    sources: {
      vdd: async () => data(await vdd.getStatus()),
      usb: channel(usbipStatusChannel),
      vigem: channel(vigemStatusChannel),
      mouse: async () => data(await vmouse.getStatus()),
      vulkan: async () => { try { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); const r = await fetch(`${base}/api/vulkan-hdr-bridge`, { signal: AbortSignal.timeout(4000) }); return r.ok ? r.json() : null } catch { return null } },
      // what Sunshine has built in is not a component: these three checks feed the 手柄 page, the 编码器 page and 关于 / 软件更新
      dualsense: channel(dualsenseStatusChannel),
      mic: async () => { const s = data(await micStatusChannel.fetch({ force: false })); return s ? { state: microphoneState(s), backend: s.active_backend || s.configured_backend } : null },
      ...enhancementSources(),
      // Sparxie is an extension the person installs themselves: its card is the install entry, so the source answers even when it is not installed
      sparxie: async () => { try { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); const r = await fetch(`${base}/api/sparxie/status`, { signal: AbortSignal.timeout(4000) }); if (!r.ok) return null; return await r.json() } catch { return null } },
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
      vdd: { ...both(go.vdd), remove: removeAction('vdd', () => vdd.uninstall()) },
      usb: both(maintenance),
      vigem: { ...both(maintenance), remove: removeAction('vigem', () => vigem.uninstall()) },
      mouse: { ...both(maintenance), remove: removeAction('mouse', () => vmouse.uninstall()) },
      vulkan: { open: open(go.vdd) },
      mic: { open: open(go.microphone) },
      dlssnr: enhancementActions('dlssnr', { texts: () => controller.text(), refresh: () => controller.check('dlssnr') }),
      rtxhdr: enhancementActions('rtxhdr', { texts: () => controller.text(), refresh: () => controller.check('rtxhdr') }),
      host: { update: open(check) },
      sparxie: sparxieActions({ base: async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''), texts: () => controller.text(), refresh: () => controller.refresh(['sparxie']), progress: (value) => controller.setProgress('sparxie', value) }),
    },
  }))
  return controller
}
