<template>
  <MDiagnostics
    :title="t.m3e.system.diagnostics"
    :text="diag.text.value"
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
      <DeviceDiagnosticsPanel @open-controller-meta="emit('open-controller-meta')" @open-stylus-input-probe="emit('open-stylus-input-probe')" />
      <MSection :title="t.sidebar.toolbox">
        <MGroup>
          <MItem v-for="tool in utilityTools" :key="tool.id" action :icon="ICONS[tool.id] || 'build'" :title="tool.label" @click="tool.action()">
            <template v-if="tool.external" #trail><MIcon name="open_in_new" /></template>
          </MItem>
        </MGroup>
      </MSection>
    </template>
  </MDiagnostics>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { MDiagnostics, MSection, MGroup, MItem, MIcon } from 'foundation-ui/vue'
import DeviceDiagnosticsPanel from '../controllersHub/DeviceDiagnosticsPanel.vue'
import { useDiagnostics } from '../../composables/useDiagnostics.js'
import { usbipStatusChannel, dualsenseStatusChannel, micStatusChannel, vigemStatusChannel } from '../../composables/deviceStatusStore.js'
import { openExternalUrl } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 日志与诊断 of the control panel: the shared page with the panel's own sources. The device diagnostics and the toolbox rows that were
// here before stay at the top; the error centre, the log viewer, the summary, the bug report and the bundle follow.
const props = defineProps({ tools: { type: Array, default: () => [] }, route: { type: Object, default: null } })
const emit = defineEmits(['back', 'open-controller-meta', 'open-stylus-input-probe', 'go'])
const { t, locale } = useI18n()
// The controller test and the stylus probe are the two cards of the device tools above.
const utilityTools = computed(() => props.tools.filter((tool) => !['gamepad-test', 'stylus-input-probe'].includes(tool.id)))
const ICONS = { 'stream-timer': 'timer', 'latency-test': 'speed' }

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
// where an error leads: the panel pages that hold the cause
const goPlace = (place) => emit('go', place)
onMounted(() => { diag.loadFacts(); diag.start() })
</script>
