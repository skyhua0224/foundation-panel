import { ElMessage, ElMessageBox } from './notifyMessage.js'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { EXTENSION_COMPONENTS } from 'foundation-ui/vue'
import { dlssNr, rtxHdr } from '../tauri-adapter.js'

// DLSS NR and RTX HDR are extension components: the person imports an NVIDIA DLL they obtained themselves (nvngx_dlssnr.dll for DLSS NR, nvngx_truehdr.dll for RTX HDR). The import
// records the digest of the file; "choose again" replaces it; removing asks first. The file chooser only takes the one DLL name, and a file of the other name is refused here as well,
// so the two are not mixed up. The words are the status texts of foundation-ui (zh and en); the host calls are the panel's own native component commands.
const API = { dlssnr: dlssNr, rtxhdr: rtxHdr }
const NAME = { dlssnr: 'DLSS NR', rtxhdr: 'RTX HDR' }
const fill = (text, values) => String(text).replace(/\{(\w+)\}/g, (_, k) => values[k] ?? `{${k}}`)
const baseName = (path) => String(path || '').split(/[\\/]/).pop().toLowerCase()
const data = (result) => (result?.success ? result.data : null)

export function enhancementSources() {
  return Object.fromEntries(Object.keys(API).map((id) => [id, async () => data(await API[id].getStatus())]))
}

/** The import and remove actions of a card; `refresh()` reads the component's status again after an operation. */
export function enhancementActions(id, { texts, refresh }) {
  const api = API[id]
  const { dll } = EXTENSION_COMPONENTS[id]
  const T = () => texts().ext
  const vars = () => ({ dll, name: NAME[id], other: EXTENSION_COMPONENTS[id].other })
  const busy = { on: false }
  const guard = async (work) => { if (busy.on) return; busy.on = true; try { await work() } finally { busy.on = false } }
  return {
    import: {
      label: (S, row) => (!row || row.state === 'missing' || row.state === 'unknown' ? S.ext.import : S.ext.reselect),
      run: () => guard(async () => {
        const status = data(await api.getStatus()) || {}
        if (status.in_use || status.maintenance) { ElMessage.warning(T().inUse); return }
        const path = await openDialog({ multiple: false, directory: false, title: fill(T().pick, vars()), filters: [{ name: dll, extensions: ['dll'] }] })
        if (!path) return
        if (baseName(path) !== dll) { ElMessage.error(fill(T().wrongFile, vars())); return }
        try { await ElMessageBox.confirm(`${fill(T().importConfirm, vars())}\n${fill(T().mixWarning, vars())}`, fill(status.installed ? T().reselectTitle : T().importTitle, vars()), { type: 'warning', confirmButtonText: status.installed ? T().reselect : T().import }) } catch { return }
        try {
          const result = await api.install(path)
          if (!result?.success) throw new Error(result?.message || '')
          ElMessage.success(T().importDone)
        } catch (error) {
          ElMessage.error(`${T().importFailed} ${String(error?.message || '')}`.trim())
        }
        await refresh()
      }),
    },
    remove: {
      label: (S) => S.ext.remove,
      run: () => guard(async () => {
        const status = data(await api.getStatus()) || {}
        if (!status.installed) return
        if (status.in_use || status.maintenance) { ElMessage.warning(T().inUse); return }
        try { await ElMessageBox.confirm(fill(T().removeConfirm, vars()), fill(T().removeTitle, vars()), { type: 'warning', danger: true, confirmButtonText: T().remove }) } catch { return }
        try {
          const result = await api.uninstall()
          if (!result?.success) throw new Error(result?.message || '')
          ElMessage.success(result.data?.state === 'pending_removal' ? T().pendingRestart : T().removeDone)
        } catch (error) {
          ElMessage.error(`${T().removeFailed} ${String(error?.message || '')}`.trim())
        }
        await refresh()
      }),
    },
  }
}
