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
defineProps({ autostart: { type: [Boolean, Number], default: false } })
const emit = defineEmits(['go', 'restart', 'ai'])
const { t, locale } = useI18n()

const proxyBase = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const request = async (path) => {
  try {
    const response = await fetch(`${await proxyBase()}${path}`, { signal: AbortSignal.timeout(110000) })
    return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
  }
  catch (error) {
    const timedOut = error && (error.name === 'TimeoutError' || error.name === 'AbortError')
    return { ok: false, status: 0, json: { error: timedOut ? 'the host did not answer within 110 s' : String(error) } }
  }
}
const local = {
  id: 'local',
  groups: DOCTOR_LOCAL_GROUPS,
  fetch: async (group, lang) => {
    try {
      const json = await Promise.race([
        invoke('doctor_local', { only: group, lang }),
        new Promise((_, reject) => setTimeout(() => reject(new Error('the check did not finish within 20 s')), 20000)),
      ])
      return { ok: true, status: 200, json }
    }
    catch (error) { return { ok: false, status: 500, json: { error: String(error) } } }
  },
}
const sources = [local, doctorHostSource(request)]

// The repairs this panel can run. With 系统 › Sunshine 服务 (panel feat/service-page) the service is started and its start type set
// through its hardened elevated helper; without it the start goes through the old restart command and the start type is not offered
// (the old writer puts a script in %TEMP% before elevating: audit danger table).
const serviceHelper = ref(false)
const REPAIRS = {
  'service:start': () => (serviceHelper.value ? invoke('control_sunshine_service', { action: 'start', acknowledgeLiveStream: false }) : invoke('restart_sunshine_service')),
  'service:start_type': () => invoke('set_sunshine_service_start_type', { startType: 'auto', acknowledgeUnreachableAfterReboot: false }),
  'driver:vdd': () => invoke('install_vdd_driver'),
  'driver:vigem': () => invoke('install_vigem_driver', { force: true }),
  'driver:vmouse': () => invoke('install_vmouse_driver'),
  'driver:usbip': () => invoke('usbip_install_transport'),
}
const canRepair = (action) => !!REPAIRS[action.target] && (action.target !== 'service:start_type' || serviceHelper.value)
const repair = async (action) => {
  const run = REPAIRS[action.target]
  if (!run) throw new Error(action.target)
  await run()
}

// a stream that is live: a fix that disconnects says so
const streaming = ref(false)
onMounted(async () => {
  diag.loadFacts()
  // the service page's commands exist only once feat/service-page is merged
  invoke('get_sunshine_service_status').then(() => { serviceHelper.value = true }, () => {})
  try { streaming.value = ((await request('/api/runtime/sessions')).json?.sessions || []).length > 0 } catch { /* Sunshine is down: no stream */ }
})
const diag = useDiagnostics()
</script>
