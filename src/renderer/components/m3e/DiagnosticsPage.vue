<template>
  <MDiagnostics
    :title="diagnosticsTexts(locale).title"
    :text="diag.text.value"
    :records="diag.records.value"
    panel
    v-model:live="diag.live.value"
    :facts="diag.facts.value"
    :collect="diag.collect"
    :save="diag.save"
    :stats-for="diag.statsFor"
    :host-names="diag.hostNames.value"
    :client-names="diag.clientNames.value"
    :locale="locale"
    :close-label="t.m3e.home.done"
    :commands="commands"
    :notes="notes"
    :disabled="disabled"
    :labels="labels"
    :ai="aiSheet"
    @open="openUrl"
    @go="goPlace"
  >
    <template #tools>
      <MSection :title="logText.title">
        <MGroup>
          <MButton variant="tonal" icon="refresh" :loading="refreshing" @click="refreshLogs">{{ logText.refresh }}</MButton>
          <MButton variant="tonal" icon="download" @click="exportLogs('txt')">{{ logText.exportTxt }}</MButton>
          <MButton variant="tonal" icon="data_object" @click="exportLogs('json')">{{ logText.exportJson }}</MButton>
          <MButton variant="danger" icon="delete" @click="clearLogs">{{ logText.clear }}</MButton>
        </MGroup>
      </MSection>
    </template>
  </MDiagnostics>
  <MDialog v-model="boomAsk" :title="texts.boom" icon="warning" tone="error">
    <p>{{ texts.boomText }}</p>
    <template #actions>
      <MButton variant="text" @click="boomAsk = false">{{ texts.cancel || t.m3e.home.done }}</MButton>
      <MButton variant="danger" icon="warning" @click="confirmBoom">{{ texts.boom }}</MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MDiagnostics, MDialog, MButton, diagnosticsTexts, guideTexts, coachTexts, remoteTexts, MGroup, MSection, useDialogs, useNotify } from 'foundation-ui/vue'
import { useDiagnostics } from '../../composables/useDiagnostics.js'
import { exportPanelLogs } from '../../composables/panelLogExport.js'
import { useLogDiagnosis } from '../../composables/useLogDiagnosis.js'
import { usbipStatusChannel, dualsenseStatusChannel, micStatusChannel, vigemStatusChannel } from '../../composables/deviceStatusStore.js'
import { openExternalUrl, sunshine } from '../../tauri-adapter.js'
import { saveThroughDialog } from '../../shared/saveDialog.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 诊断与日志. The same page as the Web UI. What only this window can read (the panel log, the device status) is passed as facts.
const emit = defineEmits(['back', 'go', 'webui-navigate', 'open-guide', 'open-remote'])
const { t, locale } = useI18n()
const logText = computed(() => t.value.logConsole)
const refreshing = ref(false)
const say = (text) => useNotify().snack(text)
const texts = computed(() => diagnosticsTexts(locale.value))

const data = (r) => (r?.success ? r.data : null)
const devices = async () => {
  const [usb, ds, mic, vigem] = await Promise.all([usbipStatusChannel, dualsenseStatusChannel, micStatusChannel, vigemStatusChannel].map((c) => c.fetch({ force: false }).catch(() => null)))
  const pick = (o, keys) => (o ? Object.fromEntries(keys.filter((k) => o[k] !== undefined).map((k) => [k, o[k]])) : null)
  return {
    usbip: pick(data(usb), ['supported', 'installed', 'ready', 'version', 'pinned_version', 'version_supported', 'signature', 'signer']),
    dualsense: pick(data(ds), ['state', 'host']),
    microphone: pick(data(mic), ['configured_backend', 'active_backend', 'component_available', 'state']),
    vigem: pick(data(vigem), ['installed', 'version']),
  }
}
const diag = useDiagnostics({ devices })
const ai = useLogDiagnosis()
const boomAsk = ref(false)
const note = ref({ restart: '', resetDisplay: '', forceClose: '', boom: '' })
const busy = ref({ restart: false, resetDisplay: false, forceClose: false, boom: false })

