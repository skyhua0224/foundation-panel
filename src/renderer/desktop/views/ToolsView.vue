<template>
  <DeskPage :title="t.tools.pageTitle" icon="build" :text="heroText" wide>
    <!-- the system check: what the host and this machine can do, in four tiles that turn green or amber -->
    <section class="dsk-diag">
      <div class="dsk-diag__grid">
        <div v-for="row in DIAGNOSTIC_ROWS" :key="row.key" class="dsk-diag__tile" :class="`is-${diagnostics[row.key].status}`">
          <span class="dsk-diag__icon">
            <span class="dsk-diag__shape" />
            <MLoading v-if="diagnostics[row.key].status === 'connecting'" size="30px" />
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
        <MButton variant="tonal" size="lg" icon="restart_alt" @click="restartGraphicsDriver">{{ t.tools.restartGraphicsDriver }}</MButton>
        <MButton size="lg" icon="refresh" :loading="diagnosing" @click="runDiagnostics">{{ t.tools.rediagnose }}</MButton>
      </div>
    </section>

    <!-- the tools of the toolbox are pages of the panel -->
    <PanelLinkSection :heading="t.tools.inPanel" :links="['liveSession', 'dpi', 'shortcuts', 'logs']" />
  </DeskPage>
</template>

<script setup>
import { computed, ref, onMounted } from 'vue'
import { probeSystemDiagnostics } from '../../composables/systemDiagnostics.js'
import { MButton, MChip, MIcon, MLoading, MNotice } from 'foundation-ui/vue'
import { requestConfirm } from '../composables/useConfirm.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'
import PanelLinkSection from '../components/PanelLinkSection.vue'

const { t } = useI18n()

// Tauri 命令
const invoke = ref(null)
const diagnosing = ref(false)

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
// the hero's fact: how many of the four checks pass
const heroText = computed(() => {
  const rows = Object.values(diagnostics.value)
  if (rows.some((row) => row.status === 'connecting')) return t.value.tools.status.detecting
  return t.value.tools.checksSummary.replace('{ok}', rows.filter((row) => row.status === 'good').length).replace('{n}', rows.length)
})

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
.dsk-diag { display: flex; flex-direction: column; gap: 18px; }
.dsk-diag__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.dsk-diag__tile { display: flex; align-items: center; gap: 18px; min-width: 0; padding: 20px 22px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface); transition: background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease); }
.dsk-diag__icon { position: relative; flex: none; width: 72px; height: 72px; display: grid; place-items: center; font-size: 34px; color: var(--md-sys-color-on-secondary-container); }
.dsk-diag__shape { position: absolute; inset: 0; background: var(--md-sys-color-secondary-container); clip-path: var(--m-shape-cookie); animation: m-turn 44s linear infinite; }
.dsk-diag__icon :deep(.m-icon), .dsk-diag__icon :deep(.m-loading) { position: relative; }
.dsk-diag__tile.is-good { background: var(--md-sys-color-success-container); color: var(--md-sys-color-on-success-container); }
.dsk-diag__tile.is-good .dsk-diag__shape { background: var(--md-sys-color-success); }
.dsk-diag__tile.is-good .dsk-diag__icon { color: var(--md-sys-color-on-success); }
.dsk-diag__tile.is-warning { background: var(--md-sys-color-warning-container); color: var(--md-sys-color-on-warning-container); }
.dsk-diag__tile.is-warning .dsk-diag__shape { background: var(--md-sys-color-warning); }
.dsk-diag__tile.is-warning .dsk-diag__icon { color: var(--md-sys-color-on-warning); }
.dsk-diag__tile.is-error { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.dsk-diag__tile.is-error .dsk-diag__shape { background: var(--md-sys-color-error); }
.dsk-diag__tile.is-error .dsk-diag__icon { color: var(--md-sys-color-on-error); }
.dsk-diag__info { flex: 1; min-width: 0; }
.dsk-diag__name { font: var(--m-label-large); opacity: .85; }
.dsk-diag__value { font: var(--m-title-large); overflow-wrap: anywhere; }
.dsk-diag__actions { display: flex; flex-wrap: wrap; gap: 12px; justify-content: flex-end; }

@media (max-width: 1300px) {
  .dsk-diag__grid { grid-template-columns: minmax(0, 1fr); }
}
</style>
