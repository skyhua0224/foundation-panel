import { computed, reactive } from 'vue'
import { downloadView } from 'foundation-ui/update'

// The update transfer belongs to the Rust command, not to the sheet: closing
// the sheet (or its window) must not stop it, and reopening the window must
// show where it is. The module-scoped store survives the sheet's v-if and is
// seeded from `download_update_status` after a webview reload.
const idle = () => ({
  active: false,
  downloadPhase: '',
  progress: null,
  downloaded: 0,
  total: 0,
  source: '',
  // why the download is not from Mirror酱 although it is the chosen source ('' when it is or was not chosen)
  fallbackReason: '',
  filename: '',
  path: '',
  errorCode: '',
  errorDetail: '',
  opened: true,
})

const codeOf = (error) => (error && typeof error === 'object' ? String(error.code || '') : '')
const detailOf = (error) => (error && typeof error === 'object' ? String(error.detail || error.message || '') : String(error || ''))

export function createUpdateDownload({ invoke, listen }) {
  const state = reactive(idle())
  let unlisten = null
  let running = null

  const percent = computed(() => {
    if (state.progress == null) return null
    return Math.round(Math.min(1, Math.max(0, state.progress)) * 100)
  })
  const visible = computed(() => state.active || !!state.path)
  const ready = computed(() => !state.active && !!state.path)

  const apply = (payload) => {
    Object.assign(state, downloadView(payload))
    if (typeof payload?.fallbackReason === 'string') state.fallbackReason = payload.fallbackReason
  }

  /** Read the transfer that may already be running after a webview reload. */
  const refresh = async () => {
    if (running) return
    try {
      const status = await invoke('download_update_status')
      if (!status || typeof status !== 'object') return
      const view = downloadView(status)
      const active = status.active === true
      if (!active && !view.downloaded && !state.path) return
      Object.assign(state, view, {
        active,
        filename: status.filename || state.filename,
        path: status.path || state.path,
        errorCode: active ? '' : state.errorCode,
        errorDetail: active ? '' : (status.error || state.errorDetail),
        fallbackReason: status.fallback_reason || '',
      })
    } catch {
      // no backend (browser preview): the sheet still works for its own start
    }
  }

  const start = async ({ url, fallbackUrl, filename, expectedSize, expectedSha256, assetType, targetVersion }) => {
    if (running) return { busy: true, ok: false }
    Object.assign(state, idle(), { active: true, opened: false, filename })
    const task = (async () => {
      try {
        if (unlisten) unlisten()
        unlisten = await listen('download-progress', (event) => apply(event.payload))
        const result = await invoke('download_update', {
          url,
          fallbackUrl: fallbackUrl || null,
          filename,
          expectedSize: expectedSize || null,
          expectedSha256: expectedSha256 || null,
          assetType: assetType || null,
          targetVersion: targetVersion || null,
        })
        if (result?.success) {
          state.path = result.file_path || ''
          state.active = false
          state.downloadPhase = 'complete'
          state.progress = 1
          state.fallbackReason = result.fallback_reason || ''
          state.errorCode = ''
          state.errorDetail = ''
          return { ok: true, path: state.path, cached: !!result.cached }
        }
        state.active = false
        state.errorCode = 'download_failed'
        return { ok: false, errorCode: state.errorCode }
      } catch (error) {
        state.active = false
        state.errorCode = codeOf(error) || 'download_failed'
        state.errorDetail = detailOf(error)
        return { ok: false, errorCode: state.errorCode, error }
      } finally {
        if (unlisten) {
          try { await unlisten() } catch { /* the listener is gone with the view */ }
          unlisten = null
        }
        running = null
      }
    })()
    running = task
    return task
  }

  const discard = () => {
    if (running) return
    Object.assign(state, idle())
  }

  const markOpened = () => { state.opened = true }

  return { state, percent, visible, ready, start, refresh, discard, markOpened }
}

let singleton = null

export function useUpdateDownload() {
  if (!singleton) {
    singleton = createUpdateDownload({
      invoke: (...args) => import('@tauri-apps/api/core').then((module) => module.invoke(...args)),
      listen: (...args) => import('@tauri-apps/api/event').then((module) => module.listen(...args)),
    })
  }
  return singleton
}
