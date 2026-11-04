<template>
  <MDoctor
    :sources="sources"
    :repair="repair"
    :can-repair="canRepair"
    :collect="diag.collect"
    :save="diag.save"
    panel
    :host-names="diag.hostNames.value"
    :client-names="diag.clientNames.value"
    :locale="locale"
    :close-label="t.m3e.home.done"
    :autostart="autostart"
    :streaming="streaming"
    @go="(target) => emit('go', target)"
    @open="openExternalUrl"
    @restart="emit('restart')"
    @ai="({ prompt }) => emit('ai', prompt)"
  />
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MDoctor, DOCTOR_LOCAL_GROUPS, doctorHostSource } from 'foundation-ui/vue'
import { useDiagnostics } from '../../composables/useDiagnostics.js'
import { openExternalUrl, sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 体检 of the control panel: the panel's own checks (doctor_local: the service, the installation, the processes and ports, the
// drivers; they answer when Sunshine is down) and Sunshine's (GET /api/doctor through the proxy), on the shared page (MDoctor).
// The repairs a check asks for run through the panel's existing commands; the bundle is the one of 日志与诊断, 米塔 AI is the chat.
defineProps({ autostart: { type: Boolean, default: false } })
const emit = defineEmits(['go', 'restart', 'ai'])
const { t, locale } = useI18n()

const proxyBase = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const request = async (path) => {
  const response = await fetch(`${await proxyBase()}${path}`)
  return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
}
const local = {
  id: 'local',
  groups: DOCTOR_LOCAL_GROUPS,
  fetch: async (group, lang) => {
    try { return { ok: true, status: 200, json: await invoke('doctor_local', { only: group, lang }) } }
    catch (error) { return { ok: false, status: 500, json: { error: String(error) } } }
  },
}
const sources = [local, doctorHostSource(request)]

// The repairs this panel can run. service:start_type waits for the service work's hardened helper (the old path writes a temp script).
const REPAIRS = {
  'service:start': () => invoke('restart_sunshine_service'),
  'driver:vdd': () => invoke('install_vdd_driver'),
  'driver:vigem': () => invoke('install_vigem_driver', { force: true }),
  'driver:vmouse': () => invoke('install_vmouse_driver'),
  'driver:usbip': () => invoke('usbip_install_transport'),
}
const canRepair = (action) => !!REPAIRS[action.target]
const repair = async (action) => {
  const run = REPAIRS[action.target]
  if (!run) throw new Error(action.target)
  await run()
}

// a stream that is live: a fix that disconnects says so
const streaming = ref(false)
onMounted(async () => {
  diag.loadFacts()
  try { streaming.value = ((await request('/api/runtime/sessions')).json?.sessions || []).length > 0 } catch { /* Sunshine is down: no stream */ }
})
const diag = useDiagnostics()
</script>
