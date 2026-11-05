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
    @open="openUrl"
    @go="goPlace"
  >
    <template #actions>
      <HostActionsPanel @open-web-page="(path) => emit('webui-navigate', path)" @open-guide="emit('open-guide')" />
    </template>
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
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MButton, MDiagnostics, diagnosticsTexts, MGroup, MSection, useDialogs, useNotify } from 'foundation-ui/vue'
import HostActionsPanel from './HostActionsPanel.vue'
import { useDiagnostics } from '../../composables/useDiagnostics.js'
import { usbipStatusChannel, dualsenseStatusChannel, micStatusChannel, vigemStatusChannel } from '../../composables/deviceStatusStore.js'
import { openExternalUrl } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 诊断 of the control panel, one destination for the old 诊断工具 and 故障诊断: the host actions, the problem summary, the bug report, the
// bundle and the error centre on one tab, the whole log on the other. The small tools moved to 工具箱.
const emit = defineEmits(['back', 'go', 'webui-navigate', 'open-guide'])
const { t, locale } = useI18n()
const logText = computed(() => t.value.logConsole)
const refreshing = ref(false)
const say = (text) => useNotify().snack(text)

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
  try { const result = await invoke('export_logs', { format }); await useDialogs().alert(result || logText.value.exportSuccess, logText.value.exportComplete, { type: 'info' }) }
  catch (error) { await useDialogs().alert(`${logText.value.exportFailed}: ${error}`, logText.value.dialogError, { type: 'error' }) }
}
onMounted(() => { diag.loadFacts(); diag.start() })
</script>
