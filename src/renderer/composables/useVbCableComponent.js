import { computed, ref } from 'vue'

export function useVbCableComponent({ api, text, shared, confirm, invalidate }) {
  const status = ref(null)
  const archiveReady = ref(false)
  const phase = ref('')
  const notice = ref('')
  const busy = computed(() => !!phase.value || shared.busyElsewhere.value)
  const canInstall = computed(() => !busy.value && status.value?.state === 'missing' && archiveReady.value)
  const canEnable = computed(() => !busy.value && status.value?.state === 'disabled')
  const hasRemovableDriver = computed(() => ['installed', 'disabled', 'unavailable', 'pending'].includes(status.value?.state))
  const canUninstall = computed(() => !busy.value && hasRemovableDriver.value)

  function applyStatus(next) {
    status.value = next
    archiveReady.value = next.archive_ready === true
  }

  async function check() {
    if (busy.value) return
    phase.value = 'check'
    try {
      const result = await api.check()
      if (!result?.success) throw new Error(result?.message || '')
      applyStatus(result.data)
      notice.value = ''
    } catch (error) {
      status.value = null
      notice.value = `${text.value.checkFailed} ${error.message || ''}`
    } finally {
      phase.value = ''
    }
  }

  const download = shared.wrap(async () => {
    if (phase.value || archiveReady.value) return
    phase.value = 'download'
    notice.value = ''
    try {
      const result = await api.download()
      if (!result?.success) throw new Error(result?.message || '')
      archiveReady.value = true
      notice.value = text.value.downloaded
    } catch (error) {
      notice.value = `${text.value.downloadFailed} ${error.message || ''}`
    } finally {
      phase.value = ''
    }
  })

  const changeDriver = shared.wrap(async (action) => {
    if (phase.value || !['install', 'enable', 'uninstall'].includes(action)) return
    const installing = action === 'install'
    if (installing ? !canInstall.value : action === 'enable' ? !canEnable.value : !canUninstall.value) return
    phase.value = action
    notice.value = ''
    try {
      await confirm(action)
    } catch {
      phase.value = ''
      return
    }
    try {
      shared.markChanged()
      invalidate()
      const result = await api[action]()
      if (!result?.success) throw new Error(result?.message || '')
      if (result.data.status) applyStatus(result.data.status)
      else status.value = null
      const successKey = { install: 'installed', enable: 'enabled', uninstall: 'uninstalled' }[action]
      const incompleteRemoval = action === 'uninstall' && result.data.status?.state !== 'missing'
      notice.value = result.data.restart_suggested === true
        ? `${incompleteRemoval ? `${text.value.states[result.data.status?.state] || text.value.notChecked} ` : ''}${text.value.restartSuggested}`
        : text.value[successKey]
      if (result.data.check_error) notice.value += ` ${text.value.checkFailed} ${result.data.check_error}`
    } catch (error) {
      const reason = String(error.message || '')
      if (installing && reason.startsWith('VBCABLE-ARCH-UNSUPPORTED:')) notice.value = text.value.unsupportedArchitecture
      else if (reason.includes('VBCABLE-OP-TIMEOUT:')) notice.value = text.value.operationTimeout
      else if (reason.includes('VBCABLE-OP-BUSY:')) notice.value = text.value.operationBusy
      else if (reason.includes('VBCABLE-DEVICE-BUSY:')) notice.value = text.value.deviceBusy
      else notice.value = `${text.value[`${action}Failed`]} ${reason}`
      try {
        const checked = await api.check()
        if (checked?.success) applyStatus(checked.data)
        else status.value = null
      } catch { status.value = null }
    } finally {
      invalidate()
      phase.value = ''
    }
  })

  return { status, archiveReady, phase, notice, busy, canInstall, canEnable, hasRemovableDriver, canUninstall, check, download, changeDriver }
}
