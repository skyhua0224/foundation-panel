<template>
  <MPage :title="pageTitle" :back="activeSection !== 'overview'" :back-label="t.m3e.back" @back="backVdd">
    <template #actions>
      <MIconButton v-if="!isDriverCheckPending && vddReady" icon="refresh" :label="t.vddSettings.reload" :disabled="isLoading" @click="reloadSettings" />
    </template>

    <MBanner v-if="!isDriverCheckPending && vddReady && vddStatus.state === 'degraded'" tone="warn" icon="warning">
      <b>{{ vddStatusLabel }}</b><br>{{ vddStatus.status_text }}
      <template #actions><MButton variant="text" size="sm" icon="download" :loading="isInstallingDriver" @click="installOrRepairDriver">{{ t.vddSettings.installRepairDriver }}</MButton></template>
    </MBanner>

    <!-- opening the page: the one loading state, nothing else (no wave, no explaining rows) -->
    <MPageLoading v-if="isDriverCheckPending && openingCheck" :rows="5" />

    <!-- Driver check (asked for: 重新检测, installing) -->
    <template v-else-if="isDriverCheckPending">
      <MStatusPill icon="sync" :label="driverDetectionTitle" :action="false" />
      <MProgress :label="driverDetectionTitle" />
      <MGroup>
        <MCheckItem :title="t.vddSettings.driverDetectionPresence" :state="driverCheckPhase === 'syncing' ? 'ok' : 'pending'" />
        <MCheckItem :title="t.vddSettings.driverDetectionHealth" :state="driverCheckPhase === 'syncing' ? 'ok' : 'pending'" />
        <MCheckItem :title="t.vddSettings.driverDetectionVersion" :state="driverCheckPhase === 'syncing' ? 'ok' : 'pending'" />
      </MGroup>
      <MFooter>{{ driverDetectionDescription }} {{ t.vddSettings.driverDetectionLocked }}</MFooter>
    </template>

    <!-- Driver missing or unhealthy -->
    <template v-else-if="!vddReady">
      <MStatusPill icon="desktop_windows" tone="warn" :label="vddStatusLabel || t.vddSettings.driverStatusUnknown" :action="false" />
      <MGroup>
        <MItem icon="info" :title="t.vddSettings.driverPrerequisiteTitle" :text="vddStatus.status_text || t.vddSettings.driverStatusUnknown" />
        <MItem v-if="vddStatus.installed_version || vddStatus.bundled_version" icon="memory" :title="t.vddSettings.driverVersionInstalled" :value="vddStatus.installed_version || '—'">
          <template #trail><span class="m-item__count">{{ t.vddSettings.driverVersionBundled }} {{ vddStatus.bundled_version || '—' }}</span></template>
        </MItem>
      </MGroup>
      <MFooter>{{ t.vddSettings.driverPrerequisiteDesc }}</MFooter>
      <div class="m-actions-row">
        <MButton v-if="vddCanInstall" icon="download" :loading="isInstallingDriver" @click="installOrRepairDriver">{{ t.vddSettings.installRepairDriver }}</MButton>
        <MButton variant="tonal" icon="refresh" :loading="isCheckingDriver" @click="recheckDriver">{{ t.vddSettings.recheckDriver }}</MButton>
      </div>
      <MProgress v-if="isInstallingDriver" class="vdd-installing" :label="t.vddSettings.installingDriver" />
    </template>

    <template v-else>
      <!-- Overview: the device card, then the way to the settings that belong to the display; nothing here is a second copy of a setting (modes, calibration and scaling live on their own pages) -->
      <template v-if="activeSection === 'overview'">
        <MBanner v-if="hostVdd.store.error.value" tone="error" icon="error">{{ vp.saveFailed.replace('{error}', hostVdd.store.error.value) }}</MBanner>
        <MSection :title="dt.sections.virtual.device">
          <MVddDevice
            v-model:keep="keepModel" data-cfg="vdd_keep_enabled" :state="deviceState" :installed-version="vddStatus.installed_version || ''" :bundled-version="vddStatus.bundled_version || ''"
            :active="hostVdd.store.summary.value.active" :busy="isInstallingDriver || isCreatingNow" :can-install="vddCanInstall" :can-keep="hostVdd.store.canSaveProtected" :keep-busy="!!hostVdd.store.busy.vdd_keep_enabled"
            :status-text="vddStatus.status_text || ''" :locale="locale" @install="installOrRepairDriver" @recheck="recheckDriver" @create="trayVdd('vdd_create')" @close="trayVdd('vdd_destroy')"
          />
          <MGroup>
            <MSwitchItem v-model="headlessModel" icon="desktop_access_disabled" :title="dt.device.headless" :text="hostVdd.store.canSaveProtected ? '' : vt.trayOnly" :disabled="!hostVdd.store.canSaveProtected || hostVdd.store.busy.vdd_headless_create" />
            <MSwitchItem v-model="reuseModel" data-cfg="vdd_reuse" icon="devices" :title="dt.device.reuse" :text="dt.device.reuseText" :disabled="hostVdd.store.busy.vdd_reuse" />
          </MGroup>
        </MSection>
        <MSection :title="dt.sections.virtual.links">
          <MGroup>
            <MItem action icon="aspect_ratio" :title="dt.device.links.modes" @click="emit('webui-navigate', '/config#tab=display&sub=modes')" />
            <MItem action icon="hdr_on" :title="dt.device.links.calibration" @click="emit('webui-navigate', '/config#tab=display&sub=hdr')" />
            <MItem action icon="format_size" :title="dt.device.links.scaling" @click="emit('webui-navigate', '/config#tab=display&sub=output')" />
          </MGroup>
        </MSection>
        <MExpander v-model:open="driverOpen" :title="vp.driverTitle" icon="build">
          <MGroup>
            <MItem action icon="monitor" :title="t.vddSettings.tabDisplay" :text="displaySummary" @click="activeSection = 'display'" />
            <MItem action icon="palette" :title="vp.qualityTitle" :text="qualitySummary" @click="activeSection = 'quality'" />
            <MItem action icon="build" :title="vp.driverTitle" :text="vp.tier3" @click="activeSection = 'driver'" />
          </MGroup>
        </MExpander>
      </template>

      <!-- Display: resolutions, refresh rates, GPU, colour -->
      <template v-else-if="activeSection === 'display'">
        <MSection :title="t.vddSettings.displaySection">
          <MGroup>
            <MValueItem v-if="!gpuOptions.length" :model-value="gpuFriendlyName" :label="t.vddSettings.gpuBinding" :placeholder="t.vddSettings.gpuPlaceholder" :ok-label="t.common.confirm" :cancel-label="t.common.cancel" @update:model-value="(v) => { gpuFriendlyName = v; saveGpuEdit() }" />
            <MChoiceItem v-if="gpuOptions.length" :model-value="gpuFriendlyName" icon="developer_board" :title="t.vddSettings.gpuBinding" :options="gpuOptions.map((g) => ({ value: g, label: g }))" :cancel-label="t.common.cancel" @update:model-value="(v) => { gpuFriendlyName = v; saveGpuEdit() }" />
            <MItem icon="monitor" :title="t.vddSettings.monitorCount" :text="t.vddSettings.monitorCountTip" :value="String(settings.monitors.count)" />
            <MSwitchItem v-model="settings.edid.Vrr" icon="sync" :title="t.vddSettings.vrr" :text="t.vddSettings.vrrTip" />
          </MGroup>
        </MSection>
      </template>

      <!-- Quality: colour depth, colour mode, cursor -->
      <template v-else-if="activeSection === 'quality'">
        <MSection :title="vp.qualityTitle">
          <MGroup>
            <MItem icon="hdr_on" :title="t.vddSettings.colorDepthProfile" :text="t.vddSettings.colorDepthMutualExclusiveTip" stack>
              <template #control>
                <MSeg v-model="colorDepthProfile" block :label="t.vddSettings.colorDepthProfile" :options="colorDepthOptions" />
              </template>
            </MItem>
            <MChoiceItem v-model="settings.colour.ColourFormat" icon="palette" :title="t.vddSettings.colorMode" :options="['RGB', 'YCbCr444', 'YCbCr422', 'YCbCr420'].map((v) => ({ value: v, label: v }))" :cancel-label="t.common.cancel" />
            <MSwitchItem v-model="settings.cursor.HardwareCursor" icon="mouse" :title="t.vddSettings.hardwareCursor" :text="t.vddSettings.hardwareCursorTip" disabled />
          </MGroup>
          <MBanner v-if="isColorDepthConflict" tone="warn" icon="warning">{{ t.vddSettings.colorDepthConflictTip }}</MBanner>
        </MSection>
      </template>

      <!-- Driver: tools, trace, EDID -->
      <template v-else-if="activeSection === 'driver'">
        <MSection :title="t.vddSettings.driverTools">
          <MGroup>
            <MItem action icon="refresh" :title="t.vddSettings.reloadDriver" :disabled="isReloadingDriver" @click="confirmThen('reload', t.vddSettings.reloadDriver, reloadDriver)" />
            <MItem v-if="edidFileExists" action icon="delete" :title="t.vddSettings.deleteEdid" :text="t.vddSettings.dangerZoneHint" :disabled="isDeletingEdid" @click="removeEdidFile" />
          </MGroup>
          <MFooter>{{ t.vddSettings.driverToolsHint }}</MFooter>
        </MSection>
        <MSection :title="t.vddSettings.vddTraceTitle">
          <MGroup>
            <MItem icon="monitor_heart" :title="traceStatus.running ? t.vddSettings.vddTraceRunning : t.vddSettings.vddTraceIdle" :text="traceDisplayPath" />
            <MItem action icon="play_arrow" :title="t.vddSettings.vddTraceStart" :disabled="traceStatus.running || isStartingTrace || isStoppingTrace" @click="startTrace" />
            <MItem action icon="stop" :title="t.vddSettings.vddTraceStop" :disabled="!traceStatus.running || isStartingTrace || isStoppingTrace" @click="stopTrace" />
            <MItem action icon="folder_open" :title="t.vddSettings.vddTraceOpenFolder" :disabled="!traceStatus.directory || isOpeningTraceFolder" @click="openTraceFolder" />
          </MGroup>
          <MFooter>{{ t.vddSettings.vddTraceHint }} {{ t.vddSettings.vddTraceAdminHint }}</MFooter>
        </MSection>
        <MSection :title="t.vddSettings.edidSection">
          <MGroup>
            <MChoiceItem :model-value="settings.edid.EdidProfile" icon="tune" :title="vp.edidProfile" :text="settings.edid.CustomEdid ? vp.edidProfileIgnored : `${vp.edidProfiles[settings.edid.EdidProfile]?.text || ''}. ${vp.edidProfileText}`" :options="edidProfileOptions" :disabled="settings.edid.CustomEdid" :cancel-label="t.common.cancel" data-cfg="EdidProfile" @update:model-value="(v) => confirmThen('edidProfile', vp.edidProfile, () => (settings.edid.EdidProfile = v))" />
            <MSwitchItem :model-value="settings.edid.DolbyVision" icon="hdr_on" :title="t.vddSettings.dolbyVision" :text="settings.edid.CustomEdid && !settings.edid.EdidCeaOverride ? t.vddSettings.dolbyVisionCustomEdidTip : t.vddSettings.dolbyVisionTip" :disabled="settings.edid.CustomEdid && !settings.edid.EdidCeaOverride" @update:model-value="(v) => confirmThen('dolby', t.vddSettings.dolbyVision, () => (settings.edid.DolbyVision = v))" />
            <MSwitchItem :model-value="settings.edid.CustomEdid" icon="description" :title="t.vddSettings.customEdid" :text="t.vddSettings.customEdidTip" @update:model-value="(v) => confirmThen('customEdid', t.vddSettings.customEdid, () => { settings.edid.CustomEdid = v; handleEdidToggle(v) })" />
            <template v-if="settings.edid.CustomEdid">
              <MSwitchItem v-model="settings.edid.PreventSpoof" icon="verified_user" :title="vp.preventSpoof" :text="vp.preventSpoofText" data-cfg="PreventSpoof" />
              <MSwitchItem v-model="settings.edid.EdidCeaOverride" icon="hdr_on" :title="vp.ceaOverride" :text="vp.ceaOverrideText" data-cfg="EdidCeaOverride" />
              <MItem icon="description" :title="edidFileExists ? t.vddSettings.edidUploaded : t.vddSettings.edidNotUploaded" :text="edidFilePath" />
              <MItem action icon="upload" :title="t.vddSettings.edidSelectFile" @click="edidInput?.click()" />
              <MItem action icon="download" :title="t.vddSettings.edidDownload" :disabled="!edidFileExists" @click="downloadEdid" />
              <template v-if="edidInfo">
                <MItem icon="storage" :title="t.vddSettings.edidFileSize" :value="`${edidInfo.size} ${t.vddSettings.edidBytes}`" />
                <MItem icon="info" :title="t.vddSettings.edidFormat" :value="getEdidFormatLabel(edidInfo.size)" />
                <MItem :icon="edidInfo.checksumValid ? 'check_circle' : 'error'" title="Checksum" :value="edidInfo.checksumValid ? t.vddSettings.edidChecksumValid : t.vddSettings.edidChecksumInvalid" />
              </template>
            </template>
          </MGroup>
          <MBanner v-if="hasCustomEdidIssue" tone="warn" icon="warning">{{ t.vddSettings.customEdidMissingInline }}</MBanner>
          <MBanner v-if="settings.edid.CustomEdid" tone="warn" icon="warning">{{ t.vddSettings.edidWarning }}</MBanner>
          <MFooter>{{ t.vddSettings.edidSectionHint }}</MFooter>
          <input ref="edidInput" type="file" accept=".bin" hidden @change="onEdidPicked">
        </MSection>
        <MSection :title="vp.cursorTitle">
          <MGroup>
            <MChoiceItem :model-value="settings.cursor.CursorMaxX" icon="mouse" :title="vp.cursorMax" :text="vp.cursorMaxText" :options="cursorMaxOptions" :cancel-label="t.common.cancel" data-cfg="CursorMax" @update:model-value="(v) => confirmThen('cursor', vp.cursorMax, () => { settings.cursor.CursorMaxX = v; settings.cursor.CursorMaxY = v })" />
            <MSwitchItem :model-value="settings.cursor.AlphaCursorSupport" icon="blur_on" :title="vp.alphaCursor" :text="vp.alphaCursorText" data-cfg="AlphaCursorSupport" @update:model-value="(v) => confirmThen('cursor', vp.alphaCursor, () => (settings.cursor.AlphaCursorSupport = v))" />
            <MChoiceItem :model-value="settings.cursor.XorCursorSupportLevel" icon="tune" :title="vp.xorCursor" :text="vp.xorCursorText" :options="xorLevelOptions" :cancel-label="t.common.cancel" data-cfg="XorCursorSupportLevel" @update:model-value="(v) => confirmThen('cursor', vp.xorCursor, () => (settings.cursor.XorCursorSupportLevel = v))" />
          </MGroup>
          <MFooter>{{ vp.cursorHint }}</MFooter>
        </MSection>
      </template>

      <MDialog v-model="confirm.open" :title="vp.confirmTitle.replace('{name}', confirm.name)" icon="warning" tone="warning">
        <p class="vdd-confirm__text">{{ confirm.text }}</p>
        <template #actions>
          <MButton variant="text" @click="confirm.open = false">{{ vp.cancel }}</MButton>
          <MButton @click="confirmApply">{{ vp.confirmChange }}</MButton>
        </template>
      </MDialog>
    </template>
  </MPage>
