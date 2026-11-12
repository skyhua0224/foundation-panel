<template>
  <MPage notices="microphone" :title="t.m3e.input.microphone" :lead="text.intro" :eyebrow="t.m3e.input.title">
    <template #actions>
      <MIconButton icon="refresh" :label="t.deviceHub.refresh" :disabled="loading || testing" @click="refresh({ force: true })" />
    </template>
    <MStatusPill :icon="loadError ? 'error' : heroIcon" :label="loadError ? t.deviceHub.statusUnavailable : text.statusLabels[state]" :tone="loadError ? 'error' : heroTone === 'success' ? 'ok' : heroTone === 'warning' ? 'warn' : ''" :action="false">
      <template #actions>
        <MButton variant="tonal" icon="play_arrow" :loading="testing" :disabled="!canTest || testing" @click="runTest">{{ text.test }}</MButton>
      </template>
    </MStatusPill>

    <!-- the path of the sound, lit by the host's real state -->
    <div v-if="!loadError && status.configured_backend" class="mic-flow" role="img" :aria-label="t.m3e.micFlow.client + ' → ' + t.m3e.micFlow.apps">
      <template v-for="(node, i) in flow" :key="node.id">
        <i v-if="i" class="mic-flow__link" :class="{ 'is-on': node.on && flow[i - 1].on }" />
        <span class="mic-flow__node" :class="{ 'is-on': node.on }"><span class="mic-flow__shape"><MIcon :name="node.icon" /></span><b>{{ node.label }}</b></span>
      </template>
      <span v-if="status.buffered_bytes != null" class="mic-flow__buffer"><MProgress :value="Math.min(1, bufferedMs / 200)" :label="t.m3e.micFlow.buffer" /><small>{{ t.m3e.micFlow.buffer }} {{ Math.round(bufferedMs) }} ms</small></span>
    </div>

    <!-- live client microphone level: the only meaningful mic test while a stream runs -->
    <MGroup v-if="!loadError && liveInput">
      <MItem icon="graphic_eq" :title="text.level" :text="inputFresh ? text.levelActive : text.levelSilent">
        <template #trail><MProgress :value="levelShown" :label="text.level" /></template>
      </MItem>
    </MGroup>

    <MSection>
      <MGroup>
        <MItem icon="tune" :title="text.backend" :text="text.backendHints[backendChoice]" stack>
          <template #control>
            <MSeg :model-value="backendChoice" block :label="text.backend" :options="backendOptions" :disabled="loadError || savingBackend || !status.configured_backend" @update:model-value="onBackend" />
          </template>
        </MItem>
        <!-- VB-CABLE is listed whatever backend is chosen: its state, and the way to get it when the host does not find it -->
        <MItem data-vb-cable :data-vb-state="vbState" :data-vb-failure="vbFailure || undefined" icon="cable" :title="text.vbCable" :text="vbText" wrap>
          <template v-if="vbAction" #trail>
            <MButton v-if="vbNeedsEnable" variant="text" size="sm" icon="open_in_new" :disabled="vbBusy" @click="openDevices">{{ text.vbCableOpenDevmgmt }}</MButton>
            <MButton v-if="vbNeedsInstall" variant="text" size="sm" icon="download" :disabled="vbBusy" @click="openVbCablePage">{{ text.vbCableDownload }}</MButton>
            <MButton variant="tonal" size="sm" :icon="vbAction === 'enable' ? 'power_settings_new' : 'download'" :loading="vbBusy" :disabled="vbBusy" @click="askVbCable(vbAction)">{{ vbAction === 'enable' ? text.vbCableEnable : text.vbCableInstall }}</MButton>
          </template>
        </MItem>
      </MGroup>
    </MSection>

    <MSection>
      <MStatusPills :items="pills" :label="t.deviceHub.tabs.microphone" />
      <MFooter>{{ text.howToTest }}</MFooter>
      <MFooter>{{ text.defaultsNote }}</MFooter>
    </MSection>

    <MSection v-if="!loadError && (usesUsbip || status.device_created)">
      <MGroup>
        <MItem action :icon="diagnosticsOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" :title="text.diagnostics" :text="diagnosticsOpen ? '' : text.device + ' · ' + text.buffer" @click="diagnosticsOpen = !diagnosticsOpen" />
        <template v-if="diagnosticsOpen">
          <MItem v-for="[label, value] in diagnostics" :key="label" :title="label" plain>
            <template #trail><span class="m-muted">{{ value }}</span></template>
          </MItem>
        </template>
      </MGroup>
    </MSection>
  </MPage>
