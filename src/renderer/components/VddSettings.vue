<template>
  <MPage :title="pageTitle" :back="activeSection !== 'overview'" :back-label="t.m3e.back" @back="backVdd">
    <template #actions>
      <MIconButton v-if="!isDriverCheckPending && vddReady" icon="refresh" :label="t.vddSettings.reload" :disabled="isLoading" @click="reloadSettings" />
    </template>

    <MBanner v-if="!isDriverCheckPending && vddReady && vddStatus.state === 'degraded'" tone="warn" icon="warning">
      <b>{{ vddStatusLabel }}</b><br>{{ vddStatus.status_text }}
      <template #actions><MButton variant="text" size="sm" icon="download" :loading="isInstallingDriver" @click="installOrRepairDriver">{{ t.vddSettings.installRepairDriver }}</MButton></template>
    </MBanner>

    <!-- Driver check -->
    <template v-if="isDriverCheckPending">
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
    </template>

    <template v-else>
      <!-- Overview -->
      <template v-if="activeSection === 'overview'">
        <MStatusPill icon="desktop_windows" :label="vddStatusLabel" tone="ok" :action="false" />
        <div class="m-vdd-badges"><MChip v-for="badge in capabilityBadges" :key="badge.key" :tone="badge.tone === 'success' ? 'ok' : badge.tone === 'warning' ? 'warn' : 'info'">{{ badge.text }} · {{ badge.sub }}</MChip></div>
        <div class="m-tiles m-tiles--static">
          <MTile v-for="card in overviewCards" :key="card.key" :title="card.label" :size="'1x1'">
            <span class="m-num m-num--sm">{{ card.value }}</span>
            <template v-if="card.hint" #foot>{{ card.hint }}</template>
          </MTile>
        </div>
        <MGroup>
          <MItem action icon="monitor" :title="t.vddSettings.tabDisplay" @click="activeSection = 'display'" />
          <MItem action :icon="calibrationStatus.calibrated ? 'check_circle' : 'warning'" :title="t.vddSettings.calibrationTitle" :value="calibrationStatusTitle" @click="activeSection = 'calibration'" />
          <MItem action icon="build" :title="t.vddSettings.tabDriver" @click="activeSection = 'driver'" />
        </MGroup>
      </template>

      <!-- Display: resolutions, refresh rates, GPU, colour -->
      <template v-else-if="activeSection === 'display'">
        <MSection :title="t.vddSettings.displaySection">
          <MGroup>
            <MItem icon="aspect_ratio" :title="t.vddSettings.resolutionPresets" stack>
              <template #control>
                <div class="m-chips-edit">
                  <span v-for="res in sortedResolutionOptions" :key="res" class="m-chip m-chip--x">{{ res }}<button type="button" :aria-label="t.vddSettings.removeResolution || res" @click="removeResolution(res)"><MIcon name="close" /></button></span>
                  <input v-if="showResInput" ref="resInputRef" v-model="newResolution" class="m-inline-input" :placeholder="t.vddSettings.resPlaceholder" @keyup.enter="addResolution" @blur="handleResInputConfirm">
                  <MButton v-else size="sm" variant="tonal" icon="add" @click="showResolutionInput">{{ t.vddSettings.addResolution }}</MButton>
                </div>
                <div class="m-chips-edit m-chips-edit--suggest">
                  <button v-for="resolution in suggestedResolutions" :key="resolution" type="button" class="m-chip m-chip--pick" :disabled="resolutionOptions.has(resolution)" @click="quickAddResolution(resolution)">{{ resolution }}</button>
                </div>
              </template>
            </MItem>
            <MItem icon="speed" :title="t.vddSettings.refreshRatePresets" stack>
              <template #control>
                <div class="m-chips-edit">
                  <span v-for="rate in sortedRefreshRateOptions" :key="rate" class="m-chip m-chip--x">{{ rate }}Hz<button type="button" :aria-label="String(rate)" @click="removeRefreshRate(rate)"><MIcon name="close" /></button></span>
                  <input v-if="showRateInput" ref="rateInputRef" v-model="newRefreshRate" class="m-inline-input" :placeholder="t.vddSettings.ratePlaceholder" @keyup.enter="addRefreshRate" @blur="handleRateInputConfirm">
                  <MButton v-else size="sm" variant="tonal" icon="add" @click="showRefreshRateInput">{{ t.vddSettings.addRefreshRate }}</MButton>
                </div>
                <div class="m-chips-edit m-chips-edit--suggest">
                  <button v-for="rate in suggestedRefreshRates" :key="rate" type="button" class="m-chip m-chip--pick" :disabled="refreshRateOptions.has(rate)" @click="quickAddRefreshRate(rate)">{{ rate }}Hz</button>
                </div>
              </template>
            </MItem>
            <MFieldItem v-model="gpuFriendlyName" :label="t.vddSettings.gpuBinding" :placeholder="t.vddSettings.gpuPlaceholder" @change="saveGpuEdit" @keyup.enter="saveGpuEdit" />
            <MChoiceItem v-if="gpuOptions.length" :model-value="gpuFriendlyName" icon="developer_board" :title="t.vddSettings.gpuBinding" :options="gpuOptions.map((g) => ({ value: g, label: g }))" :cancel-label="t.common.cancel" @update:model-value="(v) => { gpuFriendlyName = v; saveGpuEdit() }" />
            <MItem icon="monitor" :title="t.vddSettings.monitorCount" :text="t.vddSettings.monitorCountTip" :value="String(settings.monitors.count)" />
            <MSwitchItem v-model="settings.edid.Vrr" icon="sync" :title="t.vddSettings.vrr" :text="t.vddSettings.vrrTip" />
          </MGroup>
        </MSection>
        <MSection :title="t.vddSettings.qualitySection">
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

      <!-- HDR calibration -->
      <template v-else-if="activeSection === 'calibration'">
        <MStatusPill :icon="calibrationStatus.calibrated ? 'check_circle' : 'warning'" :tone="calibrationStatusTone === 'success' ? 'ok' : calibrationStatusTone === 'danger' ? 'error' : 'warn'" :label="calibrationStatusTitle" :action="false">
          <template #actions><MIconButton icon="refresh" :label="t.vddSettings.calibrationRefresh" :disabled="isRefreshingCalibration" @click="loadCalibrationContext" /></template>
        </MStatusPill>
        <MFooter>{{ calibrationStatusDescription }}</MFooter>
        <MGroup v-if="calibrationStatus.calibrated">
          <MItem v-if="calibrationStatus.profileName" icon="description" :title="calibrationStatus.profileName" plain />
          <MItem icon="brightness_6" :title="t.vddSettings.calibrationPeak" :value="formatCalibrationNits(calibrationStatus.maxNits)" />
          <MItem icon="brightness_medium" :title="t.vddSettings.calibrationMinimum" :value="formatCalibrationNits(calibrationStatus.minNits)" />
          <MItem icon="contrast" :title="t.vddSettings.calibrationFullFrame" :value="formatCalibrationNits(calibrationStatus.maxFullFrameNits)" />
        </MGroup>
        <MGroup>
          <MItem icon="monitor" :title="t.vddSettings.calibrationClient" :value="activeCalibrationClient?.name || t.vddSettings.calibrationNoActiveClient" />
          <MCheckItem :title="t.vddSettings.calibrationStepStream" :text="t.vddSettings.calibrationStepStreamHint" :state="hasActiveHdrClient ? 'ok' : 'pending'" />
          <MCheckItem :title="t.vddSettings.calibrationStepVdd" :text="t.vddSettings.calibrationStepVddHint" :state="calibrationStatus.vddActive && calibrationStatus.hdrEnabled ? 'ok' : 'pending'" />
          <MCheckItem :title="t.vddSettings.calibrationStepRun" :text="t.vddSettings.calibrationStepRunHint" :state="calibrationStatus.calibrated ? 'ok' : 'pending'" />
        </MGroup>
        <MBanner v-if="vddReuseEnabled" tone="warn" icon="warning">{{ t.vddSettings.calibrationSharedWarning }}</MBanner>
        <div class="m-actions-row">
          <MButton icon="play_arrow" :disabled="!canLaunchCalibration" :loading="isLaunchingCalibration" @click="launchHdrCalibration">{{ calibrationStatus.calibrated ? t.vddSettings.calibrationRunAgain : t.vddSettings.calibrationLaunch }}</MButton>
          <MButton variant="tonal" icon="download" @click="openHdrCalibrationStore">{{ t.vddSettings.calibrationInstall }}</MButton>
        </div>
        <MGroup>
          <MItem action icon="light_mode" :title="t.vddSettings.physicalCalibrationTitle" :text="t.vddSettings.physicalCalibrationHint" :disabled="isLaunchingPhysicalCalibration" @click="launchPhysicalHdrCalibration" />
        </MGroup>
        <MFooter>{{ t.vddSettings.calibrationHint }}</MFooter>
      </template>

      <!-- Driver: tools, trace, EDID -->
      <template v-else-if="activeSection === 'driver'">
        <MSection :title="t.vddSettings.driverTools">
          <MGroup>
            <MItem action icon="refresh" :title="t.vddSettings.reloadDriver" :disabled="isReloadingDriver" @click="reloadDriver" />
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
            <MSwitchItem v-model="settings.edid.DolbyVision" icon="hdr_on" :title="t.vddSettings.dolbyVision" :text="settings.edid.CustomEdid && !settings.edid.EdidCeaOverride ? t.vddSettings.dolbyVisionCustomEdidTip : t.vddSettings.dolbyVisionTip" :disabled="settings.edid.CustomEdid && !settings.edid.EdidCeaOverride" />
            <MSwitchItem v-model="settings.edid.CustomEdid" icon="description" :title="t.vddSettings.customEdid" :text="t.vddSettings.customEdidTip" @update:model-value="handleEdidToggle" />
            <template v-if="settings.edid.CustomEdid">
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
      </template>

      <Transition name="m-savebar">
        <div v-if="hasUnsavedChanges || isSaving" class="m-savebar" role="region" :aria-label="t.vddSettings.saveAndApply">
          <span class="m-savebar__count">{{ t.vddSettings.unsavedBadge }}</span>
          <MButton variant="text" :disabled="isSaving" @click="reloadSettings">{{ t.vddSettings.reload }}</MButton>
          <MButton :loading="isSaving" icon="save" @click="saveSettings">{{ t.vddSettings.saveAndApply }}</MButton>
        </div>
      </Transition>
    </template>
  </MPage>
