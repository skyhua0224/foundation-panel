<template>
  <MPage :title="t.m3e.input.microphone" :lead="text.intro" :eyebrow="t.m3e.input.title">
    <template #actions>
      <MIconButton icon="refresh" :label="t.deviceHub.refresh" :disabled="loading || testing" @click="refresh({ force: true })" />
    </template>
    <MStatusPill :icon="loadError ? 'error' : heroIcon" :label="loadError ? t.deviceHub.statusUnavailable : text.statusLabels[state]" :tone="loadError ? 'error' : heroTone === 'success' ? 'ok' : heroTone === 'warning' ? 'warn' : ''" :action="false">
      <template #actions>
        <MButton variant="tonal" icon="play_arrow" :loading="testing" :disabled="!canTest || testing" @click="runTest">{{ testing ? text.testing : text.test }}</MButton>
      </template>
    </MStatusPill>

    <MBanner v-if="!loadError && componentMissing" tone="warn" icon="warning">
      <b>{{ text.missingTitle }}</b><br>{{ status.configured_backend === 'auto' ? text.missingHintAuto : text.missingHintUsb }}
      <template #actions>
        <MButton v-if="status.configured_backend === 'usbip_experimental'" size="sm" variant="text" :loading="savingBackend" @click="chooseBackend('vb_cable')">{{ text.useVbCable }}</MButton>
        <MButton size="sm" variant="text" @click="emit('manage-components')">{{ text.installTransport }}</MButton>
      </template>
    </MBanner>
    <MBanner v-if="!loadError && status.fallback_reason" tone="info" icon="info"><b>{{ text.fallback }}</b><br>{{ text.fallbackReason.replace('{reason}', errorText(status.fallback_reason)) }}</MBanner>
    <MBanner v-if="!loadError && problemCode" tone="error" icon="error"><b>{{ text.problem }}</b><br>{{ errorText(problemCode) }}</MBanner>

    <MSection :title="text.backend">
      <MGroup>
        <MItem icon="tune" :title="text.backend" :text="text.backendHints[backendChoice]" stack>
          <template #control>
            <MSeg :model-value="backendChoice" block :label="text.backend" :options="backendOptions" :disabled="loadError || savingBackend || !status.configured_backend" @update:model-value="onBackend" />
          </template>
        </MItem>
      </MGroup>
    </MSection>

    <MSection :title="t.deviceHub.tabs.microphone">
      <MGroup>
        <MItem icon="graphic_eq" :title="text.activeBackend" :text="status.active_backend ? text.backends[status.active_backend] || status.active_backend : text.noneActive" />
        <MItem icon="extension" :title="text.component" :text="status.component_available ? text.componentReady : text.componentMissing">
          <template #trail><MChip :tone="status.component_available ? 'ok' : 'warn'" :icon="status.component_available ? 'check_circle' : 'warning'">{{ status.component_available ? text.componentReady : text.componentMissing }}</MChip></template>
        </MItem>
              </MGroup>
      <MFooter>{{ text.defaultsNote }}</MFooter>
    </MSection>

    <MSection v-if="!loadError && (usesUsbip || status.device_created)" :title="text.diagnostics">
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
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { MPage, MSection, MGroup, MItem, MChip, MStatusPill, MBanner, MFooter, MSeg, MNotice, MButton, MIconButton, MSkeleton } from 'foundation-ui/vue'
import { virtualMicrophone } from '../../tauri-adapter.js'
import { micStatusChannel } from '../../composables/deviceStatusStore.js'
import {
  MICROPHONE_BACKENDS, canTestMicrophone, microphoneBufferedMs, microphoneComponentMissing,
  microphoneOverviewState, microphoneStatusTone, microphoneUsesUsbip,
} from '../../composables/deviceHubStatus.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['manage-components', 'back'])
const { t } = useI18n()
const text = computed(() => t.value.deviceHub.microphone)
const loading = ref(true)
const testing = ref(false)
const loadError = ref(false)
const status = reactive({})
let pollTimer
let disposed = false
let pending = false
const state = computed(() => loadError.value || !status.configured_backend ? 'unknown' : microphoneOverviewState(status))
const statusTone = computed(() => loadError.value ? 'state-error' : microphoneStatusTone(status))
const canTest = computed(() => !loadError.value && canTestMicrophone(status, loading.value))
const usesUsbip = computed(() => microphoneUsesUsbip(status))
const componentMissing = computed(() => microphoneComponentMissing(status))
const bufferedMs = computed(() => microphoneBufferedMs(status))
const backendChoice = ref('auto')
const savingBackend = ref(false)
// 错误码只翻译不外露；未知码给出通用说明。
const errorText = code => text.value.errors[code] || text.value.errors.unknown
// 活动后端是 VB-CABLE 时，USB 端的残留错误码不描述当前路径。
const problemCode = computed(() => status.active_backend === 'vb_cable' ? '' : status.error_code)
watch(() => status.configured_backend, backend => { if (backend && !savingBackend.value) backendChoice.value = backend }, { immediate: true })

async function refresh(options = {}) {
  if (pending || testing.value) return
  pending = true
  loading.value = true
  try {
    const result = await micStatusChannel.fetch({ force: options?.force === true })
    if (disposed) return
    loadError.value = !result?.success
    if (result?.success) Object.assign(status, result.data)
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
    if (answer?.success) {
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
onUnmounted(() => { disposed = true; stopPolling() })

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
</script>