</template>

<script setup>
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, reactive, ref, watch, watchEffect } from 'vue'
import { ElMessage } from '../../composables/notifyMessage.js'
import { MPage, MSection, MGroup, MItem, MChip, MStatusPill, MFooter, useNotify, MSeg, MNotice, MButton, MIconButton, MSkeleton, MIcon, MProgress, MStatusPills, useDialogs } from 'foundation-ui/vue'
import { audioDevices, openDeviceManager, openExternalUrl, virtualMicrophone } from '../../tauri-adapter.js'
import { audioDevicesChannel, micStatusChannel } from '../../composables/deviceStatusStore.js'
import {
  MICROPHONE_BACKENDS, canTestMicrophone, microphoneBufferedMs, microphoneComponentMissing,
  microphoneOverviewState, microphoneStatusTone, microphoneUsesUsbip, microphoneInputFresh,
  microphoneInputLevel, microphoneTestIsLive, VB_CABLE_PAGE, vbCableErrorKind, vbCableJob, vbCableNeedsEnable, vbCableNeedsInstall, vbCableState,
} from '../../composables/deviceHubStatus.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['manage-components', 'back'])
const { t } = useI18n()
const text = computed(() => t.value.deviceHub.microphone)
const loading = ref(true)
const testing = ref(false)
const loadError = ref(false)
const status = reactive({})
const devices = ref(null)
let pollTimer
let disposed = false
let pending = false
const state = computed(() => loadError.value || !status.configured_backend ? 'unknown' : microphoneOverviewState(status))
const statusTone = computed(() => loadError.value ? 'state-error' : microphoneStatusTone(status))
const canTest = computed(() => !loadError.value && canTestMicrophone(status, loading.value))
const flow = computed(() => [
  { id: 'client', icon: 'mic', label: t.value.m3e.micFlow.client, on: !!status.host_streaming },
  { id: 'host', icon: 'dns', label: t.value.m3e.micFlow.host, on: !!status.host_streaming },
  { id: 'device', icon: 'settings_input_component', label: t.value.m3e.micFlow.device, on: !!status.device_created },
  { id: 'apps', icon: 'apps', label: t.value.m3e.micFlow.apps, on: !!status.online },
])
const usesUsbip = computed(() => microphoneUsesUsbip(status))
// VB-CABLE as the host finds it (the audio endpoint list); `unknown` until the host has answered
const vbState = computed(() => vbCableState(devices.value))
const vbNeedsInstall = computed(() => vbCableNeedsInstall(vbState.value))
const vbNeedsEnable = computed(() => vbCableNeedsEnable(vbState.value))
// the job of the host that installs VB-CABLE or enables its device: asked for first, run by the host, followed here
const vbJob = computed(() => vbCableJob(devices.value))
const requesting = ref(false)
const requestFailed = ref(false)
const vbBusy = computed(() => vbJob.value.running || requesting.value)
const vbAction = computed(() => (vbNeedsEnable.value ? 'enable' : vbNeedsInstall.value ? 'install' : ''))
const vbFailure = computed(() => (requestFailed.value ? 'failed' : vbState.value !== 'ready' && vbJob.value.failed ? vbCableErrorKind(vbJob.value.errorCode) : ''))
const vbText = computed(() => {
  if (vbJob.value.running) return text.value.vbCableJobs[vbJob.value.step]
  if (vbFailure.value) return text.value.vbCableErrors[vbFailure.value]
  return text.value.vbCableStates[vbState.value]
})
async function askVbCable(action) {
  const label = action === 'enable' ? text.value.vbCableEnable : text.value.vbCableInstall
  const confirmed = await useDialogs().confirm(action === 'enable' ? text.value.vbCableEnableConfirm : text.value.vbCableInstallConfirm, label, { type: 'warning', confirmText: label }).then(() => true, () => false)
  if (!confirmed) return
  requesting.value = true
  requestFailed.value = false
  try {
    const result = await audioDevices.startVbCableJob(action)
    if (!result?.success || (result.data?.success !== true && result.data?.error_code !== 'VB_BUSY')) requestFailed.value = true
  } catch {
    requestFailed.value = true
  } finally {
    requesting.value = false
    if (!disposed) await refresh({ force: true })
  }
}
const openVbCablePage = () => { void openExternalUrl(VB_CABLE_PAGE) }
const openDevices = () => { void openDeviceManager() }
const componentMissing = computed(() => microphoneComponentMissing(status))
const bufferedMs = computed(() => microphoneBufferedMs(status))
const liveInput = computed(() => Number(status.input_frames) > 0)
const inputFresh = computed(() => microphoneInputFresh(status))
const levelShown = ref(0)
let levelTimer
const backendChoice = ref('auto')
const savingBackend = ref(false)
// 错误码只翻译不外露；未知码给出通用说明。
const errorText = code => text.value.errors[code] || text.value.errors.unknown
// 活动后端是 VB-CABLE 时，USB 端的残留错误码不描述当前路径。
const problemCode = computed(() => status.active_backend === 'vb_cable' ? '' : status.error_code)
// the standing problems of this page go to its banner place (under the title), not to a banner of their own
const notify = useNotify()
const clearBanners = () => { for (const id of ['component', 'fallback', 'problem']) notify.clearBanner('microphone', id) }
watchEffect(() => {
  const shown = !loadError.value
  if (shown && componentMissing.value) {
    notify.banner('microphone', {
      id: 'component', tone: 'warn', icon: 'warning', title: text.value.missingTitle, text: status.configured_backend === 'auto' ? text.value.missingHintAuto : text.value.missingHintUsb,
      actions: [
        ...(status.configured_backend === 'usbip_experimental' ? [{ id: 'vb', label: text.value.useVbCable, onClick: () => chooseBackend('vb_cable') }] : []),
        { id: 'install', label: text.value.installTransport, onClick: () => emit('manage-components') },
      ],
    })
  } else notify.clearBanner('microphone', 'component')
  if (shown && status.fallback_reason) notify.banner('microphone', { id: 'fallback', tone: 'info', icon: 'info', title: text.value.fallback, text: text.value.fallbackReason.replace('{reason}', errorText(status.fallback_reason)) })
  else notify.clearBanner('microphone', 'fallback')
  if (shown && problemCode.value) notify.banner('microphone', { id: 'problem', tone: 'error', icon: 'error', title: text.value.problem, text: errorText(problemCode.value) })
  else notify.clearBanner('microphone', 'problem')
})
onUnmounted(clearBanners)
watch(() => status.configured_backend, backend => { if (backend && !savingBackend.value) backendChoice.value = backend }, { immediate: true })

