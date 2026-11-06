<template>
  <DeskPage class="stream-view" :title="t.stream.pageTitle" wide>
    <div v-if="loading" class="dsk-stream__wait">
      <MLoading size="40px" :label="t.stream.loading" />
      <span>{{ t.stream.loading }}</span>
    </div>

    <template v-else>
      <div class="dsk-stream">
        <div class="dsk-stream__col">
          <!-- codecs -->
          <SettingsCard :title="t.stream.codec.format" icon="movie">
            <MItem
              v-for="codec in codecs"
              :key="codec.key"
              :stack="configData[codec.key] > 0"
              :title="codec.name"
              :text="codec.desc"
            >
              <template #trail>
                <MSwitch :model-value="configData[codec.key] > 0" :label="codec.name" @update:model-value="toggleCodec(codec.key)" />
              </template>
              <template v-if="configData[codec.key] > 0" #control>
                <MSeg v-model="configData[codec.key]" block :label="codec.name" :options="codecModes" />
              </template>
            </MItem>
          </SettingsCard>

          <!-- bitrate limit -->
          <SettingsCard :title="t.stream.bitrateLimit" icon="monitoring">
            <template #actions><MChip tone="info">{{ bitrateDisplay }}</MChip></template>
            <MItem stack :title="t.stream.bitrateLimit" :value="bitrateDisplay">
              <template #control>
                <div class="dsk-stream__range">
                  <MIconButton icon="remove" tonal :label="`${t.stream.bitrateLimit} −`" @click="nudgeBitrate(-5000)" />
                  <MSlider v-model="bitrateKbps" :min="1000" :max="200000" :step="1000" size="m" :label="t.stream.bitrateLimit" />
                  <MIconButton icon="add" tonal :label="`${t.stream.bitrateLimit} +`" @click="nudgeBitrate(5000)" />
                </div>
                <MSeg v-model="bitrateKbps" block :label="t.stream.bitrateLimit" :options="BITRATE_PRESETS" />
              </template>
            </MItem>
          </SettingsCard>

          <!-- HDR -->
          <SettingsCard :title="t.stream.hdr" icon="hdr_on">
            <SettingsRow :name="t.stream.hdrAutoSwitch" :description="t.stream.hdrAutoSwitchDesc">
              <SettingsSwitch
                :model-value="configData.hdr_prep === 'automatic'"
                :label="t.stream.hdrAutoSwitch"
                @update:model-value="configData.hdr_prep = $event ? 'automatic' : 'no_operation'"
              />
            </SettingsRow>
          </SettingsCard>

          <!-- launch mode -->
          <SettingsCard :title="t.stream.launchMode" icon="send">
            <SettingsRow class="dsk-stream__tap" :name="t.stream.autoLaunchDesktopUI" :description="t.stream.autoLaunchDesktopUIDesc" @click="onLaunchRow">
              <SettingsSwitch v-model="autoLaunchDesktop" :label="t.stream.autoLaunchDesktopUI" />
            </SettingsRow>
          </SettingsCard>
        </div>

        <div class="dsk-stream__col">
          <!-- display and capture -->
          <SettingsCard :title="t.stream.displayCapture" icon="monitor">
            <SettingsRow :name="t.stream.outputDisplay" :description="t.stream.outputDisplayDesc" />
            <MItem
              v-for="option in displayOptions"
              :key="option.value"
              action
              :selected="configData.output_name === option.value"
              :title="option.label"
              @click="configData.output_name = option.value"
            >
              <template #trail><MIcon v-if="configData.output_name === option.value" name="check" /></template>
            </MItem>

            <SettingsRow stack :name="t.stream.resolutionAdapt" :description="t.stream.resolutionAdaptDesc">
              <MSeg v-model="configData.resolution_change" block :label="t.stream.resolutionAdapt" :options="adaptModes" />
            </SettingsRow>
            <MItem v-if="configData.resolution_change === 2" stack :title="t.stream.manualResolution">
              <template #control>
                <MField v-model="configData.manual_resolution" :label="t.stream.manualResolution" placeholder="1920x1080" data-focusable data-focus-key="stream-manual-resolution" />
              </template>
            </MItem>

            <SettingsRow stack :name="t.stream.refreshRateAdapt" :description="t.stream.refreshRateAdaptDesc">
              <MSeg v-model="configData.refresh_rate_change" block :label="t.stream.refreshRateAdapt" :options="adaptModes" />
            </SettingsRow>
            <MItem v-if="configData.refresh_rate_change === 2" stack :title="t.stream.manualRefreshRate">
              <template #control>
                <MField v-model="configData.manual_refresh_rate" :label="t.stream.manualRefreshRate" placeholder="60" suffix="Hz" data-focusable data-focus-key="stream-manual-refresh" />
              </template>
            </MItem>
          </SettingsCard>

          <!-- virtual display -->
          <MBanner v-if="vddStatus.state !== 'ready'" :tone="vddReady ? 'warn' : 'error'" icon="warning">
            <strong>{{ t.vddSettings.driverPrerequisiteTitle }}</strong>
            {{ vddStatus.status_text || t.vddSettings.driverPrerequisiteDesc }}
            <template #actions>
              <MButton v-if="vddCanInstall" variant="text" :loading="vddInstalling" @click="installOrRepairVdd">{{ t.vddSettings.installRepairDriver }}</MButton>
              <MButton variant="text" :loading="vddChecking" @click="loadVddStatus">{{ t.vddSettings.recheckDriver }}</MButton>
            </template>
          </MBanner>
          <SettingsCard :title="t.stream.virtualDisplay" icon="desktop_windows">
            <template #actions><MChip :tone="vddTone">{{ vddStatusLabel }}</MChip></template>
            <SettingsRow v-if="nvidiaVsync.visible" :name="t.stream.nvidiaVsync.title" :disabled="nvidiaVsync.disabled || saving">
              <template #description>
                {{ t.stream.nvidiaVsync.description }}
                <span class="dsk-stream__status" role="status">{{ t.stream.nvidiaVsync[nvidiaVsync.state] }}</span>
              </template>
              <SettingsSwitch
                :model-value="nvidiaVsync.checked"
                :label="t.stream.nvidiaVsync.title"
                :disabled="nvidiaVsync.disabled || saving"
                @update:model-value="configData.nvenc_vdd_force_vsync = nvidiaVsync.checked ? 'disabled' : 'enabled'"
              />
            </SettingsRow>
            <SettingsRow stack :name="t.stream.vddPhysicalHandling" :description="t.stream.vddPhysicalHandlingDesc">
              <MSeg
                v-model="configData.vdd_prep"
                block
                :label="t.stream.vddPhysicalHandling"
                :options="[{ value: 0, label: t.stream.vddPhysicalMode.noAction }, { value: 1, label: t.stream.vddPhysicalMode.disable }]"
              />
            </SettingsRow>
            <SettingsRow :name="t.stream.vddKeepEnabled" :description="t.stream.vddKeepEnabledDesc">
              <SettingsSwitch
                :model-value="configData.vdd_keep_enabled === 'enabled'"
                :label="t.stream.vddKeepEnabled"
                :disabled="vddConfigSaving || (configData.vdd_keep_enabled !== 'enabled' && !vddReady)"
                @update:model-value="toggleVddOption('keep')"
              />
            </SettingsRow>
            <SettingsRow :name="t.stream.vddHeadlessCreate" :description="t.stream.vddHeadlessCreateDesc">
              <SettingsSwitch
                :model-value="configData.vdd_headless_create_enabled === 'enabled'"
                :label="t.stream.vddHeadlessCreate"
                :disabled="vddConfigSaving || (configData.vdd_headless_create_enabled !== 'enabled' && !vddReady)"
                @update:model-value="toggleVddOption('headless')"
              />
            </SettingsRow>
          </SettingsCard>

          <!-- virtual mouse driver -->
          <SettingsCard :title="t.stream.vmouse" icon="mouse">
            <template #actions><MChip :tone="vmouseTone">{{ vmouseStatusLabel }}</MChip></template>
            <SettingsRow :name="t.stream.vmouseToggle" :description="vmouseNotice || t.stream.vmouseToggleDesc" :disabled="vmouseConfigSaving || !vmouseStatusKnown">
              <SettingsSwitch
                :model-value="vmouseEnabled"
                :label="t.stream.vmouseToggle"
                :disabled="vmouseConfigSaving || !vmouseStatusKnown"
                @update:model-value="toggleVmouse"
              />
            </SettingsRow>
            <SettingsRow :name="t.stream.vmouseDriverStatus" :description="vmouseStatus.status_text || t.stream.vmouseDetecting">
              <MButton v-if="!vmouseStatus.installed" icon="download" :loading="vmouseInstalling" @click="installVmouse">
                {{ vmouseInstalling ? t.stream.vmouseInstalling : t.stream.vmouseInstall }}
              </MButton>
              <MButton v-else variant="danger" icon="delete" :loading="vmouseUninstalling" @click="uninstallVmouse">
                {{ vmouseUninstalling ? t.stream.vmouseUninstalling : t.stream.vmouseUninstall }}
              </MButton>
            </SettingsRow>
          </SettingsCard>
        </div>
      </div>

      <!-- save -->
      <div class="dsk-stream__foot">
        <SettingsNotice :notice="saveNotice" />
        <div class="m-savebar">
          <MButton size="lg" icon="check" :loading="saving" @click="saveSettings">{{ saving ? t.stream.saving : t.stream.saveSettings }}</MButton>
        </div>
      </div>
    </template>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { MBanner, MButton, MChip, MField, MIcon, MIconButton, MItem, MLoading, MSeg, MSlider, MSwitch, useNotify } from 'foundation-ui/vue'
