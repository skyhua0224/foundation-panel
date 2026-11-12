<template>
  <div class="m-tool" :class="{ 'is-embedded': embedded }">
    <header v-if="!embedded" class="m-tool__head"><h2>{{ t.bitrateTool.title }}</h2><MIconButton icon="close" :label="t.toolWindow.close" @click="$emit('close')" /></header>
    <MBanner v-if="problem" tone="error" icon="error">{{ problem }}</MBanner>
    <MGroup>
      <MChoiceItem v-model="selectedClient" icon="cast_connected" :title="t.bitrateTool.selectClient" :options="clientOptions" :disabled="isLoading || applying || !activeSessions.length" :cancel-label="t.common.cancel" @update:model-value="onClientChange" />
      <MItem action icon="refresh" :title="t.bitrateTool.refresh" :disabled="isLoading || applying || refreshing" @click="loadSessions(true)">
        <template #trail><MLoading v-if="refreshing" size="20px" /></template>
      </MItem>
    </MGroup>

    <div v-if="isLoading" class="m-tool__wait"><MLoading size="40px" /><span>{{ t.bitrateTool.loading }}</span></div>
    <template v-else-if="activeSessions.length === 0">
      <MEmpty icon="link" :title="t.bitrateTool.noStream" :text="allSessions.length > 0 ? t.bitrateTool.sessionsIdle.replace('{count}', allSessions.length) : t.bitrateTool.noStreamSub" />
    </template>
    <template v-else-if="selectedClient">
      <div class="m-tool__hero" role="status">
        <b class="m-tool__big">{{ formatBitrate(bitrateValue) }}</b>
        <MChip v-if="currentBitrate" tone="info" icon="monitoring">{{ t.bitrateTool.currentBitrate }} {{ formatBitrate(currentBitrate) }}</MChip>
      </div>
      <MGroup>
        <MSliderItem v-model="bitrateProgress" icon="speed" :title="t.bitrateTool.targetBitrate" :min="bitrateSpan.min" :max="bitrateSpan.max" :step="1" :decimals="0" :marks="bitrateMarks" size="m" icon-start="signal_cellular_alt_2_bar" icon-end="signal_cellular_alt" :readout-text="formatBitrate(bitrateValue)" :disabled="applying" />
        <MItem stack title=""><template #control><MChipSet v-model="bitrateValue" :label="t.bitrateTool.targetBitrate" :options="BITRATE_PRESETS.map((v) => ({ value: v, label: formatBitrate(v) }))" :disabled="applying" /></template></MItem>
        <MItem stack icon="edit" title=""><template #control><MField v-model.number="bitrateValue" type="number" :label="t.bitrateTool.inputPlaceholder" suffix="Kbps" :error="inRange ? '' : rangeText" :disabled="applying" /></template></MItem>
      </MGroup>
      <div class="m-tool__actions"><MButton icon="check" :loading="applying" :disabled="applying || !selectedClient || !inRange" @click="applyBitrate">{{ applying ? t.bitrateTool.applying : t.bitrateTool.apply }}</MButton></div>
    </template>
    <MEmpty v-else icon="arrow_upward" :title="t.bitrateTool.selectFirst" />
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { sunshine } from '../../tauri-adapter.js'
import { bitrateChipMarks, bitrateFromProgress, bitrateSliderProgress, bitrateSliderSpan } from 'foundation-ui/settings/index.js'
import { MBanner, MButton, MChip, MChipSet, MChoiceItem, MEmpty, MField, MGroup, MIconButton, MItem, MLoading, MSliderItem, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

const { t } = useI18n()

// 常量定义
const BITRATE_LIMITS = Object.freeze({
  MIN: 1000,
  MAX: 800000,
  STEP: 1000,
})

const BITRATE_PRESETS = Object.freeze([5000, 10000, 20000, 50000, 100000, 200000])
const DEFAULT_BITRATE = 20000
const MESSAGE_TIMEOUT = 5000
const REFRESH_DELAY = 1000

defineProps({
  embedded: {
    type: Boolean,
    default: false
  }
})

defineEmits(['close'])

// 响应式状态
const activeSessions = ref([])
const allSessions = ref([])
const selectedClient = ref('')
const bitrateValue = ref(DEFAULT_BITRATE)
// the shared segmented bitrate scale between the limits; the marks stand under the chips
const bitrateSpan = bitrateSliderSpan(BITRATE_LIMITS.MIN, BITRATE_LIMITS.MAX)
const bitrateMarks = bitrateChipMarks(BITRATE_PRESETS.map((kbps) => kbps / 1000), BITRATE_LIMITS.MIN, BITRATE_LIMITS.MAX)
const bitrateProgress = computed({
  get: () => Math.max(bitrateSpan.min, bitrateSliderProgress(bitrateValue.value)),
  set: (position) => { bitrateValue.value = Math.max(BITRATE_LIMITS.MIN, bitrateFromProgress(position)) },
})
const loading = ref(true)
const refreshing = ref(false)
const applying = ref(false)
const problem = ref('')

// 计算属性
const isLoading = computed(() => loading.value && !refreshing.value)

const selectedSession = computed(() => {
  if (!selectedClient.value) return null
  return activeSessions.value.find((s) => s.client_name === selectedClient.value) ?? null
})

const currentBitrate = computed(() => selectedSession.value?.bitrate ?? null)

// 工具函数
const formatBitrate = (kbps) => (kbps >= 1000 ? `${(kbps / 1000).toFixed(0)} Mbps` : `${kbps} Kbps`)

const isValidBitrate = (value) => value >= BITRATE_LIMITS.MIN && value <= BITRATE_LIMITS.MAX

// leading marks of the old messages (✅ ❌) are the tone now: a snackbar for what worked, a banner that stays for what failed
const plain = (text) => String(text).replace(/^[\u2705\u274c\u26a0\ufe0f\s]+/u, '')
const showMessage = (msg, type = 'info') => {
  if (type === 'error') problem.value = plain(msg)
  else { problem.value = ''; useNotify().snack(plain(msg)) }
}
const clientOptions = computed(() => activeSessions.value.map((s) => ({ value: s.client_name, label: s.client_name, text: `${s.width}x${s.height}@${s.fps}fps` })))
const inRange = computed(() => isValidBitrate(bitrateValue.value))
const rangeText = computed(() => plain(t.value.bitrateTool.rangeError.replace('{min}', formatBitrate(BITRATE_LIMITS.MIN)).replace('{max}', formatBitrate(BITRATE_LIMITS.MAX))))

const getSessionBitrate = (clientName) => {
  const session = activeSessions.value.find((s) => s.client_name === clientName)
  return session?.bitrate ?? DEFAULT_BITRATE
}

// 会话管理
const loadSessions = async (isRefresh = false) => {
  if (isRefresh) {
    refreshing.value = true
  } else {
    loading.value = true
  }
  problem.value = ''

  const previousClient = selectedClient.value

  try {
    const sessions = await sunshine.getActiveSessions()
    allSessions.value = sessions
    activeSessions.value = sessions.filter((s) => s.state !== 'STOPPED' && s.state !== 'STOPPING')

    if (activeSessions.value.length === 0) {
      selectedClient.value = ''
      return
    }

    // 恢复或选择客户端
    const clientExists = activeSessions.value.some((s) => s.client_name === previousClient)
    selectedClient.value = previousClient && clientExists ? previousClient : activeSessions.value[0].client_name

    // 设置当前客户端码率到滑块
    bitrateValue.value = getSessionBitrate(selectedClient.value)
  } catch (error) {
    console.error('获取活动会话失败:', error)
    showMessage(t.value.bitrateTool.loadFailed.replace('{error}', error), 'error')
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

const onClientChange = () => {
  if (selectedClient.value) {
    bitrateValue.value = getSessionBitrate(selectedClient.value)
  }
}

// 码率调整
const applyBitrate = async () => {
  if (!selectedClient.value) {
    showMessage(t.value.bitrateTool.selectFirstError, 'error')
    return
  }

  if (!isValidBitrate(bitrateValue.value)) {
    showMessage(
      t.value.bitrateTool.rangeError
        .replace('{min}', formatBitrate(BITRATE_LIMITS.MIN))
        .replace('{max}', formatBitrate(BITRATE_LIMITS.MAX)),
      'error'
    )
    return
  }

  applying.value = true
  problem.value = ''

  try {
    const result = await sunshine.changeBitrate(selectedClient.value, bitrateValue.value)
    showMessage(`✅ ${result}`, 'success')
    setTimeout(() => loadSessions(true), REFRESH_DELAY)
  } catch (error) {
    console.error('码率调整错误:', error)
    const errorMessage = error.toString()
    if (errorMessage.includes('身份验证') || errorMessage.includes('401')) {
      showMessage(t.value.bitrateTool.authFailed, 'error')
    } else {
      showMessage(t.value.bitrateTool.applyFailed.replace('{error}', error), 'error')
    }
  } finally {
    applying.value = false
  }
}

onMounted(loadSessions)
</script>

<style scoped>
.m-tool { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; width: 100%; padding: 4px 0; color: var(--md-sys-color-on-surface); }
.m-tool__head { display: flex; align-items: center; justify-content: space-between; gap: 12px; } .m-tool__head h2 { margin: 0; font: var(--m-title-large); }
.m-tool__wait { display: grid; justify-items: center; gap: 12px; padding: 32px; color: var(--md-sys-color-on-surface-variant); font: var(--m-body-large); }
.m-tool__hero span { font: var(--m-title-medium); }
.m-tool__hero { display: grid; justify-items: start; gap: 8px; padding: 24px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.m-tool__big { font: var(--m-display-medium); font-variant-numeric: tabular-nums; letter-spacing: 0; }
.m-tool__actions { display: flex; justify-content: flex-end; }
</style>