</template>

<script setup>
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { MPage, MGroup, MSection, MItem, MSwitchItem, MChoiceItem, MFieldItem, MSeg, MChip, MTile, MIcon, MBanner, MFooter, MButton, MIconButton, MStatusPill, MProgress, MCheckItem, useSubpage } from 'foundation-ui/vue'
import { useEditableOptionField } from '../composables/useEditableOptionField.js'
import { useVddEdid } from '../composables/useVddEdid.js'
import { useVddStatusLabel } from '../composables/useVddStatusLabel.js'
import { installVddWithRecovery } from '../composables/vddInstallRecovery.js'
import { openExternalUrl, sunshine, vdd } from '../tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'

const { t } = useI18n()

const DEFAULT_REFRESH_RATES = ['60', '120', '240']
const suggestedResolutions = ['1280x720', '1920x1080', '2560x1440', '3840x2160']
const suggestedRefreshRates = ['60', '120', '144', '240']
const RESOLUTION_PATTERN = /^\d+x\d+$/
const REFRESH_RATE_PATTERN = /^\d+(\.\d+)?$/
const CHINESE_PATTERN = /[\u4e00-\u9fa5]/
const capabilityBadges = [
  { key: 'hdr', tone: 'hdr', text: 'HDR', sub: 'dynamic range' },
  { key: 'resolution', tone: 'resolution', text: '4K', sub: 'ultra hd' },
  { key: 'refresh', tone: 'refresh', text: '240Hz', sub: 'high refresh' },
  { key: 'sync', tone: 'sync', text: 'VRR', sub: 'adaptive sync' },
]
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
    Vrr: false,
  },
})

