<template>
  <DeskPage :title="t.tools.pageTitle" wide>
    <!-- the tools of the toolbar, side by side -->
    <div class="dsk-tools">
      <section v-for="tool in tools" :key="tool.id" class="dsk-tool" :class="`dsk-tool--${tool.id}`">
        <header class="dsk-tool__head">
          <span class="dsk-tool__icon"><MIcon :name="tool.icon" fill /></span>
          <h2 class="dsk-tool__title">{{ tool.title }}</h2>
        </header>
        <div class="dsk-tool__body">
          <component :is="tool.component" :embedded="true" />
        </div>
      </section>
    </div>

    <!-- diagnostics -->
    <section class="dsk-diag">
      <header class="dsk-tool__head">
        <span class="dsk-tool__icon"><MIcon name="stethoscope" fill /></span>
        <h2 class="dsk-tool__title">{{ t.tools.diagnostics }}</h2>
      </header>

      <div class="dsk-diag__grid">
        <div v-for="row in DIAGNOSTIC_ROWS" :key="row.key" class="dsk-diag__tile" :class="`is-${diagnostics[row.key].status}`">
          <span class="dsk-diag__icon">
            <MLoading v-if="diagnostics[row.key].status === 'connecting'" size="28px" />
            <MIcon v-else :name="row.icon" fill />
          </span>
          <div class="dsk-diag__info">
            <div class="dsk-diag__name">{{ t.tools[row.key] }}</div>
            <div class="dsk-diag__value">{{ diagnostics[row.key].value }}</div>
          </div>
          <MChip :tone="STATUS_TONES[diagnostics[row.key].status] || 'neutral'">{{ diagnostics[row.key].statusText }}</MChip>
        </div>
      </div>

      <MNotice v-if="actionError" tone="error">{{ actionError }}</MNotice>

      <div class="dsk-diag__actions">
        <MButton variant="tonal" icon="restart_alt" @click="restartGraphicsDriver">{{ t.tools.restartGraphicsDriver }}</MButton>
        <MButton icon="refresh" :loading="diagnosing" @click="runDiagnostics">{{ t.tools.rediagnose }}</MButton>
      </div>
    </section>
  </DeskPage>
</template>

<script setup>
import { computed, ref, onMounted } from 'vue'
import { probeSystemDiagnostics } from '../../composables/systemDiagnostics.js'
import { MButton, MChip, MIcon, MLoading, MNotice } from 'foundation-ui/vue'
import { requestConfirm } from '../composables/useConfirm.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'

// 工具组件 - 复用工具栏的工具
import BitrateTool from '../../tool-window/tools/BitrateTool.vue'
import DpiAdjusterTool from '../../tool-window/tools/DpiAdjusterTool.vue'
import HostPerformanceTool from '../../tool-window/tools/HostPerformanceTool.vue'
import ShortcutsTool from '../../tool-window/tools/ShortcutsTool.vue'

const { t } = useI18n()

// Tauri 命令
const invoke = ref(null)
const diagnosing = ref(false)

const tools = computed(() => [
  { id: 'bitrate', icon: 'monitoring', title: t.value.tools.bitrateAdjust, component: BitrateTool },
  { id: 'host', icon: 'monitor_heart', title: t.value.tools.hostPerformance, component: HostPerformanceTool },
  { id: 'dpi', icon: 'aspect_ratio', title: t.value.tools.dpiScaling, component: DpiAdjusterTool },
  { id: 'shortcuts', icon: 'keyboard', title: t.value.tools.shortcutGuide, component: ShortcutsTool },
])

const diagnostics = ref({
  gpu: { value: t.value.tools.status.detectingEllipsis, status: 'connecting', statusText: t.value.tools.status.detecting },
  encoder: { value: t.value.tools.status.detectingEllipsis, status: 'connecting', statusText: t.value.tools.status.detecting },
  network: { value: t.value.tools.status.detectingEllipsis, status: 'connecting', statusText: t.value.tools.status.detecting },
  firewall: { value: t.value.tools.status.detectingEllipsis, status: 'connecting', statusText: t.value.tools.status.detecting },
})

