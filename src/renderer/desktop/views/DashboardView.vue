<template>
  <div class="dashboard-view">
    <MHero class="dash-hero fade-in" :tone="heroTone" icon="dashboard" title="Foundation Desktop" :text="heroText" />

    <!-- 状态 -->
    <div class="desktop-grid cols-4 stat-row">
      <div v-for="(stat, i) in stats" :key="stat.id" class="desktop-card stat-card fade-in" :class="[stat.tone, 'delay-' + (i + 1)]" role="status">
        <span class="stat-icon"><MIcon :name="stat.icon" /></span>
        <span class="stat-info">
          <span class="stat-label">{{ stat.label }}</span>
          <span class="stat-big">{{ stat.value }}</span>
        </span>
      </div>
    </div>

    <!-- 快捷操作 -->
    <h2 class="section-title fade-in"><MIcon name="bolt" />{{ t.dashboard.quickActions }}</h2>

    <div class="desktop-grid cols-4">
      <div
        v-for="action in actions"
        :key="action.id"
        class="desktop-card action-card fade-in"
        :class="action.tone"
        role="button"
        tabindex="0"
        :data-focus-key="'dash-' + action.id"
        @click="action.run()"
        @keydown.enter="action.run()"
      >
        <span class="action-icon"><MIcon :name="action.icon" /></span>
        <span class="action-info">
          <span class="action-title">{{ action.title }}</span>
          <span class="action-desc">{{ action.desc }}</span>
        </span>
        <MIcon class="action-arrow" name="arrow_forward" />
      </div>
    </div>

    <!-- 系统信息 -->
    <h2 class="section-title fade-in"><MIcon name="monitor" />{{ t.dashboard.systemInfo }}</h2>

    <div class="desktop-card system-info-card fade-in">
      <div class="info-grid">
        <div class="info-item">
          <span class="info-icon"><MIcon name="light_mode" fill /></span>
          <span class="info-text"><span class="info-label">{{ t.dashboard.sunshineVersion }}</span><span class="info-value">{{ systemInfo.sunshineVersion }}</span></span>
        </div>
        <div class="info-item">
          <span class="info-icon"><MIcon name="desktop_windows" fill /></span>
          <span class="info-text"><span class="info-label">{{ t.dashboard.os }}</span><span class="info-value">{{ systemInfo.os }}</span></span>
        </div>
        <div class="info-item">
          <span class="info-icon"><MIcon name="memory" fill /></span>
          <span class="info-text"><span class="info-label">{{ t.dashboard.gpu }}</span><span class="info-value">{{ systemInfo.gpu }}</span></span>
        </div>
        <div class="info-item">
          <span class="info-icon"><MIcon name="videocam" fill /></span>
          <span class="info-text"><span class="info-label">{{ t.dashboard.encoder }}</span><span class="info-value">{{ systemInfo.encoder }}</span></span>
        </div>
      </div>
    </div>

    <!-- 内存监控 -->
    <h2 class="section-title fade-in"><MIcon name="monitor_heart" />{{ t.dashboard.memoryMonitor }}</h2>

    <div class="desktop-card memory-monitor-card fade-in">
      <div class="memory-overview">
        <div class="memory-stat">
          <span class="memory-stat-label">{{ t.dashboard.totalWorkingSet }}</span>
          <span class="memory-stat-value highlight">{{ formatBytes(memoryInfo.total_working_set) }}</span>
        </div>
        <div class="memory-stat">
          <span class="memory-stat-label">{{ t.dashboard.totalPrivateWorkingSet }}</span>
          <span class="memory-stat-value">{{ formatBytes(memoryInfo.total_private_working_set) }}</span>
        </div>
        <div class="memory-stat">
          <span class="memory-stat-label">{{ t.dashboard.systemMemory }}</span>
          <span class="memory-stat-value">{{ formatBytes(memoryInfo.system_available_memory) }} / {{ formatBytes(memoryInfo.system_total_memory) }}</span>
        </div>
      </div>

      <!-- before the first reading of the host: the bar and the process rows as bars at their size (the empty text below would be wrong, and would turn into the table) -->
      <div v-if="!memoryLoaded" class="mem-skel" role="status" aria-busy="true"><i class="mem-skel__bar" /><i class="mem-skel__line" /><i class="mem-skel__line" /><i class="mem-skel__line" /></div>
      <div class="memory-bar-container" v-else-if="memoryInfo.system_total_memory > 0">
        <div class="memory-bar" role="img" :aria-label="`Sunshine ${sunshineMemPercent}%, ${t.dashboard.systemUsed} ${systemUsedPercent}%, ${t.dashboard.free} ${freeMemPercent}%`">
          <div class="memory-bar-fill sunshine-fill" :style="{ width: sunshineMemPercent + '%' }"></div>
          <div class="memory-bar-fill system-fill" :style="{ width: systemUsedPercent + '%', left: sunshineMemPercent + '%' }"></div>
        </div>
        <div class="memory-bar-legend">
          <span class="legend-item"><span class="legend-dot sunshine"></span>Sunshine {{ sunshineMemPercent }}%</span>
          <span class="legend-item"><span class="legend-dot system"></span>{{ t.dashboard.systemUsed }} {{ systemUsedPercent }}%</span>
          <span class="legend-item"><span class="legend-dot free"></span>{{ t.dashboard.free }} {{ freeMemPercent }}%</span>
        </div>
      </div>

      <div class="process-list" v-if="memoryLoaded && memoryInfo.processes.length > 0">
        <div class="process-row header">
          <span class="process-name">{{ t.dashboard.processName }}</span>
          <span class="process-pid">PID</span>
          <span class="process-mem">{{ t.dashboard.workingSet }}</span>
          <span class="process-mem">{{ t.dashboard.peakWorkingSet }}</span>
          <span class="process-mem">{{ t.dashboard.privateWS }}</span>
        </div>
        <div class="process-row" v-for="proc in memoryInfo.processes" :key="proc.pid">
          <span class="process-name">{{ proc.process_name }}</span>
          <span class="process-pid">{{ proc.pid }}</span>
          <span class="process-mem">{{ formatBytes(proc.working_set) }}</span>
          <span class="process-mem peak">{{ formatBytes(proc.peak_working_set) }}</span>
          <span class="process-mem">{{ formatBytes(proc.private_working_set) }}</span>
        </div>
      </div>
      <MBanner v-else-if="memoryInfo.error" tone="error" icon="warning">{{ memoryInfo.error }}</MBanner>
      <div class="no-process" v-else-if="memoryLoaded">{{ t.dashboard.noSunshineProcess }}</div>

      <div class="memory-trend" v-if="memoryHistory.length > 1">
        <div class="trend-title">{{ t.dashboard.memoryTrend }}</div>
        <div class="trend-chart">
          <svg :viewBox="'0 0 ' + trendWidth + ' ' + trendHeight" class="trend-svg" role="img" :aria-label="t.dashboard.memoryTrend">
            <line v-for="i in 4" :key="'grid-' + i"
                  :x1="0" :y1="trendHeight * i / 4"
                  :x2="trendWidth" :y2="trendHeight * i / 4"
                  class="grid-line"/>
            <polyline :points="trendPolyline" class="trend-line working-set" fill="none"/>
            <polyline :points="trendPrivatePolyline" class="trend-line private-ws" fill="none"/>
          </svg>
          <div class="trend-legend">
            <span class="legend-item"><span class="legend-line ws"></span>{{ t.dashboard.workingSet }}</span>
            <span class="legend-item"><span class="legend-line pws"></span>{{ t.dashboard.privateWS }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { MBanner, MHero, MIcon } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

// Tauri 命令 - 使用 ref 存储
const invoke = ref(null)
const proxyUrl = ref('http://localhost:48081')

// 状态数据
const serviceStatus = ref({ text: '', class: 'connecting' })
const statusTone = computed(() => ({ online: 'ok', offline: 'error', connecting: 'warn' }[serviceStatus.value.class] || ''))
// the hero says how the host is, in the colour of that: online green, connecting amber, otherwise the primary tone
const HERO_TONES = { online: 'success', connecting: 'tertiary' }
const heroTone = computed(() => HERO_TONES[serviceStatus.value.class] || 'primary')
const heroText = computed(() => [serviceStatus.value.text, uptime.value !== '--:--:--' ? uptime.value : ''].filter(Boolean).join(' · '))
const pairedDevices = ref(0)
const activeSessions = ref(0)
const uptime = ref('--:--:--')

const systemInfo = ref({
  sunshineVersion: '...',
  os: 'Windows',
  gpu: '...',
  encoder: '...'
})

// 内存监控数据
// false until the first reading (or the first failure) is in: the card draws bars, not 「no process」
const memoryLoaded = ref(false)
const memoryInfo = ref({
  processes: [],
  total_working_set: 0,
  total_private_working_set: 0,
  system_total_memory: 0,
  system_available_memory: 0,
  timestamp: '',
})

// 内存历史记录（最近60个采样点）
const memoryHistory = ref([])
const MAX_HISTORY = 60

// 趋势图尺寸
const trendWidth = 600
const trendHeight = 120

// 计算属性：内存百分比
const sunshineMemPercent = computed(() => {
  if (memoryInfo.value.system_total_memory === 0) return 0
  return Math.round(memoryInfo.value.total_working_set / memoryInfo.value.system_total_memory * 100)
})

const systemUsedPercent = computed(() => {
  if (memoryInfo.value.system_total_memory === 0) return 0
  const systemUsed = memoryInfo.value.system_total_memory - memoryInfo.value.system_available_memory
  const nonSunshine = systemUsed - memoryInfo.value.total_working_set
  return Math.max(0, Math.round(nonSunshine / memoryInfo.value.system_total_memory * 100))
})

const freeMemPercent = computed(() => {
  return Math.max(0, 100 - sunshineMemPercent.value - systemUsedPercent.value)
})

// 计算属性：趋势图折线
const trendPolyline = computed(() => {
  const history = memoryHistory.value
  if (history.length < 2) return ''
  const maxVal = Math.max(...history.map(h => h.working_set), 1)
  return history.map((h, i) => {
    const x = (i / (MAX_HISTORY - 1)) * trendWidth
    const y = trendHeight - (h.working_set / maxVal) * (trendHeight - 10) - 5
    return `${x},${y}`
  }).join(' ')
})

const trendPrivatePolyline = computed(() => {
  const history = memoryHistory.value
  if (history.length < 2) return ''
  const maxVal = Math.max(...history.map(h => h.working_set), 1)
  return history.map((h, i) => {
    const x = (i / (MAX_HISTORY - 1)) * trendWidth
    const y = trendHeight - (h.private_ws / maxVal) * (trendHeight - 10) - 5
    return `${x},${y}`
  }).join(' ')
})

// 格式化字节
function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let val = bytes
  while (val >= 1024 && i < units.length - 1) {
    val /= 1024
    i++
  }
  return `${val.toFixed(1)} ${units[i]}`
}

