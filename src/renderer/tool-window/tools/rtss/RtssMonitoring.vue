<template>
  <MSection>
    <MGroup>
      <MSwitchItem :model-value="active" icon="monitoring" :title="t.rtssTool.monitoring" @update:model-value="toggleMonitoring" />
      <MItem v-for="group in metricGroups" :key="group.id" stack :title="group.label">
        <template #control>
          <MChipSet multiple :model-value="config.metrics.filter((m) => group.metrics.some((g) => g.id === m))" :label="group.label" :options="group.metrics.map((m) => ({ value: m.id, label: m.label }))" @update:model-value="(ids) => setGroupMetrics(group, ids)" />
        </template>
      </MItem>
    </MGroup>
    <MNotice v-if="config.metrics.includes('pipeline_fps')" tone="info">{{ pipelineHint }}</MNotice>

    <!-- 样式配置 -->
    <MGroup>
      <MItem stack icon="edit" :title="t.rtssTool.headerText" :text="cjkHint"><template #control><MField v-model="config.header_text" :label="t.rtssTool.headerText" placeholder="☀ Foundation Sunshine" /></template></MItem>
      <MChoiceItem v-model="config.interval_ms" icon="timer" :title="t.rtssTool.updateInterval" :options="INTERVALS.map((v) => ({ value: v, label: `${v / 1000}s` }))" :cancel-label="t.common.cancel" @update:model-value="(v) => { config.interval_ms = Number(v) }" />
      <MItem stack icon="palette" :title="t.rtssTool.titleColor"><template #control><RtssColor v-model="config.title_color" :label="t.rtssTool.titleColor" /></template></MItem>
      <MItem stack icon="palette" :title="t.rtssTool.labelColor"><template #control><RtssColor v-model="config.label_color" :label="t.rtssTool.labelColor" /></template></MItem>
      <MItem stack icon="palette" :title="t.rtssTool.valueColor"><template #control><RtssColor v-model="config.value_color" :label="t.rtssTool.valueColor" /></template></MItem>
    </MGroup>

    <!-- OSD 预览 -->
    <figure v-if="active && snapshot.osd_text" class="m-rtss__preview">
      <figcaption>{{ t.rtssTool.osdPreview }}</figcaption>
      <pre>{{ snapshot.osd_text }}</pre>
    </figure>
  </MSection>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onUnmounted, watch } from 'vue'
import { MChipSet, MChoiceItem, MField, MGroup, MItem, MNotice, MSection, MSwitchItem } from 'foundation-ui/vue'
import RtssColor from './RtssColor.vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../../../desktop/i18n/index.js'

const { t, locale } = useI18n()
const emit = defineEmits(['message'])

const STORAGE_KEY = 'rtss-monitoring-config'

const DEFAULT_CONFIG = {
  interval_ms: 1000,
  metrics: ['session_state', 'stream_fps', 'stream_bitrate', 'pipeline_fps', 'host_perf_p95'],
  title_color: 'FFD700',
  label_color: 'AAAAAA',
  value_color: '00FF00',
  font_size: 0,
  header_text: '☀ Foundation Sunshine',
}

function loadPersistedConfig() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw)
      return { ...DEFAULT_CONFIG, ...parsed }
    }
  } catch {}
  return { ...DEFAULT_CONFIG }
}

const active = ref(false)
const snapshot = reactive({ active: false, osd_text: '', metrics: {} })
const config = reactive(loadPersistedConfig())

const availableMetrics = ref([])
const pipelineHint = computed(() => locale.value === 'zh'
  ? 'Src 源更新 · Cap 捕获交付 · Submit 编码提交 · Out 编码输出 · Repeat 重复提交（FPS）。与 RTSS 游戏帧率对照；仅在监控开启并选择该指标时启用本入口采集，重新开启需约 1 秒预热。'
  : 'Src source updates · Cap capture delivery · Submit encoder input · Out encoded output · Repeat repeated input (FPS). Compare with RTSS game FPS. This monitor collects only while enabled with this metric selected; allow about 1 second to warm up after enabling.')
const cjkHint = computed(() => t.value.rtssTool.cjkHint || '中文/CJK 需在 RTSS Setup 中启用“Vector 3D”并选择中文字体（如 Microsoft YaHei）')
let pollTimer = null

// 持久化 config 到 localStorage
watch(config, (v) => {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(v)) } catch {}
}, { deep: true })

const metricGroups = computed(() => {
  const isZh = locale.value === 'zh'
  const groups = [
    { id: 'session', label: isZh ? '串流会话' : 'Streaming', metrics: [] },
    { id: 'host_perf', label: isZh ? '主机性能' : 'Host Performance', metrics: [] },
    { id: 'process', label: isZh ? '进程性能' : 'Process', metrics: [] },
  ]
  for (const m of availableMetrics.value) {
    const grp = groups.find(g => g.id === m.group)
    if (grp) grp.metrics.push({ id: m.id, label: isZh ? m.label_zh : m.label_en })
  }
  return groups.filter(g => g.metrics.length > 0)
})