import { requestConfirm } from '../composables/useConfirm.js'
import DeskPage from '../components/DeskPage.vue'
import SettingsCard from '../components/settings/SettingsCard.vue'
import SettingsNotice from '../components/settings/SettingsNotice.vue'
import SettingsRow from '../components/settings/SettingsRow.vue'
import SettingsSwitch from '../components/settings/SettingsSwitch.vue'
import { useVmouse, reloadVmouseAfterOperation } from '../../composables/useVmouse.js'
import { vdd as vddApi, vmouse as vmouseApi } from '../../tauri-adapter.js'
import { installVddWithRecovery } from '../../composables/vddInstallRecovery.js'
import { useVddStatusLabel } from '../../composables/useVddStatusLabel.js'
import { useI18n } from '../i18n/index.js'
import { nvidiaVsyncUi, observeNvidiaVsyncStatus } from '../../composables/nvidiaVsync.js'

const { t } = useI18n()
const say = (message) => useNotify().snack(String(message), { timeout: 8000 })
const ask = (message, confirmLabel = '') => requestConfirm({ message, confirmLabel })

const invoke = ref(null)
const proxyUrl = ref('http://localhost:48081')
const loading = ref(true)
const saving = ref(false)
const saveMsg = ref(null)
const displays = ref([])
const autoLaunchDesktop = ref(false)
const appsData = ref(null)  // 原始 apps.json 数据