</template>

<script setup>
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from '../composables/notifyMessage.js'
import { MExpander, MVddDevice, displayTreeTexts, MPage, MGroup, MSection, MItem, MSwitchItem, MChoiceItem, MDialog, MValueItem, MSeg, MBanner, MFooter, MButton, MIconButton, MStatusPill, MProgress, MCheckItem, MPageLoading, useSubpage, vddPageTexts, vddTexts, vddOn } from 'foundation-ui/vue'
import { useHostVdd } from '../composables/useHostVdd.js'
import { cleanList } from 'foundation-ui/settings/modeList.js'
import { useVddEdid } from '../composables/useVddEdid.js'
import { useVddStatusLabel } from '../composables/useVddStatusLabel.js'
import { installVddWithRecovery } from '../composables/vddInstallRecovery.js'
import { sunshine, vdd } from '../tauri-adapter.js'
import { setPath, useVddPending } from '../composables/useVddPending.js'
import { useI18n } from '../desktop/i18n/index.js'

const { t, locale } = useI18n()
const vp = computed(() => vddPageTexts(locale.value))
const vt = computed(() => vddTexts(locale.value))

const DEFAULT_REFRESH_RATES = ['60', '120', '240']
const CHINESE_PATTERN = /[\u4e00-\u9fa5]/
const COLOR_DEPTH_DEFAULT = 'default'
const COLOR_DEPTH_SDR10 = 'sdr10'
const COLOR_DEPTH_HDR12 = 'hdr12'
const COLOR_DEPTH_CONFLICT = 'conflict'