// ─── 方法 ───
const INTERVALS = [500, 1000, 2000, 5000]
// the chips of a group are one multi-select: the metrics of the other groups stay as they are
function setGroupMetrics(group, ids) {
  const keep = config.metrics.filter((m) => !group.metrics.some((g) => g.id === m))
  config.metrics.splice(0, config.metrics.length, ...keep, ...ids)
  if (active.value) startMonitoring()
}
function toggleMetric(id) {
  const idx = config.metrics.indexOf(id)
  if (idx >= 0) config.metrics.splice(idx, 1)
  else config.metrics.push(id)
  if (active.value) startMonitoring()
}

async function toggleMonitoring(val) {
  if (val) await startMonitoring()
  else await stopMonitoring()
}

async function startMonitoring() {
  try {
    // 深拷贝 config，避免 reactive Proxy 序列化问题
    const rawConfig = JSON.parse(JSON.stringify(config))
    await invoke('rtss_start_monitoring', { config: rawConfig })
    active.value = true
    startPoll()
    emit('message', t.value.rtssTool.monitoringStarted, 'success')
  } catch (e) {
    emit('message', String(e), 'error')
  }
}

async function stopMonitoring() {
  try {
    await invoke('rtss_stop_monitoring')
    active.value = false
    stopPoll()
    snapshot.active = false
    snapshot.osd_text = ''
    emit('message', t.value.rtssTool.monitoringStopped, 'success')
  } catch (e) {
    emit('message', String(e), 'error')
  }
}

function startPoll() {
  stopPoll()
  pollTimer = setInterval(async () => {
    try {
      const snap = await invoke('rtss_get_monitoring_status')
      Object.assign(snapshot, snap)
      if (!snap.active) { active.value = false; stopPoll() }
    } catch (e) { /* ignore */ }
  }, 2000)
}

function stopPoll() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
}

async function loadState() {
  try {
    availableMetrics.value = await invoke('rtss_get_available_metrics')
  } catch (e) { /* ignore */ }
  try {
    const snap = await invoke('rtss_get_monitoring_status')
    if (snap.active) {
      active.value = true
      Object.assign(snapshot, snap)
      // 同步后端运行中的 config（若存在）
      if (snap.config) {
        Object.assign(config, snap.config)
      }
      startPoll()
    }
  } catch (e) { /* ignore */ }
}

onMounted(() => loadState())
onUnmounted(() => stopPoll())

defineExpose({ loadState })
</script>

<style lang="less" scoped>
.frame-rate-hint {
  display: block;
  color: rgba(255, 255, 255, 0.6);
  line-height: 1.6;
}

.metric-groups {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.metric-group-label {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.55);
  margin-bottom: 3px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.metric-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.metric-chip {
  padding: 3px 10px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.75);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.2s;

  &:hover { background: rgba(255, 255, 255, 0.14); color: white; }
  &.active {
    background: rgba(96, 165, 250, 0.35);
    border-color: rgba(96, 165, 250, 0.7);
    color: #60a5fa;
    font-weight: 600;
    box-shadow: 0 0 6px rgba(96, 165, 250, 0.25);
  }
}

.monitor-style {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 4px;
}

.style-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;

  > label {
    color: rgba(255, 255, 255, 0.8);
    min-width: 65px;
    flex-shrink: 0;
  }
}

.style-input {
  flex: 1;
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  color: white;
  font-size: 12px;
  outline: none;

  &:focus { border-color: rgba(96, 165, 250, 0.6); box-shadow: 0 0 0 2px rgba(96, 165, 250, 0.15); }
  &::placeholder { color: rgba(255, 255, 255, 0.3); }
}

.colors-row {
  flex-wrap: wrap;
  gap: 10px;
}

.color-field {
  display: flex;
  align-items: center;
  gap: 5px;

  label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.7);
    min-width: unset;
  }

  input[type="color"] {
    width: 24px;
    height: 24px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    background: none;
    cursor: pointer;
    padding: 0;

    &::-webkit-color-swatch-wrapper { padding: 2px; }
    &::-webkit-color-swatch { border-radius: 4px; border: none; }
  }
}

.fmt-select {
  padding: 4px 8px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: rgba(255, 255, 255, 0.9);
  font-size: 12px;
  cursor: pointer;
  outline: none;

  &.wide { min-width: 80px; }
  option { background: #1e1e2e; color: white; }
}

.osd-preview {
  background: rgba(0, 0, 0, 0.45);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  padding: 8px 12px;
}

.osd-preview-label {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 4px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.osd-preview-text {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  color: #4ade80;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  line-height: 1.6;
}
</style>

<style scoped>
.m-rtss__preview { margin: 8px 0 0; padding: 12px 16px; border-radius: var(--md-sys-shape-corner-large); background: var(--md-sys-color-surface-container-high); }
.m-rtss__preview figcaption { font: var(--m-label-medium); color: var(--md-sys-color-on-surface-variant); }
.m-rtss__preview pre { margin: 4px 0 0; font: var(--m-body-medium); font-family: var(--m-font-mono); white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
