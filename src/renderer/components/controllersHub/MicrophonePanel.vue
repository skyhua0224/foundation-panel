<template>
  <section class="chub-panel">
    <div class="chub-window" :class="statusTone">
      <span class="chub-window-tab">{{ text.title }}</span>
      <div class="chub-hud-row chub-hud-row--divided">
        <div class="chub-hud-state"><span class="chub-status-dot"></span><strong>{{ text.statusLabels[state] }}</strong></div>
        <div class="chub-hud-actions">
          <el-button size="small" :loading="loading" :disabled="testing" @click="refresh({ force: true })">{{ t.deviceHub.refresh }}</el-button>
          <el-button size="small" type="primary" :loading="testing" :disabled="!canTest || testing" @click="runTest">{{ testing ? text.testing : text.test }}</el-button>
        </div>
      </div>
      <p class="chub-hint">{{ text.intro }}</p>
      <p>{{ text.summaries[state] }}</p>

      <div class="chub-mic-backend">
        <span class="chub-mic-backend-label">{{ text.backend }}</span>
        <el-radio-group v-model="backendChoice" :disabled="loadError || savingBackend || !status.configured_backend" size="small" @change="saveBackend">
          <el-radio-button v-for="backend in MICROPHONE_BACKENDS" :key="backend" :value="backend">{{ text.backends[backend] }}</el-radio-button>
        </el-radio-group>
        <p class="chub-hint">{{ text.backendHints[backendChoice] }}</p>
      </div>

      <el-alert
        v-if="!loadError && componentMissing"
        class="chub-notice"
        type="warning"
        :closable="false"
        :title="text.missingTitle"
      >
        <p>{{ status.configured_backend === 'auto' ? text.missingHintAuto : text.missingHintUsb }}</p>
        <div class="chub-card-actions">
          <el-button v-if="status.configured_backend === 'usbip_experimental'" size="small" type="primary" :loading="savingBackend" @click="chooseBackend('vb_cable')">{{ text.useVbCable }}</el-button>
          <el-button size="small" @click="emit('manage-components')">{{ text.installTransport }}</el-button>
        </div>
      </el-alert>
      <el-alert v-if="!loadError && status.fallback_reason" class="chub-notice" type="info" :closable="false" :title="text.fallback" :description="text.fallbackReason.replace('{reason}', errorText(status.fallback_reason))" />
      <el-alert v-if="!loadError && problemCode" class="chub-notice" type="error" :closable="false" :title="text.problem" :description="errorText(problemCode)" />
      <p v-if="loadError" role="alert" class="chub-status-error">{{ t.deviceHub.statusUnavailable }}</p>

      <div class="chub-runtime-grid">
        <div><span>{{ text.activeBackend }}</span><strong>{{ status.active_backend ? text.backends[status.active_backend] || status.active_backend : text.noneActive }}</strong></div>
        <div><span>{{ text.component }}</span><strong>{{ status.component_available ? text.componentReady : text.componentMissing }}</strong></div>
      </div>
      <p class="chub-hint">{{ text.defaultsNote }}</p>
    </div>

    <details v-if="!loadError && (usesUsbip || status.device_created)" class="chub-section chub-disclosure chub-disclosure--section">
      <summary>
        <span class="chub-section-label">◈ {{ text.diagnostics }}</span>
        <span class="chub-section-rule"></span>
        <span class="chub-disclosure-mark" aria-hidden="true">▸</span>
      </summary>
      <div class="chub-runtime-grid chub-mic-grid">
        <div><span>{{ text.online }}</span><strong>{{ status.online ? text.onlineYes : text.onlineNo }}</strong></div>
        <div><span>{{ text.device }}</span><strong>{{ status.device_created ? text.deviceCreated : text.deviceNotCreated }}</strong></div>
        <div><span>{{ text.capture }}</span><strong>{{ status.host_streaming ? text.captureActive : text.captureIdle }}</strong></div>
        <div><span>{{ text.buffer }}</span><strong>{{ text.bufferMs.replace('{ms}', bufferedMs) }}</strong></div>
        <div><span>{{ text.underruns }}</span><strong>{{ status.underruns }}</strong></div>
        <div><span>{{ text.droppedFrames }}</span><strong>{{ status.dropped_frames }}</strong></div>
        <div><span>{{ text.submitErrors }}</span><strong>{{ status.submit_errors }}</strong></div>
      </div>
    </details>
  </section>
</template>

<script setup>
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { virtualMicrophone } from '../../tauri-adapter.js'
import { micStatusChannel } from '../../composables/deviceStatusStore.js'
import {
  MICROPHONE_BACKENDS, canTestMicrophone, microphoneBufferedMs, microphoneComponentMissing,
  microphoneOverviewState, microphoneStatusTone, microphoneUsesUsbip,
} from '../../composables/deviceHubStatus.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['manage-components'])
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
</script>