const createInitialSettings = () => ({
  monitors: { count: 1 },
  gpu: { friendlyname: '' },
  global: {
    g_refresh_rate: [...DEFAULT_REFRESH_RATES],
  },
  resolutions: { resolution: [] },
  colour: {
    SDR10bit: false,
    HDRPlus: false,
    ColourFormat: 'RGB',
  },
  cursor: {
    HardwareCursor: true,
    CursorMaxY: 128,
    CursorMaxX: 128,
    AlphaCursorSupport: true,
    XorCursorSupportLevel: 2,
  },
  edid: {
    CustomEdid: false,
    DolbyVision: false,
    PreventSpoof: false,
    EdidCeaOverride: false,
    EdidProfile: 'auto',
    Vrr: false,
  },
})

const getErrorMessage = (error, fallback = '') => {
  // the browser's own network failures ("Failed to fetch") are not wording for a person: the page's own sentence says what happened
  if (error instanceof Error && error.message && !(error instanceof TypeError && /fetch|network|load failed/i.test(error.message))) {
    return error.message
  }

  if (typeof error === 'string' && error.trim()) {
    return error
  }

  return fallback
}

const fillPlaceholders = (message, replacements = {}) => Object.entries(replacements).reduce(
  (result, [key, value]) => result.replace(`{${key}}`, String(value)),
  message || ''
)