const configData = ref({
  hevc_mode: 2,
  av1_mode: 0,
  max_bitrate: '50000',
  hdr_prep: 'automatic',
  output_name: '',
  resolution_change: 1,
  manual_resolution: '',
  refresh_rate_change: 1,
  manual_refresh_rate: '',
  vdd_prep: 0,
  nvenc_vdd_force_vsync: 'disabled',
  vdd_keep_enabled: 'disabled',
  vdd_headless_create_enabled: 'disabled',
})
const nvidiaCapability = ref(null)
const nvidiaManagement = ref('enabled')
const hostPlatform = ref('')
const nvidiaVsync = computed(() => nvidiaVsyncUi({
  nvenc_vdd_force_vsync: configData.value.nvenc_vdd_force_vsync,
  nvenc_manage_driver_settings: nvidiaManagement.value,
  nvidia_vsync: nvidiaCapability.value,
}, hostPlatform.value))
const trayManagedVddKeys = new Set(['vdd_keep_enabled', 'vdd_headless_create_enabled'])

const bitrateKbps = computed({
  get: () => parseInt(configData.value.max_bitrate) || 0,
  set: (v) => { configData.value.max_bitrate = String(v) },
})

const bitrateDisplay = computed(() => {
  const kbps = bitrateKbps.value
  if (kbps <= 0) return t.value.stream.bitrateUnlimited
  return kbps >= 1000 ? `${(kbps / 1000).toFixed(0)} Mbps` : `${kbps} Kbps`
})