const base = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const post = async (path, body) => {
  const response = await fetch(`${await base()}${path}`, { method: 'POST', headers: body ? { 'Content-Type': 'application/json' } : undefined, body: body ? JSON.stringify(body) : undefined })
  if (!response.ok) throw new Error(`HTTP ${response.status}`)
  return response
}
async function run(id, action) {
  busy.value = { ...busy.value, [id]: true }
  try { await action(); return true } catch { note.value = { ...note.value, [id]: { text: texts.value.failed, tone: 'error' } }; return false }
  finally { busy.value = { ...busy.value, [id]: false } }
}
const restart = () => run('restart', () => post('/api/restart')).then((ok) => { if (ok) note.value = { ...note.value, restart: texts.value.restart } })
const resetDisplay = () => run('resetDisplay', () => post('/api/reset-display-device-persistence')).then((ok) => { note.value = { ...note.value, resetDisplay: ok ? { text: texts.value.resetDisplay, tone: 'info' } : { text: texts.value.resetDisplay, tone: 'error' } } })
const forceClose = () => run('forceClose', () => post('/api/apps/close')).then((ok) => { note.value = { ...note.value, forceClose: ok ? { text: texts.value.forceClose, tone: 'info' } : { text: texts.value.forceClose, tone: 'error' } } })
async function confirmBoom() {
  boomAsk.value = false
  const ok = await run('boom', async () => { await fetch(`${await base()}/api/boom`) })
  if (ok) note.value = { ...note.value, boom: texts.value.boom }
}
async function wizard() {
  await run('wizard', async () => {
    const response = await fetch(`${await base()}/api/config`)
    if (!response.ok) throw new Error('config')
    const config = await response.json()
    config.setup_wizard_completed = false
    await post('/api/config', config)
    emit('open-guide')
  })
}
async function copyLogs() { try { await navigator.clipboard.writeText(diag.text.value || '') } catch { /* the clipboard is not available */ } }
async function copyConfig() {
  try {
    const response = await fetch(`${await base()}/api/config`)
    if (!response.ok) throw new Error('config')
    await navigator.clipboard.writeText(JSON.stringify(await response.json(), null, 2))
  } catch { /* the configuration stays where it is */ }
}
async function downloadLogs() {
  const name = 'sunshine-logs-' + new Date().toISOString().slice(0, 19).replace(/[:.]/g, '-') + '.txt'
  try {
    const response = await fetch(`${await base()}/api/logs`)
    if (!response.ok) throw new Error('logs')
    const content = await response.text()
    const invoke = window.__TAURI_INTERNALS__?.invoke
    if (invoke) {
      const saved = await saveThroughDialog(invoke, 'save_text_file', { content, defaultName: name, filterName: 'Text', extensions: ['txt'] })
      if (saved === null) return
      return
    }
    const link = Object.assign(document.createElement('a'), { href: URL.createObjectURL(new Blob([content], { type: 'text/plain' })), download: name })
    link.click()
    URL.revokeObjectURL(link.href)
  } catch (e) { note.value = { ...note.value, downloadLogs: { text: `${texts.value.failed}: ${e?.message || e}`, tone: 'error' } } }
}

const commands = computed(() => ({
  restart,
  resetDisplay: diag.platform.value === 'windows' ? resetDisplay : undefined,
  ai: () => ai.open(),
  diagnose: () => ai.diagnose(diag.text.value),
  downloadLogs,
  copyLogs,
  copyConfig,
  guide: () => emit('open-guide'),
  coach: () => window.dispatchEvent(new CustomEvent('foundation-coach-start')),
  remote: () => emit('open-remote'),
  wizard,
  forceClose,
  boom: () => { boomAsk.value = true },
}))
const notes = computed(() => note.value)
const disabled = computed(() => busy.value)
const labels = computed(() => ({
  guide: { title: guideTexts(locale.value).guide, text: guideTexts(locale.value).helpRow },
  coach: { title: coachTexts(locale.value).rerun },
  remote: { title: remoteTexts(locale.value).openText },
}))
const aiSheet = computed(() => ({
  show: ai.show.value,
  config: ai.config,
  isConfigLoading: ai.isConfigLoading.value,
  isSavingConfig: ai.isSavingConfig.value,
  isLoading: ai.isLoading.value,
  result: ai.result.value,
  error: ai.error.value,
  localFindings: ai.localFindings.value,
  localSuggestions: ai.localSuggestions.value,
  onClose: ai.close,
}))
const openUrl = (url) => { openExternalUrl(url) }
const goPlace = (place) => emit('go', place)
async function refreshLogs() {
  refreshing.value = true
  try { await diag.refresh(); say(logText.value.refreshed.replace('{count}', String(diag.records.value.length))) }
  catch (error) { say(`${logText.value.refreshFailed}: ${error}`) }
  finally { refreshing.value = false }
}
async function clearLogs() {
  const confirmed = await useDialogs().confirm(logText.value.confirmClear, logText.value.clearLogsTitle, { type: 'warning', danger: true, confirmText: logText.value.clear }).then(() => true, () => false)
  if (!confirmed) return
  try { await invoke('clear_logs'); await diag.refresh(); say(logText.value.logsCleared) }
  catch (error) { say(`${logText.value.clearFailed}: ${error}`) }
}
async function exportLogs(format) {
  await exportPanelLogs({ invoke, format, texts: logText.value, alert: useDialogs().alert })
}
onMounted(() => { diag.loadFacts(); diag.start() })
</script>