const syncGpuOptions = (options = []) => {
  const uniqueOptions = [...new Set(options.filter(Boolean))]

  if (gpuFriendlyName.value && !uniqueOptions.includes(gpuFriendlyName.value)) {
    uniqueOptions.unshift(gpuFriendlyName.value)
  }

  gpuOptions.value = uniqueOptions
}

const settings = reactive(createInitialSettings())
const gpuFriendlyName = ref('')
const gpuOptions = ref([])
const traceStatus = ref({
  running: false,
  directory: '',
  latest_file: '',
})
const vddStatus = reactive({
  state: 'unknown',
  installed: false,
  running: false,
  control_available: false,
  installed_version: '',
  bundled_version: '',
  version_match: false,
  monitor_active: false,
  status_text: '',
})

const isLoading = ref(false)
const isCheckingDriver = ref(false)
const hasCompletedDriverCheck = ref(false)
// the first check, made when the page opens
const openingCheck = ref(true)
const driverCheckPhase = ref('detecting')
const isInstallingDriver = ref(false)
const isSaving = ref(false)
const isReloadingDriver = ref(false)
const isDeletingEdid = ref(false)
const isStartingTrace = ref(false)
const isStoppingTrace = ref(false)
const isOpeningTraceFolder = ref(false)
const isRefreshingCalibration = ref(false)
const { current: vddSub, open: openVddSub, back: backVdd } = useSubpage('vdd')

// ---- the host's keys of the virtual display (the modes, the shared display, the rules, the expert options): the shared store, each change saved at once ----
const hostVdd = useHostVdd()
const emit = defineEmits(['close', 'webui-navigate'])
const dt = computed(() => displayTreeTexts(locale.value))
const driverOpen = ref(false)
const keepModel = computed({ get: () => hostVdd.store.mode.value === 'persistent', set: (on) => { void hostVdd.store.set({ vdd_keep_enabled: on }) } })
// the state of the device card: the driver as the host reports it ('degraded' still works but asks for a repair)
const deviceState = computed(() => (isDriverCheckPending.value ? 'checking' : vddStatus.state === 'degraded' ? 'degraded' : vddReady.value ? 'ready' : ['unknown', ''].includes(vddStatus.state) ? 'unknown' : 'missing'))
// create or close the display now: the same tray actions the tray menu runs, through the host API
const isCreatingNow = ref(false)
async function trayVdd(action) {
  isCreatingNow.value = true
  try { await hostVdd.fetchJson('/api/tray/action', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ action }) }); await hostVdd.store.refresh() } catch (error) { ElMessage.error(getErrorMessage(error, '')) } finally { isCreatingNow.value = false }
}
const hostConfig = hostVdd.store.config
const flagOf = (key, def = false) => computed({
  get: () => (hostConfig.value[key] === undefined || hostConfig.value[key] === '' ? def : vddOn(hostConfig.value[key])),
  set: (on) => { void hostVdd.store.set({ [key]: typeof hostConfig.value[key] === 'boolean' ? on : on ? 'enabled' : 'disabled' }) },
})
const reuseModel = flagOf('vdd_reuse')
const headlessModel = flagOf('vdd_headless_create')