const codecs = computed(() => [
  { key: 'hevc_mode', name: t.value.stream.codec.hevc, desc: t.value.stream.codec.hevcDesc },
  { key: 'av1_mode', name: t.value.stream.codec.av1, desc: t.value.stream.codec.av1Desc },
])

const codecModes = computed(() => [
  { value: 1, label: t.value.stream.codec.allow },
  { value: 2, label: t.value.stream.codec.always },
  { value: 3, label: t.value.stream.codec.alwaysHdr },
])

const adaptModes = computed(() => [
  { value: 0, label: t.value.stream.adaptMode.noChange },
  { value: 1, label: t.value.stream.adaptMode.auto },
  { value: 2, label: t.value.stream.adaptMode.manual },
])

const displayOptions = computed(() => {
  const opts = [{ value: '', label: t.value.stream.outputDisplayAuto }]
  for (const d of displays.value) {
    if (typeof d === 'string') {
      opts.push({ value: d, label: d })
    } else {
      opts.push(d)
    }
  }
  return opts
})

// A press anywhere on the launch-mode row but the switch itself flips it (the old card did).
function onLaunchRow(event) {
  if (event.target?.closest?.('.m-switch')) return
  autoLaunchDesktop.value = !autoLaunchDesktop.value
}

const STATUS_TONES = { good: 'ok', warn: 'warn', off: 'neutral' }
const vddTone = computed(() => STATUS_TONES[vddStatusClass.value] || 'neutral')
const vmouseTone = computed(() => STATUS_TONES[vmouseStatusClass.value] || 'neutral')
const saveNotice = computed(() => (saveMsg.value ? { type: saveMsg.value.type, message: saveMsg.value.text } : null))
const BITRATE_PRESETS = computed(() => [
  { value: 20000, label: '20 Mbps' },
  { value: 50000, label: '50 Mbps' },
  { value: 100000, label: '100 Mbps' },
  { value: 0, label: t.value.stream.bitrateUnlimited },
])
// the step buttons beside the slider (a pad cannot drag): 5 Mbps, inside the slider's range
const nudgeBitrate = (delta) => { bitrateKbps.value = Math.min(200000, Math.max(1000, (bitrateKbps.value || 200000) + delta)) }

function formatDisplayDeviceName(data = '') {
  const text = String(data || '')
  const displayMatch = text.match(/DISPLAY\d+/)
  const friendlyMatch = text.match(/FRIENDLY NAME:\s*([^\n\r]+)/)
  const friendly = friendlyMatch?.[1]?.trim()
  const display = displayMatch?.[0]
  if (friendly && display) return `${friendly} (${display})`
  return friendly || display || text.trim()
}

function normalizeDisplayDevices(devices) {
  if (!Array.isArray(devices)) return []
  return devices
    .map((device) => {
      if (typeof device === 'string') return { value: device, label: device }
      const value = device.device_id || device.id || device.value || ''
      const label = formatDisplayDeviceName(device.data || device.name || device.label || value)
      return value ? { value, label: label || value } : null
    })
    .filter(Boolean)
}

function toggleCodec(key) {
  configData.value[key] = configData.value[key] > 0 ? 0 : 2
}