async function refresh(options = {}) {
  if (pending || testing.value) return
  pending = true
  loading.value = true
  try {
    const [result, audio] = await Promise.all([
      micStatusChannel.fetch({ force: options?.force === true }),
      audioDevicesChannel.fetch({ force: options?.force === true }).catch(() => null),
    ])
    if (disposed) return
    loadError.value = !result?.success
    if (result?.success) Object.assign(status, result.data)
    if (audio?.success) devices.value = audio.data
    if (vbJob.value.running && !disposed) window.setTimeout(() => refresh({ force: true }), 1500)
  } catch {
    if (!disposed) loadError.value = true
  } finally {
    loading.value = false
    pending = false
  }
}
async function runTest() {
  if (testing.value || !canTest.value) return
  testing.value = true
  try {
    const result = await virtualMicrophone.test()
    const answer = result?.success ? result.data : null
    const backend = answer?.backend ? text.value.backends[answer.backend] || answer.backend : ''
    if (answer?.success && microphoneTestIsLive(answer)) {
      ElMessage.success(text.value.testLive.replace('{backend}', backend))
    } else if (answer?.success) {
      ElMessage.success(text.value.testSuccess.replace('{backend}', backend))
    } else {
      const reason = errorText(answer?.error_code)
      ElMessage.error(backend
        ? text.value.testFailed.replace('{backend}', backend).replace('{reason}', reason)
        : text.value.testFailedNoBackend.replace('{reason}', reason))
    }
  } catch {
    ElMessage.error(text.value.testFailedNoBackend.replace('{reason}', text.value.errors.unknown))
  } finally {
    testing.value = false
    if (!disposed) await refresh({ force: true })
  }
}
async function chooseBackend(backend) {
  backendChoice.value = backend
  await saveBackend(backend)
}
async function saveBackend(backend) {
  if (savingBackend.value || backend === status.configured_backend) return
  savingBackend.value = true
  try {
    const result = await virtualMicrophone.saveBackend(backend)
    if (result?.success) {
      ElMessage.success(text.value.saved)
    } else {
      backendChoice.value = status.configured_backend || 'auto'
      ElMessage.error(text.value.saveFailed)
    }
  } finally {
    savingBackend.value = false
    if (!disposed) await refresh({ force: true })
  }
}
const startPolling = () => {
  if (pollTimer || disposed) return
  pollTimer = window.setInterval(refresh, 3000)
}
watch(() => status.input_peak, () => {
  const peak = microphoneInputLevel(status)
  if (peak > levelShown.value) levelShown.value = peak
})
onMounted(() => {
  // Peak-hold with decay so a 3 s poll still shows one spoken phrase as a visible bump.
  levelTimer = window.setInterval(() => {
    levelShown.value = Math.max(0, levelShown.value - 0.04)
  }, 250)
})
const stopPolling = () => {
  if (pollTimer) {
    window.clearInterval(pollTimer)
    pollTimer = undefined
  }
}
onMounted(() => {
  refresh()
  startPolling()
})
// KeepAlive 下被隐藏时暂停 3s 轮询，重新显示时立即校准一次。
onActivated(() => {
  refresh()
  startPolling()
})
onDeactivated(stopPolling)
onUnmounted(() => { disposed = true; stopPolling(); window.clearInterval(levelTimer) })

