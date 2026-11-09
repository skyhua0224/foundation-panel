<template>
  <div ref="containerRef" class="m-perf" :class="{ 'is-embedded': embedded, 'is-expanded': isExpanded }">
    <header class="m-perf__head" v-bind="dragRegionAttrs">
      <div class="m-perf__session" v-bind="dragRegionAttrs">
        <MChip :tone="statusTone">{{ statusText }}</MChip>
        <b data-redact="client">{{ sessionTitle }}</b>
        <span data-redact="address">{{ sessionMeta }}</span>
      </div>
      <div class="m-perf__actions">
        <MIconButton icon="refresh" :label="t.performanceTool.refresh" :disabled="refreshing" @click="loadSnapshot(true)" />
        <MIconButton v-if="!embedded" :icon="isExpanded ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" :label="expandTitle" @click="isExpanded = !isExpanded" />
        <MIconButton v-if="!embedded" icon="close" :label="t.performanceTool.close" @click="$emit('close')" />
      </div>
    </header>

    <div v-if="loading" class="m-perf__wait"><MLoading size="32px" /><span>{{ t.performanceTool.loading }}</span></div>
    <MBanner v-else-if="error" tone="error" icon="error">{{ t.performanceTool.loadFailed.replace('{error}', error) }}</MBanner>
    <MEmpty v-else-if="!currentSession" icon="monitor" :title="t.performanceTool.noStream" />

    <template v-else>
      <div class="m-perf__hud" role="group" :aria-label="t.performanceTool.hostLatency">
        <div class="m-perf__metric is-primary"><span>{{ t.performanceTool.p95 }}</span><b>{{ formatMs(hostLatency.p95_ms) }}</b></div>
        <div class="m-perf__metric"><span>{{ t.performanceTool.recentFps }}</span><b>{{ formatNumber(hostLatency.recent_fps) }}</b></div>
        <div class="m-perf__metric" :class="`is-${statusTone}`"><span>{{ t.performanceTool.budgetUsage }}</span><b>{{ formatPercent(budgetUsage) }}</b></div>
      </div>

      <figure class="m-perf__chart" :class="{ 'is-compact': !isExpanded && !embedded }">
        <figcaption><span>{{ t.performanceTool.hostLatency }}</span><span>{{ t.performanceTool.samples.replace('{count}', hostLatency.samples ?? 0) }}</span></figcaption>
        <svg viewBox="0 0 320 74" preserveAspectRatio="none" aria-hidden="true">
          <line x1="0" y1="56" x2="320" y2="56" class="m-perf__grid" />
          <polyline v-if="sparklinePoints" :points="sparklinePoints" class="m-perf__line" />
        </svg>
      </figure>

      <div v-if="isExpanded || embedded" class="m-perf__details">
        <MSection :title="t.performanceTool.windowStats">
          <dl class="m-perf__grid-list"><div v-for="item in detailStats" :key="item.label"><dt>{{ item.label }}</dt><dd>{{ item.value }}</dd></div></dl>
        </MSection>
        <MSection v-if="pipelineSegments.length" :title="t.performanceTool.pipelineStats">
          <MGroup>
            <MItem v-for="segment in pipelineSegments" :key="segment.key" :title="segment.label" :text="`${t.performanceTool.avgShort} ${formatMs(segment.stats.avg_ms)} / ${t.performanceTool.lastShort} ${formatMs(segment.stats.last_ms)}`" :value="formatMs(segment.stats.p95_ms)" />
          </MGroup>
        </MSection>
        <MSection :title="t.performanceTool.sessionStats">
          <MGroup>
            <MItem icon="memory" :title="t.performanceTool.encoderCapture" :value="`${currentSession.encoder || 'auto'} / ${currentSession.capture || 'auto'}`" />
            <MItem icon="speed" :title="t.performanceTool.bitrate" :value="formatBitrate(currentSession.bitrate_kbps)" />
            <MItem icon="timer" :title="t.performanceTool.uptime" :value="formatUptime(currentSession.uptime_ms)" />
          </MGroup>
        </MSection>
        <div class="m-perf__stop">
          <MButton :variant="stopArmed ? 'danger' : 'tonal'" icon="stop_circle" :loading="stopBusy" :disabled="stopBusy" @click="requestStopAll">{{ stopArmed ? t.performanceTool.confirmStopAll : t.performanceTool.stopAllSessions }}</MButton>
          <span v-if="stopMessage" class="m-perf__note" role="status">{{ stopMessage }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { useTouchWindowDrag } from '../../composables/useTouchWindowDrag.js'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { MBanner, MButton, MChip, MEmpty, MGroup, MIconButton, MItem, MLoading, MSection } from 'foundation-ui/vue'
import { sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { useAdaptiveWindowSize } from '../../composables/useAdaptiveWindowSize.js'

const { t } = useI18n()

const props = defineProps({
  embedded: {
    type: Boolean,
    default: false,
  },
})

defineEmits(['close'])

const POLL_INTERVAL_MS = 1000
const SPARKLINE_SAMPLE_LIMIT = 80
const SPARKLINE_SMOOTHING_ALPHA = 0.42
const SPARKLINE_CEILING_RISE_WEIGHT = 0.65
const SPARKLINE_CEILING_FALL_WEIGHT = 0.08

const snapshot = ref(null)
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const isExpanded = ref(props.embedded)
const containerRef = ref(null)
const sparklineCeilingMs = ref(16)

let pollTimer = null
let snapshotInFlight = false
let queuedManualRefresh = false
let isDisposed = false

const activeSessions = computed(() => snapshot.value?.sessions?.filter((session) => session.active) ?? [])

const currentSession = computed(() => {
  const sessions = activeSessions.value
  if (sessions.length === 0) return null

  const latestSessionId = snapshot.value?.latest_session_id
  return sessions.find((session) => session.session_id === latestSessionId) ?? sessions[0]
})

const hostLatency = computed(() => currentSession.value?.host_latency ?? {})
const pipeline = computed(() => currentSession.value?.pipeline ?? {})

// 断开所有会话:两段式确认(第一次点击只上膛,3 秒自动解除)
const stopArmed = ref(false)
const stopBusy = ref(false)
const stopMessage = ref('')
let stopArmTimer = null
let stopMessageTimer = null

async function requestStopAll() {
  if (!stopArmed.value) {
    stopArmed.value = true
    clearTimeout(stopArmTimer)
    stopArmTimer = setTimeout(() => {
      stopArmed.value = false
    }, 3000)
    return
  }
  stopArmed.value = false
  clearTimeout(stopArmTimer)
  stopBusy.value = true
  try {
    await sunshine.stopAllSessions()
    stopMessage.value = t.value.performanceTool.stopAllOk
  } catch (e) {
    stopMessage.value = t.value.performanceTool.stopAllFailed.replace('{error}', String(e))
  } finally {
    stopBusy.value = false
    clearTimeout(stopMessageTimer)
    stopMessageTimer = setTimeout(() => {
      stopMessage.value = ''
    }, 3000)
  }
}

const frameBudgetMs = computed(() => {
  const fps = currentSession.value?.fps || 60
  return 1000 / fps
})

const budgetUsage = computed(() => {
  const p95 = hostLatency.value.p95_ms
  if (p95 == null) return null
  return (p95 / frameBudgetMs.value) * 100
})

const statusPressure = computed(() => {
  if (budgetUsage.value == null) return null

  const average = hostLatency.value.avg_ms
  const averageUsage = average == null ? 0 : (average / frameBudgetMs.value) * 100
  return Math.max(averageUsage, budgetUsage.value * 0.7)
})

const statusClass = computed(() => {
  if (error.value) return 'error'
  if (!currentSession.value) return 'idle'
  if (!hostLatency.value.samples) return 'waiting'
  if (statusPressure.value == null) return 'waiting'
  if (statusPressure.value <= 90) return 'good'
  if (statusPressure.value <= 170) return 'warning'
  return 'error'
})

const statusTone = computed(() => ({ good: 'ok', warning: 'warn', error: 'error' })[statusClass.value] || 'neutral')

const statusText = computed(() => {
  if (error.value) return t.value.performanceTool.status.error
  if (!currentSession.value) return t.value.performanceTool.status.idle
  if (!hostLatency.value.samples) return t.value.performanceTool.status.waiting
  if (statusClass.value === 'good') return t.value.performanceTool.status.good
  if (statusClass.value === 'warning') return t.value.performanceTool.status.warning
  return t.value.performanceTool.status.slow
})

const touchDrag = useTouchWindowDrag(null, { restoreMaximized: false })
const dragRegionAttrs = computed(() => props.embedded ? {} : {
  'data-tauri-drag-region': '', onPointerdown: touchDrag.onTouchWindowDragStart,
})
const expandTitle = computed(() => isExpanded.value ? t.value.performanceTool.collapse : t.value.performanceTool.expand)
const sessionTitle = computed(() => currentSession.value?.client_name || t.value.performanceTool.noClient)

const sessionMeta = computed(() => {
  const session = currentSession.value
  if (!session) return t.value.performanceTool.noSessionMeta
  return `${session.width}x${session.height}@${session.fps} FPS`
})

const detailStats = computed(() => [
  { label: t.value.performanceTool.last, value: formatMs(hostLatency.value.last_ms) },
  { label: t.value.performanceTool.min, value: formatMs(hostLatency.value.min_ms) },
  { label: t.value.performanceTool.average, value: formatMs(hostLatency.value.avg_ms) },
  { label: t.value.performanceTool.max, value: formatMs(hostLatency.value.max_ms) },
  { label: t.value.performanceTool.frameBudget, value: formatMs(frameBudgetMs.value) },
  { label: t.value.performanceTool.sampleAge, value: formatAge(hostLatency.value.last_sample_age_ms) },
  { label: t.value.performanceTool.totalFrames, value: formatInteger(hostLatency.value.frames_total) },
  { label: t.value.performanceTool.totalSamples, value: formatInteger(hostLatency.value.total_samples) },
])

const pipelineSegments = computed(() => [
  { key: 'capture_to_convert', label: t.value.performanceTool.captureWait },
  { key: 'convert', label: t.value.performanceTool.convert },
  { key: 'encode_queue', label: t.value.performanceTool.encodeQueue },
  { key: 'encode', label: t.value.performanceTool.encode },
  { key: 'packet_to_broadcast', label: t.value.performanceTool.broadcastQueue },
  { key: 'total', label: t.value.performanceTool.totalPipeline },
].map((segment) => ({
  ...segment,
  stats: pipeline.value?.[segment.key] ?? {},
})).filter((segment) => segment.stats.samples))

const smoothSparklineValues = (values) => {
  if (values.length < 2) return values

  let smoothed = values[0]
  return values.map((value, index) => {
    if (index === 0) return value
    smoothed += (value - smoothed) * SPARKLINE_SMOOTHING_ALPHA
    return smoothed
  })
}

const updateSparklineCeiling = () => {
  const series = hostLatency.value.series_ms ?? []
  const values = series.slice(-SPARKLINE_SAMPLE_LIMIT)
  const target = Math.max(
    ...values,
    hostLatency.value.p95_ms ?? 0,
    frameBudgetMs.value,
    1,
  ) * 1.18
  const current = sparklineCeilingMs.value || target
  const weight = target > current ? SPARKLINE_CEILING_RISE_WEIGHT : SPARKLINE_CEILING_FALL_WEIGHT
  sparklineCeilingMs.value = current + (target - current) * weight
}

const sparklinePoints = computed(() => {
  const series = hostLatency.value.series_ms ?? []
  if (series.length < 2) return ''

  const values = smoothSparklineValues(series.slice(-SPARKLINE_SAMPLE_LIMIT))
  const max = Math.max(sparklineCeilingMs.value, 1)

  return values.map((value, index) => {
    const x = (index / (values.length - 1)) * 320
    const normalized = Math.min(Math.max(value / max, 0), 1)
    const y = 64 - normalized * 48
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
})

const formatMs = (value) => value == null ? '--' : `${Number(value).toFixed(2)} ms`
const formatNumber = (value) => value == null ? '--' : Number(value).toFixed(1)
const formatPercent = (value) => value == null ? '--' : `${Math.round(value)}%`
const formatInteger = (value) => value == null ? '--' : Number(value).toLocaleString()
const formatAge = (value) => value == null ? '--' : `${Math.round(value)} ms`
const formatBitrate = (kbps) => !kbps ? '--' : `${Math.round(kbps / 1000)} Mbps`

const formatUptime = (ms) => {
  if (!ms) return '--'
  const totalSeconds = Math.floor(ms / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}m ${seconds}s`
}

const buildApiUrl = (baseUrl) => {
  const normalized = baseUrl.endsWith('/') ? baseUrl.slice(0, -1) : baseUrl
  return `${normalized}/api/perf/current`
}

const fetchSnapshot = async () => {
  const proxyUrl = await sunshine.getProxyUrl()
  const response = await fetch(buildApiUrl(proxyUrl), {
    signal: AbortSignal.timeout(3000),
  })

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }

  return response.json()
}

const loadSnapshot = async (manual = false) => {
  if (snapshotInFlight) {
    if (manual) {
      queuedManualRefresh = true
      refreshing.value = true
    }
    return
  }

  snapshotInFlight = true
  if (manual) {
    refreshing.value = true
  }
  error.value = ''

  try {
    const nextSnapshot = await fetchSnapshot()
    if (!isDisposed) {
      snapshot.value = nextSnapshot
    }
  } catch (err) {
    if (!isDisposed) {
      error.value = err?.message || String(err)
    }
  } finally {
    snapshotInFlight = false
    if (!isDisposed) {
      loading.value = false
    }

    if (queuedManualRefresh && !isDisposed) {
      queuedManualRefresh = false
      loadSnapshot(true)
      return
    }

    queuedManualRefresh = false
    if (!isDisposed) {
      refreshing.value = false
    }
  }
}

const adaptiveWidth = computed(() => isExpanded.value ? 430 : 340)
const { scheduleSyncWindowSize } = useAdaptiveWindowSize(containerRef, {
  enabled: computed(() => !props.embedded),
  width: adaptiveWidth,
  isDragging: () => touchDrag.active,
  minHeight: 220,
  animate: true,
})

onMounted(() => {
  loadSnapshot()
  pollTimer = window.setInterval(() => loadSnapshot(), POLL_INTERVAL_MS)
})

watch(isExpanded, scheduleSyncWindowSize)
watch(currentSession, scheduleSyncWindowSize)
watch(hostLatency, updateSparklineCeiling, { immediate: true })
watch(frameBudgetMs, updateSparklineCeiling)
watch(pipelineSegments, scheduleSyncWindowSize)

onUnmounted(() => {
  isDisposed = true
  if (pollTimer) {
    window.clearInterval(pollTimer)
    pollTimer = null
  }
  clearTimeout(stopArmTimer)
  clearTimeout(stopMessageTimer)
})
</script>

<style scoped>
.m-perf { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; width: 100%; box-sizing: border-box; padding: 12px; color: var(--md-sys-color-on-surface); background: var(--md-sys-color-surface-container); border-radius: var(--md-sys-shape-corner-extra-large); }
.m-perf:not(.is-embedded) { width: 100vw; }
.m-perf.is-embedded { padding: 0; background: none; }
.m-perf__head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.m-perf__session { display: grid; gap: 2px; min-width: 0; justify-items: start; } .m-perf__session b { font: var(--m-title-medium); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; } .m-perf__session span { font: var(--m-body-small); color: var(--md-sys-color-on-surface-variant); }
.m-perf__actions { display: flex; flex: none; }
.m-perf__wait { display: flex; align-items: center; gap: 12px; padding: 16px; color: var(--md-sys-color-on-surface-variant); font: var(--m-body-large); }
.m-perf__hud { display: grid; grid-template-columns: 1.4fr 1fr 1fr; gap: 6px; }
.m-perf__metric { display: grid; gap: 2px; padding: 12px 16px; border-radius: var(--md-sys-shape-corner-large); background: var(--md-sys-color-surface-container-high); } .m-perf__metric span { font: var(--m-label-medium); color: var(--md-sys-color-on-surface-variant); } .m-perf__metric b { font: var(--m-headline-small); font-variant-numeric: tabular-nums; }
.m-perf__metric.is-primary { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); } .m-perf__metric.is-primary span { color: inherit; }
.m-perf__metric.is-warn { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); } .m-perf__metric.is-error { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.m-perf__metric.is-warn span, .m-perf__metric.is-error span { color: inherit; }
.m-perf__chart { margin: 0; padding: 12px 16px; border-radius: var(--md-sys-shape-corner-large); background: var(--md-sys-color-surface-container-high); } .m-perf__chart figcaption { display: flex; justify-content: space-between; font: var(--m-label-medium); color: var(--md-sys-color-on-surface-variant); }
.m-perf__chart svg { display: block; width: 100%; height: 74px; } .m-perf__chart.is-compact svg { height: 52px; }
.m-perf__grid { stroke: var(--md-sys-color-outline-variant); stroke-width: 1; stroke-dasharray: 4 4; vector-effect: non-scaling-stroke; }
.m-perf__line { fill: none; stroke: var(--md-sys-color-primary); stroke-width: 2.5; stroke-linejoin: round; stroke-linecap: round; vector-effect: non-scaling-stroke; }
.m-perf__details { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; }
.m-perf__grid-list { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 6px; margin: 0; } .m-perf__grid-list div { display: grid; padding: 8px 12px; border-radius: var(--md-sys-shape-corner-medium); background: var(--md-sys-color-surface-container-high); } .m-perf__grid-list dt { font: var(--m-label-small); color: var(--md-sys-color-on-surface-variant); } .m-perf__grid-list dd { margin: 0; font: var(--m-title-small); font-variant-numeric: tabular-nums; }
.m-perf__stop { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; } .m-perf__note { font: var(--m-body-medium); color: var(--md-sys-color-on-surface-variant); }
@media (max-width: 380px) { .m-perf__hud { grid-template-columns: 1fr 1fr; } .m-perf__metric.is-primary { grid-column: 1 / -1; } }
</style>