async function initTauri() {
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
    const url = await invoke.value('get_proxy_url_command')
    if (url) proxyUrl.value = url
  } catch (e) {
    // not in Tauri
  }
}

async function apiFetch(path, options = {}) {
  const response = await fetch(`${proxyUrl.value}${path}`, options)
  return await response.json()
}

async function loadSettings() {
  loading.value = true
  try {
    const data = await apiFetch('/api/config')
    hostPlatform.value = data.platform || ''
    nvidiaCapability.value = data.nvidia_vsync || null
    nvidiaManagement.value = data.nvenc_manage_driver_settings ?? 'enabled'
    if (data.status?.toString() === 'true') {
      const keys = Object.keys(configData.value)
      for (const key of keys) {
        if (data[key] != null) {
          // 整数类型字段
          if (['hevc_mode', 'av1_mode', 'resolution_change', 'refresh_rate_change', 'vdd_prep'].includes(key)) {
            configData.value[key] = parseInt(data[key]) || 0
          } else if (trayManagedVddKeys.has(key)) {
            configData.value[key] = ['enabled', 'true', '1', 'yes'].includes(String(data[key]).toLowerCase())
              ? 'enabled'
              : 'disabled'
          } else {
            configData.value[key] = data[key]
          }
        }
      }
    }

    // 检测可用显示器
    const configDevices = normalizeDisplayDevices(data.display_devices)
    if (configDevices.length > 0) displays.value = configDevices

    if (invoke.value && displays.value.length === 0) {
      try {
        const monitors = await invoke.value('get_monitors')
        if (monitors && monitors.length > 0) displays.value = monitors
      } catch (e) { /* no monitor list command */ }
    }

    // 加载 apps 数据，检测 Desktop 应用是否配置了自动启动 Desktop UI
    try {
      const appsResp = await apiFetch('/api/apps')
      const appsList = appsResp.apps || appsResp || []
      appsData.value = { apps: appsList, env: appsResp.env || {} }
      const desktopApp = appsList.find(isDesktopApp)
      if (desktopApp) {
        const detached = desktopApp.detached || []
        autoLaunchDesktop.value = detached.some(cmd => 
          cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d'))
        )
      }
    } catch (e) {
      // apps load failed
    }
  } catch (e) {
    console.error('Failed to load settings:', e)
  } finally {
    loading.value = false
  }
}

async function saveSettings() {
  saving.value = true
  saveMsg.value = null
  try {
    // 先获取当前完整配置，避免覆盖其他字段
    const current = await apiFetch('/api/config')
    if (current.status?.toString() !== 'true') {
      saveMsg.value = { type: 'error', text: t.value.stream.msg.readFailed }
      return
    }

    // 基于完整配置合并我们修改的字段
    const payload = {}
    for (const [key, value] of Object.entries(current)) {
      if (key === 'status' || key === 'nvidia_vsync') continue
      payload[key] = String(value)
    }
    // 覆盖 StreamView 管理的字段
    for (const [key, value] of Object.entries(configData.value)) {
      if (trayManagedVddKeys.has(key)) continue
      payload[key] = String(value)
    }

    const result = await apiFetch('/api/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
    if (result.status?.toString() !== 'true') {
      saveMsg.value = { type: 'error', text: result.error || t.value.stream.msg.saveFailed }
      return
    }

    // 保存 Desktop 应用的启动模式
    await saveDesktopLaunchMode()

    if (!saveMsg.value) {
      saveMsg.value = { type: 'success', text: t.value.stream.msg.saveSuccess }
    }
  } catch (e) {
    saveMsg.value = { type: 'error', text: t.value.stream.msg.connectionError }
  } finally {
    saving.value = false
    setTimeout(() => { saveMsg.value = null }, 5000)
  }
}

const GUI_DESKTOP_CMD = '.\\assets\\gui\\sunshine-gui.exe --desktop'
const DESKTOP_APP_NAMES = ['Desktop', '桌面']
function isDesktopApp(app) { return DESKTOP_APP_NAMES.includes(app.name) }

async function saveDesktopLaunchMode() {
  if (!appsData.value) return
  const appsList = appsData.value.apps || []
  const desktopIdx = appsList.findIndex(isDesktopApp)
  if (desktopIdx === -1) return

  const desktopApp = { ...appsList[desktopIdx] }
  let detached = [...(desktopApp.detached || [])]

  // 移除旧的 Desktop UI 启动命令
  detached = detached.filter(cmd => 
    !(cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d')))
  )

  // 如果开启了自动启动，添加命令
  if (autoLaunchDesktop.value) {
    detached.push(GUI_DESKTOP_CMD)
  }

  desktopApp.detached = detached

  try {
    const editApp = { ...desktopApp, index: desktopIdx }
    const appsResult = await apiFetch('/api/apps', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ apps: appsList, editApp }),
    })
    if (appsResult.status?.toString() !== 'true') {
      saveMsg.value = { type: 'error', text: t.value.stream.msg.launchModeSaveFailed + ': ' + (appsResult.error || '') }
    }
  } catch (e) {
    saveMsg.value = { type: 'error', text: t.value.stream.msg.launchModeSaveFailed }
  }
}