// 计时器
let uptimeInterval = null
let memoryInterval = null
// Sunshine 进程启动时间（Unix ms），0 表示未获取到
const sunshineStartTimeMs = ref(0)

function updateUptime() {
  if (sunshineStartTimeMs.value === 0) {
    uptime.value = '--:--:--'
    return
  }
  const elapsed = Math.floor((Date.now() - sunshineStartTimeMs.value) / 1000)
  if (elapsed < 0) {
    uptime.value = '--:--:--'
    return
  }
  const hours = Math.floor(elapsed / 3600).toString().padStart(2, '0')
  const minutes = Math.floor((elapsed % 3600) / 60).toString().padStart(2, '0')
  const seconds = (elapsed % 60).toString().padStart(2, '0')
  uptime.value = `${hours}:${minutes}:${seconds}`
}

async function loadSystemInfo() {
  if (!invoke.value) return

  try {
    // 获取 Sunshine 版本
    const version = await invoke.value('get_sunshine_version')
    systemInfo.value.sunshineVersion = version || t.value.dashboard.status.unknown

    // 获取 GPU 信息（get_gpus 返回 Vec<String>，即 GPU 名称列表）
    const gpus = await invoke.value('get_gpus')
    if (gpus && gpus.length > 0) {
      systemInfo.value.gpu = gpus[0]
      // 根据 GPU 名称推断编码器
      const gpuLower = gpus[0].toLowerCase()
      systemInfo.value.encoder = gpuLower.includes('nvidia') || gpuLower.includes('geforce') ? 'NVENC' :
                                  gpuLower.includes('amd') || gpuLower.includes('radeon') ? 'AMF' :
                                  gpuLower.includes('intel') ? 'QuickSync' : 'Software'
    }

    // 获取活动会话
    const sessions = await invoke.value('get_active_sessions')
    activeSessions.value = sessions?.length || 0

    // 获取 Sunshine 进程启动时间
    try {
      const startInfo = await invoke.value('get_sunshine_start_time')
      console.log('Sunshine start time info:', startInfo)
      if (startInfo && startInfo.start_time_ms > 0) {
        sunshineStartTimeMs.value = startInfo.start_time_ms
        updateUptime()
      }
    } catch (e) {
      console.error('Failed to get sunshine start time:', e)
    }

    // 从 API 获取已配对设备数量
    try {
      const resp = await fetch(`${proxyUrl.value}/api/clients/list`)
      const data = await resp.json()
      if (data.status?.toString() === 'true' && data.named_certs) {
        pairedDevices.value = data.named_certs.length
      }
    } catch (_) { /* proxy not available */ }

    serviceStatus.value = { text: t.value.dashboard.status.online, class: 'online' }
  } catch (e) {
    console.error('Failed to load system info:', e)
    serviceStatus.value = { text: t.value.dashboard.status.offline, class: 'offline' }
  }
}