// the Vulkan HDR bridge: what the host reports and a test of the current display
const vulkanStatus = ref({ state: 'idle', artifacts_installed: false, display_available: false })
const vulkanLoaded = ref(false)
const vulkanValidating = ref(false)
const vulkan = {
  validating: vulkanValidating,
  async refresh() {
    try { const r = await hostVdd.fetchJson('/api/vulkan-hdr-bridge'); if (String(r?.status) === 'true') vulkanStatus.value = r; else throw new Error('no status') } catch { vulkanStatus.value = { state: 'unavailable', artifacts_installed: false, display_available: false } } finally { vulkanLoaded.value = true }
  },
  async validate() {
    vulkanValidating.value = true; vulkanStatus.value = { ...vulkanStatus.value, state: 'validating' }
    try { vulkanStatus.value = await hostVdd.fetchJson('/api/vulkan-hdr-bridge/validate', { method: 'POST' }) } catch { vulkanStatus.value = { ...vulkanStatus.value, state: 'error' } } finally { vulkanValidating.value = false; vulkanLoaded.value = true }
  },
}

// an expert option is confirmed before it changes: what it does, then the change
const confirm = reactive({ open: false, name: '', text: '', apply: null })
const confirmThen = (riskKey, name, apply) => { Object.assign(confirm, { open: true, name, text: vp.value.risk[riskKey] || '', apply }) }
const confirmApply = () => { const run = confirm.apply; confirm.open = false; confirm.apply = null; if (run) run() }
const displaySummary = computed(() => [gpuFriendlyName.value, `${settings.monitors.count}`].filter(Boolean).join(' · '))
const edidProfileOptions = computed(() => ['auto', 'legacy', 'modern'].map((value) => ({ value, label: vp.value.edidProfiles[value].title })))
const cursorMaxOptions = computed(() => [...new Set([64, 128, 256, Number(settings.cursor.CursorMaxX) || 128])].sort((x, y) => x - y).map((value) => ({ value, label: `${value} × ${value}` })))
const xorLevelOptions = computed(() => [0, 1, 2, 3].map((value) => ({ value, label: vp.value.xorLevels[value] })))
const qualitySummary = computed(() => [colorDepthOptions.value.find((o) => o.value === colorDepthProfile.value)?.label, settings.colour.ColourFormat].filter(Boolean).join(' · '))
onMounted(() => { void hostVdd.load(); void vulkan.refresh() })
const activeSection = computed({
  get: () => vddSub.value || 'overview',
  set: (value) => { if (value === 'overview') { if (vddSub.value) backVdd() } else openVddSub(value) },
})
// the toolbar menu's 调整 DPI: the overview, scrolled to 桌面缩放
const pageTitle = computed(() => ({ display: t.value.vddSettings.tabDisplay, quality: vp.value.qualityTitle, calibration: t.value.vddSettings.calibrationTitle, driver: vp.value.driverTitle }[activeSection.value] || t.value.m3e.display.vdd))
const edidInput = ref(null)
const onEdidPicked = (event) => {
  const file = event.target.files?.[0]
  event.target.value = ''
  if (file) handleEdidFileChange({ raw: file })
}
const colorDepthOptions = computed(() => [
  { value: COLOR_DEPTH_DEFAULT, label: t.value.vddSettings.colorDepthDefault },
  { value: COLOR_DEPTH_SDR10, label: t.value.vddSettings.sdr10bit },
  { value: COLOR_DEPTH_HDR12, label: t.value.vddSettings.hdr12bit },
  ...(isColorDepthConflict.value ? [{ value: COLOR_DEPTH_CONFLICT, label: t.value.vddSettings.colorDepthConflict, disabled: true }] : []),
])
const hasLoadedSnapshot = ref(false)
const lastSavedSnapshot = ref('')
const createCalibrationStatus = () => ({
  supported: true,
  vddActive: false,
  hdrEnabled: false,
  calibrated: false,
  canCalibrate: false,
  state: 'waiting_stream',
  clientUuid: '',
  deviceId: '',
  sessionId: 0,
  profileName: null,
  profileHash: null,
  diagnostic: null,
  sharedVdd: false,
  activeClients: [],
  maxNits: null,
  minNits: null,
  maxFullFrameNits: null,
  statusText: '',
})
const calibrationStatus = reactive(createCalibrationStatus())

let calibrationContextPollTimer = null
let calibrationContextRequest = null

const {
  edidFileExists,
  edidFilePath,
  edidInfo,
  getEdidFormatLabel,
  checkEdidFile,
  handleEdidToggle,
  handleEdidFileChange,
  downloadEdid,
  removeEdidFile: removeEdidFileAction,
} = useVddEdid({ t, settings })