let disposed = false
let stopNvidiaStatus = () => {}
onBeforeUnmount(() => {
  disposed = true
  stopNvidiaStatus()
})
onMounted(async () => {
  await initTauri()
  await Promise.all([loadSettings(), loadVddStatus(), loadVmouseStatus()])
  if (disposed) return
  stopNvidiaStatus = observeNvidiaVsyncStatus({
    load: options => apiFetch('/api/config', options),
    onStatus: data => {
      // Do not reload configData: it can contain unsaved edits.
      hostPlatform.value = data.platform || ''
      nvidiaCapability.value = data.nvidia_vsync || null
      nvidiaManagement.value = data.nvenc_manage_driver_settings ?? 'enabled'
    },
    onError: () => { nvidiaCapability.value = { ...nvidiaCapability.value, detection_known: false, state: 'unknown' } },
  })
})

const vddStatus = ref({
  state: 'unknown',
  installed: false,
  running: false,
  control_available: false,
  status_text: '',
})
const vddChecking = ref(false)
const vddInstalling = ref(false)
const vddConfigSaving = ref(false)

const vddReady = computed(() =>
  vddStatus.value.running &&
  vddStatus.value.control_available &&
  ['ready', 'degraded'].includes(vddStatus.value.state)
)
const vddCanInstall = computed(() => !['unsupported', 'payload_missing'].includes(vddStatus.value.state))
const vddStatusClass = computed(() => {
  if (vddStatus.value.state === 'ready') return 'good'
  if (vddReady.value) return 'warn'
  return 'off'
})
const vddStatusLabel = useVddStatusLabel(t, vddStatus)

async function loadVddStatus() {
  vddChecking.value = true
  try {
    const result = await vddApi.getStatus()
    if (!result?.success) throw new Error(result?.message || t.value.vddSettings.driverStatusCheckFailed)
    vddStatus.value = { ...vddStatus.value, ...result.data }
  } catch (error) {
    vddStatus.value = {
      ...vddStatus.value,
      state: 'unknown',
      running: false,
      status_text: error?.message || String(error),
    }
  } finally {
    vddChecking.value = false
  }
}

async function installOrRepairVdd() {
  if (!(await ask(t.value.vddSettings.installRepairConfirm, t.value.vddSettings.installRepairDriver))) return
  vddInstalling.value = true
  try {
    const install = () => installVddWithRecovery({
      install: () => vddApi.install(),
      getStatus: () => vddApi.getStatus(),
      onStatus: (status) => {
        vddStatus.value = { ...vddStatus.value, ...status }
      },
      verificationError: t.value.vddSettings.installVerificationFailed,
    })

    await install()

    say(t.value.vddSettings.installRepairSuccess)
  } catch (error) {
    say(`${t.value.vddSettings.installRepairFailed}: ${error?.message || error}`)
  } finally {
    vddInstalling.value = false
  }
}