const actionError = ref('')

// The window's own confirm dialog: a native one cannot be answered with a pad.
async function restartGraphicsDriver() {
  const confirmed = await requestConfirm({
    title: t.value.tools.restartGraphicsDriver,
    message: t.value.tools.confirmRestartGraphics,
    confirmLabel: t.value.tools.restartGraphicsDriver,
    danger: true,
  })
  if (!confirmed || !invoke.value) return
  actionError.value = ''
  try {
    await invoke.value('restart_graphics_driver')
  } catch (e) {
    console.error('Failed to restart graphics driver:', e)
    actionError.value = e?.message || String(e)
  }
}

// The tone of a diagnostic, and the glyph of what it checks.
const STATUS_TONES = { good: 'ok', warning: 'warn', error: 'error', connecting: 'info' }
const DIAGNOSTIC_ROWS = [
  { key: 'gpu', icon: 'memory' },
  { key: 'encoder', icon: 'videocam' },
  { key: 'network', icon: 'network_check' },
  { key: 'firewall', icon: 'shield' },
]

async function runDiagnostics() {
  if (diagnosing.value) return
  diagnosing.value = true
  for (const key of Object.keys(diagnostics.value)) {
    diagnostics.value[key] = { value: t.value.tools.status.detectingEllipsis, status: 'connecting', statusText: t.value.tools.status.detecting }
  }
  try {
    diagnostics.value = await probeSystemDiagnostics(
      invoke.value || (() => { throw new Error('Bridge unavailable') }),
      fetch,
      t.value.tools.status,
    )
  } finally {
    diagnosing.value = false
  }
}

onMounted(async () => {
  // 动态导入 Tauri API
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
  } catch (e) {
    console.log('Tauri invoke not available:', e)
  }
  runDiagnostics()
})
</script>

<style scoped>
.dsk-tools { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; align-items: start; margin-bottom: 28px; }
.dsk-tool--shortcuts { grid-column: 1 / -1; }
.dsk-tool, .dsk-diag { display: flex; flex-direction: column; gap: 16px; min-width: 0; padding: 20px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--m-card); color: var(--md-sys-color-on-surface); }
.dsk-tool__head { display: flex; align-items: center; gap: 12px; }
.dsk-tool__icon { flex: none; width: 40px; height: 40px; border-radius: 14px; display: grid; place-items: center; font-size: 24px; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.dsk-tool__title { margin: 0; min-width: 0; color: var(--md-sys-color-on-surface); font: var(--m-title-large); letter-spacing: 0; }
.dsk-tool__body { min-height: 0; }
.dsk-tool__body :deep(.tool-header) { display: none; }

.dsk-diag__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.dsk-diag__tile { display: flex; align-items: center; gap: 14px; min-width: 0; padding: 16px; border-radius: var(--md-sys-shape-corner-large); background: var(--md-sys-color-surface-container-high); }
.dsk-diag__icon { flex: none; width: 52px; height: 52px; border-radius: 18px; display: grid; place-items: center; font-size: 28px; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.dsk-diag__tile.is-good .dsk-diag__icon { background: var(--md-sys-color-success-container); color: var(--md-sys-color-on-success-container); }
.dsk-diag__tile.is-warning .dsk-diag__icon { background: var(--md-sys-color-warning-container); color: var(--md-sys-color-on-warning-container); }
.dsk-diag__tile.is-error .dsk-diag__icon { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.dsk-diag__info { flex: 1; min-width: 0; }
.dsk-diag__name { font: var(--m-label-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-diag__value { font: var(--m-title-medium); overflow-wrap: anywhere; }
.dsk-diag__actions { display: flex; flex-wrap: wrap; gap: 12px; justify-content: flex-end; }

@media (max-width: 1100px) {
  .dsk-tools, .dsk-diag__grid { grid-template-columns: minmax(0, 1fr); }
}
</style>