// the two mode lists of the driver, in the person's own order (the editor moves, edits, adds, removes; sorting is a button, never automatic)
const resolutionList = ref([])
const rateList = ref([...DEFAULT_REFRESH_RATES])
const setResolutionOptions = (values = []) => { resolutionList.value = cleanList('resolution', values) }
const setRefreshRateOptions = (values = []) => { rateList.value = cleanList('rate', values) }
const traceDisplayPath = computed(() => traceStatus.value.latest_file || t.value.vddSettings.vddTraceNoFile)
const hasCustomEdidIssue = computed(() => settings.edid.CustomEdid && !edidFileExists.value)
const isColorDepthConflict = computed(() => settings.colour.SDR10bit && settings.colour.HDRPlus)
const colorDepthProfile = computed({
  get() {
    if (settings.colour.SDR10bit && settings.colour.HDRPlus) {
      return COLOR_DEPTH_CONFLICT
    }

    if (settings.colour.SDR10bit) {
      return COLOR_DEPTH_SDR10
    }

    if (settings.colour.HDRPlus) {
      return COLOR_DEPTH_HDR12
    }

    return COLOR_DEPTH_DEFAULT
  },
  set(value) {
    settings.colour.SDR10bit = value === COLOR_DEPTH_SDR10
    settings.colour.HDRPlus = value === COLOR_DEPTH_HDR12
  },
})
const buildSettingsPayload = () => ({
  ...settings,
  gpu: {
    friendlyname: gpuFriendlyName.value.trim(),
  },
  global: {
    g_refresh_rate: [...rateList.value],
  },
  resolutions: {
    resolution: resolutionList.value.map((res) => {
      const [width, height] = res.split('x').map(Number)
      return { width, height }
    }),
  },
})

const takeSnapshot = () => JSON.stringify(buildSettingsPayload())

// the edits belong to the window's unsaved state (one bar for every page, saved from anywhere); the page reports them and takes them back
useVddPending({
  payload: buildSettingsPayload,
  saved: () => JSON.parse(lastSavedSnapshot.value || '{}'),
  ready: hasLoadedSnapshot,
  locale: () => locale.value,
  set: (path, value) => {
    if (path === 'gpu.friendlyname') { gpuFriendlyName.value = value; settings.gpu.friendlyname = value }
    else if (path === 'global.g_refresh_rate') setRefreshRateOptions((value || []).map(String))
    else if (path === 'resolutions.resolution') setResolutionOptions((value || []).map((r) => `${r.width}x${r.height}`))
    else setPath(settings, path, value)
  },
  onSaved: () => markSettingsClean(),
  onReverted: () => syncVddState({ silentLoad: true }),
})
const hasUnsavedChanges = computed(() => hasLoadedSnapshot.value && takeSnapshot() !== lastSavedSnapshot.value)
const vddReady = computed(() =>
  vddStatus.running &&
  vddStatus.control_available &&
  ['ready', 'degraded'].includes(vddStatus.state)
)
const isDriverCheckPending = computed(() => !hasCompletedDriverCheck.value)
const driverDetectionTitle = computed(() => driverCheckPhase.value === 'syncing'
  ? t.value.vddSettings.driverSyncTitle
  : t.value.vddSettings.driverDetectionTitle)
const driverDetectionDescription = computed(() => driverCheckPhase.value === 'syncing'
  ? t.value.vddSettings.driverSyncDesc
  : t.value.vddSettings.driverDetectionDesc)
const vddCanInstall = computed(() => !['unsupported', 'payload_missing'].includes(vddStatus.state))
const vddStatusLabel = useVddStatusLabel(t, vddStatus)


const stopCalibrationContextPolling = () => {
  if (calibrationContextPollTimer) {
    clearInterval(calibrationContextPollTimer)
    calibrationContextPollTimer = null
  }
}

const requestCalibration = async (snapshot = null) => {
  const body = snapshot ? JSON.stringify({
    clientUuid: snapshot.clientUuid, deviceId: snapshot.deviceId,
    sessionId: snapshot.sessionId, userSid: snapshot.diagnostic?.userSid,
    profileName: snapshot.profileName, profileHash: snapshot.profileHash,
  }) : undefined
  const proxyUrl = await sunshine.getProxyUrl()
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), 5000)
  try {
    const response = await fetch(`${proxyUrl}/api/runtime/hdr-calibration`, {
      method: snapshot ? 'POST' : 'GET',
      ...(snapshot ? { headers: { 'Content-Type': 'application/json' }, body } : {}),
      signal: controller.signal,
    })
    const context = await response.json()
    if (!response.ok || !context.success) throw new Error(context.status_message || t.value.vddSettings.calibrationRefreshFailed)
    return {
      ...context,
      canCalibrate: context.canCalibrate === true,
      activeClients: Array.isArray(context.activeClients) ? context.activeClients : [],
    }
  } finally {
    clearTimeout(timeout)
  }
}

const loadCalibrationContext = ({ silent = false, snapshot = null } = {}) => {
  if (calibrationContextRequest) {
    if (!snapshot) return calibrationContextRequest
    return calibrationContextRequest.then(() => loadCalibrationContext({ silent, snapshot }))
  }
  calibrationContextRequest = (async () => {
    if (!silent) isRefreshingCalibration.value = true
    try {
      const context = await requestCalibration(snapshot)
      Object.assign(calibrationStatus, createCalibrationStatus(), context, { statusText: '' })
      return context
    } catch (error) {
      console.error('Failed to load VDD calibration context:', error)
      Object.assign(calibrationStatus, createCalibrationStatus(), {
        state: 'unavailable',
        statusText: getErrorMessage(error, t.value.vddSettings.calibrationRefreshFailed),
      })
      return null
    } finally {
      calibrationContextRequest = null
      if (!silent) isRefreshingCalibration.value = false
    }
  })()
  return calibrationContextRequest
}