// 操作函数
async function openWebUI() {
  if (invoke.value) {
    try {
      const url = await invoke.value('get_sunshine_url')
      await invoke.value('open_external_url', { url })
    } catch (e) {
      console.error('Failed to open web UI:', e)
    }
  }
}

async function restartService() {
  if (invoke.value) {
    try {
      await invoke.value('restart_sunshine_service')
      serviceStatus.value = { text: t.value.dashboard.status.restarting, class: 'connecting' }
      setTimeout(() => {
        serviceStatus.value = { text: t.value.dashboard.status.online, class: 'online' }
      }, 3000)
    } catch (e) {
      console.error('Failed to restart service:', e)
    }
  }
}

async function openLogs() {
  if (invoke.value) {
    try {
      await invoke.value('open_tool_window', { toolName: 'logs' })
    } catch (e) {
      console.error('Failed to open logs window:', e)
    }
  }
}

async function openHostPerformance() {
  if (invoke.value) {
    try {
      await invoke.value('open_tool_window', { toolName: 'performance' })
    } catch (e) {
      console.error('Failed to open host performance monitor:', e)
    }
  }
}

// 状态卡：一个数据表驱动四张
const stats = computed(() => [
  { id: 'service', icon: 'sensors', label: t.value.dashboard.serviceStatus, value: serviceStatus.value.text || '…', tone: statusTone.value },
  { id: 'paired', icon: 'devices', label: t.value.dashboard.pairedDevices, value: pairedDevices.value, tone: '' },
  { id: 'sessions', icon: 'cast_connected', label: t.value.dashboard.activeSessions, value: activeSessions.value, tone: '' },
  { id: 'uptime', icon: 'schedule', label: t.value.dashboard.uptime, value: uptime.value, tone: '' },
])