const getErrorMessage = (error, fallback = '') => {
  if (error instanceof Error && error.message) {
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

const getVddText = (key, replacements = {}) => fillPlaceholders(t.value.vddSettings[key], replacements)

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
const driverCheckPhase = ref('detecting')
const isInstallingDriver = ref(false)
const isSaving = ref(false)
const isReloadingDriver = ref(false)
const isDeletingEdid = ref(false)
const isStartingTrace = ref(false)
const isStoppingTrace = ref(false)
const isOpeningTraceFolder = ref(false)
const isRefreshingCalibration = ref(false)
const isLaunchingCalibration = ref(false)
const isLaunchingPhysicalCalibration = ref(false)
const { current: vddSub, open: openVddSub, back: backVdd } = useSubpage('vdd')
const activeSection = computed({
  get: () => vddSub.value || 'overview',
  set: (value) => { if (value === 'overview') { if (vddSub.value) backVdd() } else openVddSub(value) },
})
const pageTitle = computed(() => ({ display: t.value.vddSettings.tabDisplay, calibration: t.value.vddSettings.calibrationTitle, driver: t.value.vddSettings.tabDriver }[activeSection.value] || t.value.vddSettings.title))
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
const activeHdrClients = ref([])
const vddReuseEnabled = ref(false)
const calibrationStatus = reactive({
  supported: true,
  vddActive: false,
  hdrEnabled: false,
  calibrated: false,
  profileName: null,
  maxNits: null,
  minNits: null,
  maxFullFrameNits: null,
  statusText: '',
})
let calibrationContextPollTimer = null
let isCalibrationContextLoading = false

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

const compareResolutions = (a, b) => {
  const [aWidth, aHeight] = a.split('x').map(Number)
  const [bWidth, bHeight] = b.split('x').map(Number)
  return aWidth * aHeight - bWidth * bHeight || aWidth - bWidth || aHeight - bHeight
}

const compareRefreshRates = (a, b) => parseFloat(a) - parseFloat(b)

const {
  options: resolutionOptions,
  draft: newResolution,
  visible: showResInput,
  inputRef: resInputRef,
  sortedOptions: sortedResolutionOptions,
  setValues: setResolutionOptions,
  showInput: showResolutionInput,
  addDraft: addResolution,
  addValue: quickAddResolution,
  confirmInput: handleResInputConfirm,
  removeValue: removeResolution,
} = useEditableOptionField({
  compare: compareResolutions,
  validate: (value) => {
    if (!RESOLUTION_PATTERN.test(value)) {
      return false
    }

    const [width, height] = value.split('x').map(Number)
    return width > 0 && height > 0
  },
  messages: {
    invalid: { type: 'warning', text: () => t.value.vddSettings.resolutionFormatError },
    exists: { type: 'info', text: () => t.value.vddSettings.resolutionExists },
    added: { type: 'success', text: (value) => getVddText('resolutionAdded', { value }) },
    removed: { type: 'info', text: (value) => getVddText('resolutionRemoved', { value }) },
    minOne: { type: 'error', text: () => t.value.vddSettings.resolutionMinOne },
  },
})

const {
  options: refreshRateOptions,
  draft: newRefreshRate,
  visible: showRateInput,
  inputRef: rateInputRef,
  sortedOptions: sortedRefreshRateOptions,
  setValues: setRefreshRateOptions,
  showInput: showRefreshRateInput,
  addDraft: addRefreshRate,
  addValue: quickAddRefreshRate,
  confirmInput: handleRateInputConfirm,
  removeValue: removeRefreshRate,
} = useEditableOptionField({
  initialValues: DEFAULT_REFRESH_RATES,
  compare: compareRefreshRates,
  validate: (value) => {
    if (!REFRESH_RATE_PATTERN.test(value)) {
      return false
    }

    const rate = parseFloat(value)
    return rate >= 1 && rate <= 480
  },
  messages: {
    invalid: { type: 'warning', text: () => t.value.vddSettings.refreshRateInvalidExt },
    exists: { type: 'warning', text: () => t.value.vddSettings.refreshRateExists },
    added: { type: 'success', text: (value) => getVddText('refreshRateAdded', { value }) },
    removed: { type: 'info', text: (value) => getVddText('refreshRateRemoved', { value }) },
    minOne: { type: 'error', text: () => t.value.vddSettings.refreshRateMinOne },
  },
})

const gpuSummary = computed(() => gpuFriendlyName.value || t.value.vddSettings.unboundGpu)
const presetSummary = computed(() => getVddText('presetsSummary', {
  resolutions: sortedResolutionOptions.value.length,
  rates: sortedRefreshRateOptions.value.length,
}))
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
const currentVddModeLabel = computed(() => vddReuseEnabled.value
  ? t.value.vddSettings.sharedVddMode
  : t.value.vddSettings.dedicatedVddMode)
const currentVddModeHint = computed(() => vddReuseEnabled.value
  ? t.value.vddSettings.sharedVddModeDesc
  : t.value.vddSettings.dedicatedVddModeDesc)
const overviewCards = computed(() => [
  {
    key: 'mode',
    label: t.value.vddSettings.overviewMode,
    value: currentVddModeLabel.value,
    hint: currentVddModeHint.value,
  },
  {
    key: 'presets',
    label: t.value.vddSettings.overviewPresets,
    value: presetSummary.value,
  },
  {
    key: 'gpu',
    label: t.value.vddSettings.overviewGpu,
    value: gpuSummary.value,
  },
])
const sectionTabs = computed(() => [
  { key: 'overview', label: t.value.vddSettings.tabOverview, icon: DataAnalysis },
  { key: 'display', label: t.value.vddSettings.tabDisplay, icon: Monitor },
  { key: 'calibration', label: t.value.vddSettings.tabCalibration, icon: Sunny },
  { key: 'driver', label: t.value.vddSettings.tabDriver, icon: Tools },
])

const focusSectionByIndex = async (index) => {
  const tabs = sectionTabs.value
  if (!tabs.length) return
  const normalizedIndex = (index + tabs.length) % tabs.length
  activeSection.value = tabs[normalizedIndex].key
  await nextTick()
  document.getElementById(`vdd-tab-${activeSection.value}`)?.focus()
}

const focusAdjacentSection = (key, offset) => {
  const currentIndex = sectionTabs.value.findIndex((tab) => tab.key === key)
  if (currentIndex >= 0) focusSectionByIndex(currentIndex + offset)
}
const heroActions = computed(() => [
  {
    key: 'reload',
    label: t.value.vddSettings.reload,
    icon: Refresh,
    loading: isLoading.value,
    plain: true,
    type: undefined,
    onClick: reloadSettings,
  },
  {
    key: 'save',
    label: t.value.vddSettings.saveAndApply,
    icon: UploadFilled,
    loading: isSaving.value,
    plain: !hasUnsavedChanges.value,
    type: hasUnsavedChanges.value ? 'primary' : undefined,
    onClick: saveSettings,
  },
])
const buildSettingsPayload = () => ({
  ...settings,
  gpu: {
    friendlyname: gpuFriendlyName.value.trim(),
  },
  global: {
    g_refresh_rate: [...sortedRefreshRateOptions.value],
  },
  resolutions: {
    resolution: sortedResolutionOptions.value.map((res) => {
      const [width, height] = res.split('x').map(Number)
      return { width, height }
    }),
  },
})

const takeSnapshot = () => JSON.stringify(buildSettingsPayload())

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
const activeCalibrationClient = computed(() => activeHdrClients.value[0] || null)
const hasActiveHdrClient = computed(() => Boolean(activeCalibrationClient.value))
const canLaunchCalibration = computed(() =>
  hasActiveHdrClient.value &&
  calibrationStatus.supported &&
  calibrationStatus.vddActive &&
  calibrationStatus.hdrEnabled &&
  !vddReuseEnabled.value
)
const calibrationStatusTone = computed(() => {
  if (calibrationStatus.calibrated) return 'success'
  if (!calibrationStatus.supported) return 'danger'
  if (!calibrationStatus.vddActive || !calibrationStatus.hdrEnabled) return 'warning'
  return 'ready'
})
const calibrationStatusTitle = computed(() => {
  if (calibrationStatus.calibrated) return t.value.vddSettings.calibrationDetected
  if (!calibrationStatus.supported) return t.value.vddSettings.calibrationUnsupported
  if (!calibrationStatus.vddActive) return t.value.vddSettings.calibrationWaitingVdd
  if (!calibrationStatus.hdrEnabled) return t.value.vddSettings.calibrationWaitingHdr
  return t.value.vddSettings.calibrationReady
})
const calibrationStatusDescription = computed(() => {
  if (calibrationStatus.statusText) return calibrationStatus.statusText
  if (calibrationStatus.calibrated) return t.value.vddSettings.calibrationDetectedHint
  if (!calibrationStatus.vddActive) return t.value.vddSettings.calibrationWaitingStream
  if (!calibrationStatus.hdrEnabled) return t.value.vddSettings.calibrationWaitingHdrHint
  return t.value.vddSettings.calibrationReadyHint
})

const formatCalibrationNits = (value) => {
  if (!Number.isFinite(value)) return '—'
  return `${value < 1 ? value.toFixed(4).replace(/0+$/, '').replace(/\.$/, '') : Math.round(value)} nits`
}

const stopCalibrationContextPolling = () => {
  if (calibrationContextPollTimer) {
    clearInterval(calibrationContextPollTimer)
    calibrationContextPollTimer = null
  }
}

const loadCalibrationContext = async ({ silent = false } = {}) => {
  if (isCalibrationContextLoading) return
  isCalibrationContextLoading = true
  if (!silent) isRefreshingCalibration.value = true
  try {
    const proxyUrl = await sunshine.getProxyUrl()
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 2000)
    let response
    try {
      response = await fetch(`${proxyUrl}/api/runtime/hdr-calibration`, { signal: controller.signal })
    } finally {
      clearTimeout(timeout)
    }
    if (!response.ok) throw new Error(`Sunshine API request failed (${response.status})`)
    const context = await response.json()
    if (!context.success) throw new Error(context.status_message || t.value.vddSettings.calibrationRefreshFailed)

    Object.assign(calibrationStatus, context, { statusText: '' })
    activeHdrClients.value = Array.isArray(context.activeClients) ? context.activeClients : []
    vddReuseEnabled.value = Boolean(context.sharedVdd)
  } catch (error) {
    console.error('Failed to load VDD calibration context:', error)
    activeHdrClients.value = []
    vddReuseEnabled.value = false
    Object.assign(calibrationStatus, {
      vddActive: false,
      hdrEnabled: false,
      calibrated: false,
      profileName: null,
      maxNits: null,
      minNits: null,
      maxFullFrameNits: null,
      statusText: getErrorMessage(error, t.value.vddSettings.calibrationRefreshFailed),
    })
  } finally {
    isCalibrationContextLoading = false
    if (!silent) isRefreshingCalibration.value = false
  }
}

const startCalibrationContextPolling = () => {
  stopCalibrationContextPolling()
  if (activeSection.value !== 'calibration' || !vddReady.value) return
  loadCalibrationContext({ silent: true })
  calibrationContextPollTimer = setInterval(() => loadCalibrationContext({ silent: true }), 2500)
}

const launchHdrCalibration = async () => {
  try {
    await ElMessageBox.confirm(
      t.value.vddSettings.calibrationLaunchConfirm,
      t.value.vddSettings.calibrationLaunchTitle,
      {
        confirmButtonText: t.value.vddSettings.calibrationLaunch,
        cancelButtonText: t.value.vddSettings.cancel,
        type: 'info',
      }
    )
    isLaunchingCalibration.value = true
    const result = await vdd.launchHdrCalibration()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.calibrationLaunchFailed)
    }
    ElMessage.success(t.value.vddSettings.calibrationLaunched)
    startCalibrationContextPolling()
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(getErrorMessage(error, t.value.vddSettings.calibrationLaunchFailed))
    }
  } finally {
    isLaunchingCalibration.value = false
  }
}