const startCalibrationContextPolling = () => {
  stopCalibrationContextPolling()
  if (activeSection.value !== 'calibration' || !vddReady.value) return
  loadCalibrationContext({ silent: true })
  calibrationContextPollTimer = setInterval(() => loadCalibrationContext({ silent: true }), 2500)
}






const markSettingsClean = () => {
  lastSavedSnapshot.value = takeSnapshot()
  hasLoadedSnapshot.value = true
}

const applyLoadedSettings = (data) => {
  const initialSettings = createInitialSettings()
  const mergedData = {
    ...initialSettings,
    ...data,
    monitors: data?.monitors || initialSettings.monitors,
    gpu: data?.gpu || initialSettings.gpu,
    global: data?.global || initialSettings.global,
    resolutions: data?.resolutions || initialSettings.resolutions,
    colour: data?.colour || initialSettings.colour,
    cursor: {
      ...(data?.cursor || initialSettings.cursor),
      // Cursor export is infrastructure. Sunshine switches ownership between
      // video composition and client rendering without reloading the driver.
      HardwareCursor: true,
    },
    edid: { ...initialSettings.edid, ...data?.edid },
  }

  Object.assign(settings, mergedData)

  gpuFriendlyName.value = mergedData.gpu?.friendlyname || ''
  settings.gpu.friendlyname = gpuFriendlyName.value

  const loadedResolutions = []
  for (const resolution of mergedData.resolutions?.resolution || []) {
    if (resolution.width && resolution.height) {
      loadedResolutions.push(`${resolution.width}x${resolution.height}`)
    }
  }

  const loadedRates = mergedData.global?.g_refresh_rate?.length
    ? mergedData.global.g_refresh_rate.map((rate) => String(rate))
    : initialSettings.global.g_refresh_rate
  setResolutionOptions(loadedResolutions)
  setRefreshRateOptions(loadedRates)
}

const applyTraceStatus = (data = {}) => {
  traceStatus.value = {
    running: Boolean(data.running),
    directory: data.directory || '',
    latest_file: data.latest_file || '',
  }
}

const refreshTraceStatus = async () => {
  try {
    const result = await vdd.getTraceStatus()
    if (result?.success) {
      applyTraceStatus(result.data)
    }
  } catch (error) {
    console.error('Failed to refresh VDD trace status:', error)
  }
}

const syncVddState = async ({ silentLoad = false } = {}) => Promise.all([
  loadSettings({ silent: silentLoad }),
  loadGPUs(),
  checkEdidFile(),
  refreshTraceStatus(),
])

const loadSettings = async ({ silent = false } = {}) => {
  isLoading.value = true
  try {
    const result = await vdd.loadSettings()

    if (!result?.success) {
      applyLoadedSettings(createInitialSettings())
      if (!silent) {
        ElMessage.warning(t.value.vddSettings.loadDefault)
      }
      markSettingsClean()
      return false
    }

    applyLoadedSettings(result.data)
    markSettingsClean()

    if (!silent) {
      ElMessage.success(t.value.vddSettings.reloadSuccess)
    }

    return true
  } catch (error) {
    console.error('Load settings error:', error)
    if (!silent) {
      ElMessage.error(t.value.vddSettings.loadFailed)
    }
    return false
  } finally {
    isLoading.value = false
  }
}

const reloadSettings = async () => {
  await syncVddState()
}

const refreshVddStatus = async () => {
  isCheckingDriver.value = true
  try {
    const result = await vdd.getStatus()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.driverStatusCheckFailed)
    }
    Object.assign(vddStatus, result.data)
    return vddReady.value
  } catch (error) {
    vddStatus.state = 'unknown'
    vddStatus.status_text = getErrorMessage(error, t.value.vddSettings.driverStatusCheckFailed)
    return false
  } finally {
    isCheckingDriver.value = false
  }
}

const recheckDriver = async () => {
  hasCompletedDriverCheck.value = false
  driverCheckPhase.value = 'detecting'
  try {
    if (await refreshVddStatus()) {
      driverCheckPhase.value = 'syncing'
      await syncVddState({ silentLoad: true })
    }
  } finally {
    hasCompletedDriverCheck.value = true
    openingCheck.value = false
  }
}

const installOrRepairDriver = async () => {
  if (hasUnsavedChanges.value) {
    ElMessage.warning(t.value.vddSettings.unsavedDesc)
    return
  }

  try {
    await ElMessageBox.confirm(
      t.value.vddSettings.installRepairConfirm,
      t.value.vddSettings.installRepairTitle,
      {
        confirmButtonText: t.value.vddSettings.installRepairDriver,
        cancelButtonText: t.value.vddSettings.cancel,
        type: 'warning',
      }
    )
    isInstallingDriver.value = true
    const install = () => installVddWithRecovery({
      install: () => vdd.install(),
      getStatus: () => vdd.getStatus(),
      onStatus: (status) => Object.assign(vddStatus, status),
      verificationError: t.value.vddSettings.installVerificationFailed,
    })

    await install()

    await syncVddState({ silentLoad: true })
    ElMessage.success(t.value.vddSettings.installRepairSuccess)
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(getErrorMessage(error, t.value.vddSettings.installRepairFailed))
    }
  } finally {
    isInstallingDriver.value = false
  }
}

const loadGPUs = async () => {
  try {
    const result = await vdd.getGPUs()
    if (result?.success) {
      syncGpuOptions(result.data)
    }
  } catch (error) {
    console.error('Failed to get GPU list:', error)
  }
}