// 快捷操作：一个数据表驱动四张卡
const actions = computed(() => [
  { id: 'web', icon: 'public', title: t.value.dashboard.webConsole, desc: t.value.dashboard.webConsoleDesc, run: openWebUI },
  { id: 'restart', icon: 'restart_alt', title: t.value.dashboard.restartService, desc: t.value.dashboard.restartServiceDesc, run: restartService, tone: 'warning' },
  { id: 'perf', icon: 'speed', title: t.value.dashboard.hostPerformance, desc: t.value.dashboard.hostPerformanceDesc, run: openHostPerformance },
  { id: 'logs', icon: 'description', title: t.value.dashboard.viewLogs, desc: t.value.dashboard.viewLogsDesc, run: openLogs },
])

async function loadMemoryInfo() {
  if (!invoke.value) { memoryLoaded.value = true; return }
  try {
    const data = await invoke.value('get_process_memory_info')
    memoryInfo.value = data

    // 记录历史
    memoryHistory.value.push({
      working_set: data.total_working_set,
      private_ws: data.total_private_working_set,
      time: Date.now(),
    })
    if (memoryHistory.value.length > MAX_HISTORY) {
      memoryHistory.value.shift()
    }
  } catch (e) {
    console.error('Failed to load memory info:', e)
    // 如果命令不存在（旧版后端），标记错误
    memoryInfo.value.error = String(e)
  } finally {
    memoryLoaded.value = true
  }
}