async function toggleVddOption(option) {
  const key = option === 'keep' ? 'vdd_keep_enabled' : 'vdd_headless_create_enabled'
  const enabled = configData.value[key] !== 'enabled'
  vddConfigSaving.value = true
  try {
    const result = option === 'keep'
      ? await vddApi.setKeepEnabled(enabled)
      : await vddApi.setHeadlessCreateEnabled(enabled)
    if (!result?.success) throw new Error(result?.message || t.value.stream.msg.saveFailed)
    configData.value[key] = enabled ? 'enabled' : 'disabled'
  } catch (error) {
    say(error?.message || error)
  } finally {
    vddConfigSaving.value = false
  }
}

// ========== 虚拟鼠标驱动管理 ==========
const { vmouseStatus, vmouseEnabled, vmouseStatusKnown, vmouseNotice,
  vmouseConfigSaving, loadVmouseStatus, toggleVmouse } = useVmouse(t)
const vmouseInstalling = ref(false)
const vmouseUninstalling = ref(false)

const vmouseStatusClass = computed(() => {
  if (!vmouseStatusKnown.value) return 'warn'
  if (vmouseStatus.value.running) return 'good'
  if (vmouseStatus.value.installed) return 'warn'
  return 'off'
})

const vmouseStatusLabel = computed(() => {
  if (!vmouseStatusKnown.value) return t.value.stream.vmouseUnknown
  if (vmouseStatus.value.running) return t.value.stream.vmouseStatusRunning
  if (vmouseStatus.value.installed) return t.value.stream.vmouseStatusInstalled
  return t.value.stream.vmouseStatusNotInstalled
})

async function installVmouse() {
  if (!(await ask(t.value.stream.msg.vmouseInstallConfirm, t.value.stream.vmouseInstall))) return
  vmouseInstalling.value = true
  try {
    const result = await vmouseApi.install()
    if (result?.success) {
      say(result.data)
      await reloadVmouseAfterOperation()
    } else {
      say(t.value.stream.msg.vmouseInstallFailed + (result?.message || ''))
    }
  } catch (e) {
    say(t.value.stream.msg.vmouseInstallFailed + e)
  } finally {
    vmouseInstalling.value = false
  }
}

async function uninstallVmouse() {
  if (!(await ask(t.value.stream.msg.vmouseUninstallConfirm, t.value.stream.vmouseUninstall))) return
  vmouseUninstalling.value = true
  try {
    const result = await vmouseApi.uninstall()
    if (result?.success) {
      say(result.data)
      await reloadVmouseAfterOperation()
    } else {
      say(t.value.stream.msg.vmouseUninstallFailed + (result?.message || ''))
    }
  } catch (e) {
    say(t.value.stream.msg.vmouseUninstallFailed + e)
  } finally {
    vmouseUninstalling.value = false
  }
}
</script>

<style scoped>
.dsk-stream { display: grid; grid-template-columns: minmax(0, 1fr); gap: 24px; margin-bottom: 24px; }
.dsk-stream__col { display: flex; flex-direction: column; gap: 24px; min-width: 0; }
@media (min-width: 1500px) {
  .dsk-stream { grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; }
}
.dsk-stream__wait { display: flex; align-items: center; justify-content: center; gap: 16px; padding: 64px; font: var(--m-title-medium); color: var(--md-sys-color-on-surface-variant); }
.dsk-stream__range { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
.dsk-stream__range .m-slider { flex: 1; min-width: 0; }
.dsk-stream__status { display: block; margin-top: 2px; color: var(--md-sys-color-primary-text); }
.dsk-stream__tap { cursor: pointer; }
.dsk-stream__foot { position: sticky; bottom: 16px; z-index: 8; display: flex; flex-direction: column; align-items: center; gap: 8px; }
</style>