const launchPhysicalHdrCalibration = async () => {
  try {
    await ElMessageBox.confirm(
      t.value.vddSettings.physicalCalibrationConfirm,
      t.value.vddSettings.physicalCalibrationTitle,
      {
        confirmButtonText: t.value.vddSettings.physicalCalibrationLaunch,
        cancelButtonText: t.value.vddSettings.cancel,
        type: 'info',
      }
    )
    isLaunchingPhysicalCalibration.value = true
    const result = await vdd.launchHdrCalibration()
    if (!result?.success) {
      throw new Error(result?.message || t.value.vddSettings.calibrationLaunchFailed)
    }
    ElMessage.success(t.value.vddSettings.physicalCalibrationLaunched)
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(getErrorMessage(error, t.value.vddSettings.calibrationLaunchFailed))
    }
  } finally {
    isLaunchingPhysicalCalibration.value = false
  }
}

const openHdrCalibrationStore = async () => {
  try {
    const opened = await openExternalUrl('ms-windows-store://pdp/?ProductId=9N7F2SM5D1LR')
    if (!opened) ElMessage.error(t.value.vddSettings.calibrationStoreFailed)
  } catch (error) {
    console.error('Failed to open HDR calibration store page:', error)
    ElMessage.error(t.value.vddSettings.calibrationStoreFailed)
  }
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
.m-vdd-badges { display: flex; flex-wrap: wrap; gap: 8px; }
.m-tiles--static { --m-cols: 2; grid-auto-rows: minmax(104px, auto); }
.m-chips-edit { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.m-chips-edit--suggest { margin-top: 8px; }
.m-chip--x { padding-right: 4px; } .m-chip--x button { display: inline-grid; place-items: center; width: 20px; height: 20px; border: 0; border-radius: 50%; padding: 0; background: transparent; color: inherit; cursor: pointer; font-size: 14px; } .m-chip--x button:hover { background: color-mix(in srgb, currentColor 14%, transparent); }
.m-chip--pick { border: 0; padding: 0 12px; cursor: pointer; background: transparent; box-shadow: inset 0 0 0 1px var(--md-sys-color-outline-variant); color: var(--md-sys-color-on-surface-variant); } .m-chip--pick:hover:not(:disabled) { background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent); } .m-chip--pick:disabled { opacity: .38; cursor: not-allowed; }
.m-inline-input { height: 32px; width: 140px; padding: 0 12px; border: 0; border-radius: 16px; outline: 0; background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface); font: var(--m-body-medium); box-shadow: inset 0 0 0 2px var(--md-sys-color-primary); }
</style>