onMounted(async () => {
  // 动态导入 Tauri API
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
    const url = await invoke.value('get_proxy_url_command')
    if (url) proxyUrl.value = url
  } catch (e) {
    console.log('Tauri invoke not available:', e)
  }

  updateUptime()
  uptimeInterval = setInterval(updateUptime, 1000)
  loadSystemInfo()

  // 内存监控：每10秒采样一次
  loadMemoryInfo()
  memoryInterval = setInterval(loadMemoryInfo, 10000)
})

onUnmounted(() => {
  if (uptimeInterval) {
    clearInterval(uptimeInterval)
  }
  if (memoryInterval) {
    clearInterval(memoryInterval)
  }
})
</script>
<style lang="less" scoped>
.dashboard-view {
  max-width: 1500px;
  margin: 0 auto;
}

// the hero band, the same one the other pages open with: a burst shape with the page's icon, the name, and how the host is right now
.dash-hero {
  margin-bottom: 28px;
  padding: 30px 40px;
  gap: 28px;

  :deep(.m-hero__shape) { width: 120px; height: 120px; font-size: 56px; }
  :deep(.m-hero__title) { font: var(--m-display-medium); font-weight: 700; letter-spacing: -0.01em; color: inherit; }
  :deep(.m-hero__text) { font: var(--m-headline-small); color: inherit; }
}

.stat-row {
  margin-bottom: 8px;
}

// 状态卡：大数字，图标在 tone 色的圆角块里；服务状态按 ok / warn / error 换底色
.stat-card {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 22px 24px;
  min-height: 112px;
  border: 0;
  background: var(--md-sys-color-surface-container-high);

  .stat-icon {
    flex: none;
    display: grid;
    place-items: center;
    position: relative;
    width: 72px;
    height: 72px;
    color: var(--md-sys-color-on-primary-container);

    &::before {
      content: '';
      position: absolute;
      inset: 0;
      background: var(--md-sys-color-primary-container);
      clip-path: var(--m-shape-burst);
      animation: m-turn 50s linear infinite;
    }

    .m-icon { position: relative; width: 34px; height: 34px; }
  }

  .stat-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .stat-label {
    font: 500 16px/24px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  .stat-big {
    font: 700 36px/44px var(--md-ref-typeface-brand, system-ui);
    letter-spacing: -0.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &.ok { background: var(--md-sys-color-success-container, var(--md-sys-color-secondary-container)); color: var(--md-sys-color-on-success-container, var(--md-sys-color-on-secondary-container)); }
  &.warn { background: var(--md-sys-color-warning-container, var(--md-sys-color-tertiary-container)); color: var(--md-sys-color-on-warning-container, var(--md-sys-color-on-tertiary-container)); }
  &.error { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
  &.ok .stat-label, &.warn .stat-label, &.error .stat-label { color: inherit; opacity: 0.8; }
}

.section-title {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 44px 0 20px;
  font: 700 28px/36px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface);

  .m-icon {
    box-sizing: content-box;
    width: 28px;
    height: 28px;
    padding: 10px;
    color: var(--md-sys-color-on-primary-container);
    background: var(--md-sys-color-primary-container);
    clip-path: var(--m-shape-squircle);
  }
}

// === 快捷操作：tonal 大卡 ===
.action-card {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 22px 24px;
  min-height: 112px;
  cursor: pointer;
  outline: none;
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
  border: 0;

  transition: border-radius var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease), transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease), box-shadow var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease);

  &:hover, &:focus {
    box-shadow: var(--d-shadow-3);
    transform: translateY(-3px);
    border-radius: var(--md-sys-shape-corner-extra-extra-large);
  }

  &:active {
    transform: scale(0.98);
  }

  .action-icon {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    color: var(--md-sys-color-on-primary);

    &::before {
      content: '';
      position: absolute;
      inset: 0;
      background: var(--md-sys-color-primary);
      clip-path: var(--m-shape-cookie);
      animation: m-turn 40s linear infinite;
      transition: transform var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease);
    }

    .m-icon { position: relative; width: 34px; height: 34px; }
  }

  &:hover .action-icon::before, &:focus .action-icon::before { transform: scale(1.1); }

  &.warning .action-icon {
    color: var(--md-sys-color-on-tertiary);

    &::before { background: var(--md-sys-color-tertiary); }
  }

  .action-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .action-title {
    font: 700 20px/28px var(--md-ref-typeface-brand, system-ui);
  }

  .action-desc {
    font: 400 15px/22px var(--md-ref-typeface-brand, system-ui);
    opacity: 0.8;
  }

  .action-arrow {
    flex: none;
    width: 28px;
    height: 28px;
    opacity: 0.7;
  }
}

