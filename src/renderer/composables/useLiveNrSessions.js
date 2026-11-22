import { reactive, ref } from 'vue'
import { nrPipelines } from './nrOverlayState.js'

export function useLiveNrSessions(ready, invoke) {
  const nr = reactive({ list: [], selected: null, enabled: null, online: true, hasStreams: false })
  const nrError = ref('')
  let pollRevision = 0, selectionRevision = 0, savedTarget, targetRestored = false

  let pendingTarget = null, selectionWrite = Promise.resolve()
  function selectNr(id) {
    if (id === pendingTarget) return selectionWrite
    pendingTarget = id
    const revision = ++selectionRevision
    const current = () => revision === selectionRevision && ready.value && nr.online && nr.list.some(p => p.id === id)
    // Keep native writes in order. A newer poll may still contain the target.
    selectionWrite = selectionWrite.then(async () => {
      try {
        if (!current()) return
        await invoke('nr_overlay_select_session', { id })
        if (!current()) return
        nr.selected = id
        nrError.value = ''
      } catch (error) {
        if (current()) nrError.value = String(error)
      } finally {
        if (revision === selectionRevision) pendingTarget = null
      }
    })
    return selectionWrite
  }

  async function pollNr() {
    const revision = ++pollRevision
    const current = () => revision === pollRevision && ready.value
    if (!ready.value) return
    try {
      // A timer and a refresh after a save can overlap, including on first load.
      savedTarget ??= invoke('nr_overlay_settings').then(s => s?.target ?? null, () => null)
      const [status, target] = await Promise.all([invoke('nr_live_status'), savedTarget])
      if (!current()) return
      if (!targetRestored) {
        targetRestored = true
        if (!selectionRevision && target != null) nr.selected = target
      }
      nr.enabled = status.nr_enabled
      nr.hasStreams = status.has_streams === true
      nr.list = nrPipelines(status.pipelines)
      nr.online = true
      if (!nr.list.some(p => p.id === nr.selected)) nr.selected = null
      if (nr.selected === null && nr.list.length === 1) await selectNr(nr.list[0].id)
    } catch {
      if (!current()) return
      nr.online = false
      nr.list = []
    }
  }

  function dispose() {
    ++pollRevision
    ++selectionRevision
  }
  return { nr, nrError, selectNr, pollNr, dispose }
}
