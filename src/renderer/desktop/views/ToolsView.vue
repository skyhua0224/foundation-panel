<template>
  <DeskPage :title="t.tools.pageTitle" wide>
    <!-- the system check: what the host and this machine can do, one row each -->
    <MSection>
      <MGroup>
        <MItem v-for="row in DIAGNOSTIC_ROWS" :key="row.key" :icon="row.icon" :title="t.tools[row.key]" :text="diagnostics[row.key].value">
          <template #trail>
            <MLoading v-if="diagnostics[row.key].status === 'connecting'" size="24px" />
            <MChip :tone="STATUS_TONES[diagnostics[row.key].status] || 'neutral'">{{ diagnostics[row.key].statusText }}</MChip>
          </template>
        </MItem>
      </MGroup>
      <MNotice v-if="actionError" tone="error">{{ actionError }}</MNotice>
    </MSection>

    <MSection>
      <MGroup>
        <MItem action icon="restart_alt" :title="t.tools.restartGraphicsDriver" @click="restartGraphicsDriver" />
        <MItem action icon="refresh" :title="t.tools.rediagnose" :disabled="diagnosing" @click="runDiagnostics" />
      </MGroup>
    </MSection>

    <!-- the tools of the toolbox are pages of the panel -->
    <PanelLinkSection :heading="t.tools.inPanel" :links="['liveSession', 'dpi', 'shortcuts', 'logs']" />
  </DeskPage>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { probeSystemDiagnostics } from '../../composables/systemDiagnostics.js'
import { MChip, MGroup, MItem, MLoading, MNotice, MSection } from 'foundation-ui/vue'
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