// === 系统信息 ===
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 24px 32px;
}

.info-item {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 0;
  padding: 16px 18px;
  border-radius: var(--md-sys-shape-corner-large);
  background: var(--md-sys-color-surface-container-high);

  .info-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
    clip-path: var(--m-shape-squircle);
  }

  .info-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .info-label {
    font: 500 14px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  .info-value {
    font: 600 20px/28px var(--md-ref-typeface-brand, system-ui);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

// === 内存监控 ===
.memory-overview {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 24px;
  margin-bottom: 24px;
}

.memory-stat {
  display: flex;
  flex-direction: column;
  gap: 4px;

  .memory-stat-label {
    font: 500 14px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  .memory-stat-value {
    font: 700 28px/36px var(--md-ref-typeface-brand, system-ui);

    &.highlight {
      color: var(--md-sys-color-primary);
    }
  }
}

.memory-bar-container {
  margin-bottom: 24px;
}

.memory-bar {
  position: relative;
  height: 16px;
  border-radius: 8px;
  background: var(--md-sys-color-surface-container-highest);
  overflow: hidden;

  .memory-bar-fill {
    position: absolute;
    top: 0;
    bottom: 0;
    transition: width var(--m-spring-default-spatial-duration, 400ms) var(--m-spring-default-spatial, ease);
  }

  .sunshine-fill {
    left: 0;
    background: var(--md-sys-color-primary);
    z-index: 1;
  }

  .system-fill {
    background: var(--md-sys-color-tertiary);
  }
}

.memory-bar-legend,
.trend-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
  margin-top: 12px;
  font: 500 15px/22px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);

  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .legend-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;

    &.sunshine { background: var(--md-sys-color-primary); }
    &.system { background: var(--md-sys-color-tertiary); }
    &.free { background: var(--md-sys-color-surface-container-highest); border: 1px solid var(--md-sys-color-outline-variant); }
  }

  .legend-line {
    width: 24px;
    height: 4px;
    border-radius: 2px;

    &.ws { background: var(--md-sys-color-primary); }
    &.pws { background: var(--md-sys-color-tertiary); }
  }
}

.process-list {
  display: flex;
  flex-direction: column;
  margin-bottom: 24px;
  border-radius: 20px;
  overflow: hidden;
  background: var(--md-sys-color-surface-container-low);
}

.process-row {
  display: grid;
  grid-template-columns: 2fr 0.8fr 1fr 1fr 1fr;
  gap: 12px;
  padding: 14px 20px;
  font: 400 16px/24px var(--md-ref-typeface-brand, system-ui);

  &:not(.header):nth-child(even) {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-highest) 50%, transparent);
  }

  &.header {
    font-weight: 600;
    font-size: 14px;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-container-high);
  }

  .process-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .process-mem.peak {
    color: var(--md-sys-color-on-surface-variant);
  }
}

.mem-skel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: 24px;
  i { display: block; border-radius: 8px; background: color-mix(in srgb, var(--md-sys-color-on-surface) 12%, transparent); }
  .mem-skel__bar { height: 16px; margin-bottom: 18px; }
  .mem-skel__line { height: 22px; }
}

.no-process {
  padding: 20px 0 28px;
  font: 400 17px/26px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
}

.trend-title {
  margin-bottom: 12px;
  font: 600 18px/26px var(--md-ref-typeface-brand, system-ui);
}

.trend-svg {
  width: 100%;
  height: 160px;
  border-radius: 20px;
  background: var(--md-sys-color-surface-container-low);

  .grid-line {
    stroke: var(--md-sys-color-outline-variant);
    stroke-width: 1;
  }

  .trend-line {
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;

    &.working-set { stroke: var(--md-sys-color-primary); }
    &.private-ws { stroke: var(--md-sys-color-tertiary); }
  }
}
</style>
