// State of the controller type page: which pad games see, the per-game overrides, and the app list they come from.
import { computed, onMounted, ref, watch } from 'vue'
import { ElMessage } from './notifyMessage.js'
import { sunshine } from '../tauri-adapter.js'
import { useControllerHubConfig } from './useControllerHubConfig.js'
import { useI18n } from '../desktop/i18n/index.js'

export function useGamepadType() {

  const { t } = useI18n()

  const APPS_TIMEOUT_MS = 8000

  const { config, loading, unreachable, load, save } = useControllerHubConfig()

  const apps = ref([])
  const appsLoading = ref(false)
  const appsError = ref(false)
  const appSavingIndex = ref(-1)
  let proxyUrl = ''
  let appsRequestId = 0

  const busyKey = ref('')

  const modeOptions = computed(() => {
    const e = t.value.controllersHub.emulation
    return [
      { value: 'auto', label: e.familyAuto, hint: e.familyAutoHint },
      { value: 'x360', label: e.xbox360, hint: e.xbox360Hint },
      { value: 'ds4', label: 'DualShock 4', hint: e.ds4ModeHint },
      { value: 'ds5', label: e.ds5Variant, hint: e.ds5ModeHint },
    ]
  })

  const selectedMode = computed({
    get: () => config.value?.gamepad ?? 'auto',
    set: (mode) => saveKey('gamepad', mode),
  })

  const appModeOptions = computed(() => {
    const e = t.value.controllersHub.emulation
    return [
      { value: '', label: e.followGlobal },
      { value: 'auto', label: e.familyAuto },
      { value: 'x360', label: e.xbox360 },
      { value: 'ds4', label: 'DualShock 4' },
      { value: 'ds5', label: e.ds5Variant },
    ]
  })

  const hasDs5Selection = computed(() =>
    (config.value?.gamepad === 'ds5') || apps.value.some((app) => app.gamepad === 'ds5'))


  async function loadConfig() {
    await load()
  }

  async function saveKey(key, value) {
    if (busyKey.value !== '' || config.value?.[key] === value) return
    const prev = config.value?.[key]
    config.value = { ...(config.value || {}), [key]: value }
    busyKey.value = key
    try {
      const result = await save({ [key]: value })
      if (result?.success) {
        ElMessage.success(t.value.controllersHub.emulation.saveSuccess)
      } else {
        config.value = { ...(config.value || {}), [key]: prev }
        ElMessage.error(result?.message || t.value.controllersHub.emulation.saveFailed)
      }
    } catch (error) {
      config.value = { ...(config.value || {}), [key]: prev }
      ElMessage.error(String(error))
    } finally {
      busyKey.value = ''
    }
  }

  async function loadApps() {
    const requestId = ++appsRequestId
    appsLoading.value = true
    appsError.value = false
    let timeoutTimer = null
    try {
      const proxyResult = await sunshine.waitForProxyReady()
      if (!proxyResult?.success) throw new Error(proxyResult?.message || 'proxy not ready')
      if (requestId !== appsRequestId) return
      proxyUrl = proxyResult.data

      const controller = new AbortController()
      timeoutTimer = setTimeout(() => controller.abort('timeout'), APPS_TIMEOUT_MS)
      const resp = await fetch(`${proxyUrl}/api/apps`, { signal: controller.signal })
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)

      const data = await resp.json()
      if (requestId !== appsRequestId) return
      apps.value = Array.isArray(data) ? data : data?.apps || []
    } catch (error) {
      if (requestId !== appsRequestId) return
      console.error('加载应用列表失败:', error)
      appsError.value = true
    } finally {
      if (timeoutTimer !== null) clearTimeout(timeoutTimer)
      if (requestId === appsRequestId) appsLoading.value = false
    }
  }

  async function saveAppGamepad(index, value) {
    const app = apps.value[index]
    if (!app || appSavingIndex.value !== -1) return
    const prev = app.gamepad || ''
    app.gamepad = value
    appSavingIndex.value = index
    let timeoutTimer = null
    try {
      const controller = new AbortController()
      timeoutTimer = setTimeout(() => controller.abort('timeout'), APPS_TIMEOUT_MS)
      const resp = await fetch(`${proxyUrl}/api/apps`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ apps: apps.value, editApp: { index, ...app } }),
        signal: controller.signal,
      })
      if (resp.status === 409) {
        app.gamepad = prev
        ElMessage.error(t.value.controllersHub.emulation.appSaveConflict)
        await loadApps()
        return
      }
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
      ElMessage.success(t.value.controllersHub.emulation.appSaved)
    } catch {
      app.gamepad = prev
      ElMessage.error(t.value.controllersHub.emulation.appSaveFailed)
    } finally {
      if (timeoutTimer !== null) clearTimeout(timeoutTimer)
      appSavingIndex.value = -1
    }
  }


  onMounted(() => {
    loadConfig()
    loadApps()
  })
  return {
    config, loading, unreachable, apps, appsLoading, appsError, appSavingIndex, busyKey, modeOptions, selectedMode,
    appModeOptions, hasDs5Selection, loadConfig, loadApps, saveAppGamepad,
  }
}