const saveSettings = async () => {
  if (CHINESE_PATTERN.test(gpuFriendlyName.value)) {
    ElMessage.error(t.value.vddSettings.saveGpuError)
    return
  }

  if (settings.edid.CustomEdid && !edidFileExists.value) {
    ElMessage.error(t.value.vddSettings.customEdidMissingSave)
    return
  }

  isSaving.value = true
  try {
    const payload = buildSettingsPayload()
    const result = await vdd.saveSettings(payload)

    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.unknownError)
    }

    await syncVddState({ silentLoad: true })
    markSettingsClean()
    ElMessage.success(t.value.vddSettings.saveSuccessDetail)
  } catch (error) {
    console.error('Save settings error:', error)
    ElMessage.error(t.value.vddSettings.saveFailed.replace('{error}', getErrorMessage(error, t.value.vddSettings.unknownError)))
  } finally {
    isSaving.value = false
  }
}

const saveGpuEdit = () => {
  const trimmedValue = gpuFriendlyName.value.trim()
  if (CHINESE_PATTERN.test(trimmedValue)) {
    ElMessage.error(t.value.vddSettings.gpuNameNoChinese)
    gpuFriendlyName.value = settings.gpu.friendlyname || ''
    return
  }

  gpuFriendlyName.value = trimmedValue

  if (trimmedValue) {
    syncGpuOptions([...gpuOptions.value, trimmedValue])
  }

  settings.gpu.friendlyname = trimmedValue
}

const reloadDriver = async () => {
  isReloadingDriver.value = true
  try {
    const success = await vdd.execVddCmd('RELOAD_DRIVER')
    if (!success) {
      throw new Error(t.value.vddSettings.reloadDriverFailed)
    }

    ElMessage.success(t.value.vddSettings.reloadDriverSuccess)
  } catch (error) {
    ElMessage.error(getErrorMessage(error, t.value.vddSettings.reloadDriverFailed))
  } finally {
    isReloadingDriver.value = false
  }
}

const startTrace = async () => {
  isStartingTrace.value = true
  try {
    const result = await vdd.startTrace()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.vddTraceStartFailed)
    }

    applyTraceStatus(result.data)
    ElMessage.success(t.value.vddSettings.vddTraceStartSuccess)
  } catch (error) {
    ElMessage.error(getErrorMessage(error, t.value.vddSettings.vddTraceStartFailed))
  } finally {
    isStartingTrace.value = false
    await refreshTraceStatus()
  }
}

const stopTrace = async () => {
  isStoppingTrace.value = true
  try {
    const result = await vdd.stopTrace()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.vddTraceStopFailed)
    }

    applyTraceStatus(result.data)
    ElMessage.success(t.value.vddSettings.vddTraceStopSuccess)
  } catch (error) {
    ElMessage.error(getErrorMessage(error, t.value.vddSettings.vddTraceStopFailed))
  } finally {
    isStoppingTrace.value = false
    await refreshTraceStatus()
  }
}

const openTraceFolder = async () => {
  isOpeningTraceFolder.value = true
  try {
    const result = await vdd.openTraceFolder()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.vddTraceOpenFolderFailed)
    }
  } catch (error) {
    ElMessage.error(getErrorMessage(error, t.value.vddSettings.vddTraceOpenFolderFailed))
  } finally {
    isOpeningTraceFolder.value = false
  }
}

const removeEdidFile = async () => {
  isDeletingEdid.value = true
  try {
    await removeEdidFileAction()
  } finally {
    isDeletingEdid.value = false
  }
}

onMounted(async () => {
  await recheckDriver()
  if (vddReady.value) await loadCalibrationContext()
})

watch(activeSection, startCalibrationContextPolling)

onUnmounted(() => {
  stopCalibrationContextPolling()
})
</script>

<style>
.vdd-preview { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 14px; padding: 18px 20px; border-radius: 28px; background: var(--m-card); margin-bottom: 8px; }
.vdd-preview__item { display: flex; flex-direction: column; align-items: center; } .vdd-preview .m-dinfo__display { max-height: none; }
.vdd-preview__note { margin-left: auto; align-self: center; font: var(--m-label-large); color: var(--md-sys-color-on-surface-variant); font-variant-numeric: tabular-nums; }
.m-vdd-badges { display: flex; flex-wrap: wrap; gap: 8px; }
.m-tiles--static { --m-cols: 2; grid-auto-rows: minmax(104px, auto); min-height: 0; max-height: none; }
.m-chips-edit { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.m-chips-edit--suggest { margin-top: 8px; }
.m-chip--x { padding-right: 4px; } .m-chip--x button { display: inline-grid; place-items: center; width: 20px; height: 20px; border: 0; border-radius: 50%; padding: 0; background: transparent; color: inherit; cursor: pointer; font-size: 14px; } .m-chip--x button:hover { background: color-mix(in srgb, currentColor 14%, transparent); }
.m-chip--pick { border: 0; padding: 0 12px; cursor: pointer; background: transparent; box-shadow: inset 0 0 0 1px var(--md-sys-color-outline-variant); color: var(--md-sys-color-on-surface-variant); } .m-chip--pick:hover:not(:disabled) { background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent); } .m-chip--pick:disabled { opacity: .38; cursor: not-allowed; }
.m-inline-input { height: 32px; width: 140px; padding: 0 12px; border: 0; border-radius: 16px; outline: 0; background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface); font: var(--m-body-medium); box-shadow: inset 0 0 0 2px var(--md-sys-color-primary); }
</style>
