import { onActivated, onDeactivated, onMounted, onUnmounted, ref } from 'vue'
import { dualsenseStatusChannel, STATUS_POLL_INTERVAL_MS } from './deviceStatusStore.js'
import { setVisibleInterval, clearVisibleInterval } from './pageVisibility.js'

/**
 * Status of the native DualSense path (host + USB/IP transport). Read only: it
 * never installs or repairs anything. Polls while its pane is visible.
 */
export function useDualSenseStatus() {
  const status = ref(null)
  const loading = ref(false)
  const failed = ref(false)
  let timer
  let active = true
  let disposed = false

  async function refresh({ force = false } = {}) {
    loading.value = true
    const result = await dualsenseStatusChannel.fetch({ force })
    if (disposed) return
    failed.value = !result?.success
    if (result?.success) status.value = result.data
    loading.value = false
  }

  const start = () => {
    if (timer || disposed || !active) return
    // A poll reads the live state (a controller connecting), so it asks the host again (force); the cached answer
    // of the channel serves everything else. It rests while the page is hidden and reads once when it is shown.
    timer = setVisibleInterval(() => refresh({ force: true }), STATUS_POLL_INTERVAL_MS, { resume: true })
  }
  const stop = () => {
    if (timer) clearVisibleInterval(timer)
    timer = undefined
  }

  onMounted(async () => {
    await refresh()
    start()
  })
  // KeepAlive hides the pane without unmounting it: stop polling, recalibrate on return.
  onActivated(() => {
    active = true
    start()
    refresh()
  })
  onDeactivated(() => {
    active = false
    stop()
  })
  onUnmounted(() => {
    disposed = true
    stop()
  })

  return { status, loading, failed, refresh }
}