const heroTone = computed(() => ({ 'state-ready': 'success', 'state-active': 'success' }[statusTone.value] || 'neutral'))
const heroIcon = computed(() => (loadError.value ? 'error' : state.value === 'idle' || state.value === 'unknown' ? 'mic' : 'check_circle'))
const backendOptions = computed(() => MICROPHONE_BACKENDS.map((value) => ({ value, label: text.value.backends[value] })))
const diagnosticsOpen = ref(false)
const diagnostics = computed(() => [
  [text.value.online, status.online ? text.value.onlineYes : text.value.onlineNo],
  [text.value.device, status.device_created ? text.value.deviceCreated : text.value.deviceNotCreated],
  [text.value.capture, status.host_streaming ? text.value.captureActive : text.value.captureIdle],
  [text.value.buffer, text.value.bufferMs.replace('{ms}', bufferedMs.value)],
  [text.value.underruns, status.underruns], [text.value.droppedFrames, status.dropped_frames], [text.value.submitErrors, status.submit_errors],
])
const onBackend = (value) => { backendChoice.value = value; saveBackend(value) }

// the backend in use and the component as labelled pills (what used to be two text rows)
const pills = computed(() => [
  { id: 'backend', icon: 'graphic_eq', label: text.value.activeBackend, value: status.active_backend ? text.value.backends[status.active_backend] || status.active_backend : text.value.noneActive, tone: status.active_backend ? 'live' : 'off' },
  { id: 'component', icon: status.component_available ? 'check_circle' : 'warning', label: text.value.component, value: status.component_available ? text.value.componentReady : text.value.componentMissing, tone: status.component_available ? 'ok' : 'warn' },
])
</script>
