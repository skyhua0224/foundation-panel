<template>
  <div class="m-rtss" :class="{ 'is-embedded': embedded }">
    <header v-if="!embedded" class="m-rtss__head"><h2>{{ t.rtssTool.title }}</h2><MIconButton icon="close" :label="t.toolWindow.close" @click="$emit('close')" /></header>
    <MBanner v-if="problem" tone="error" icon="error">{{ problem }}</MBanner>

    <RtssStatusBar :status="status" :loading="statusLoading" @refresh="loadStatus" @message="showMessage" />

    <template v-if="status.running">
      <div class="m-rtss__grid">
        <RtssFpsSection v-if="status.cli_path" @message="showMessage" />
        <RtssOsdEditor :has-cli="!!status.cli_path" @message="showMessage" />
        <RtssOsdSettings v-if="status.cli_path" @message="showMessage" />
        <RtssMonitoring @message="showMessage" />
        <HwInfoMonitor @message="showMessage" />
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MBanner, MIconButton, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

import RtssStatusBar from './rtss/RtssStatusBar.vue'
import RtssFpsSection from './rtss/RtssFpsSection.vue'
import RtssOsdEditor from './rtss/RtssOsdEditor.vue'
import RtssOsdSettings from './rtss/RtssOsdSettings.vue'
import RtssMonitoring from './rtss/RtssMonitoring.vue'
import HwInfoMonitor from './rtss/HwInfoMonitor.vue'

const { t } = useI18n()

defineProps({
  embedded: { type: Boolean, default: false },
})
defineEmits(['close'])

const statusLoading = ref(true)
const status = ref({
  running: false,
  version: '',
  osd_slot_count: 0,
  app_count: 0,
  cli_path: '',
  hooks_dll_path: '',
})

const problem = ref('')

async function loadStatus() {
  statusLoading.value = true
  try {
    status.value = await invoke('get_rtss_status')
  } catch (e) {
    console.error('获取 RTSS 状态失败:', e)
    status.value.running = false
  } finally {
    statusLoading.value = false
  }
}

// what worked is a snackbar; what failed (or needs a choice first) stays as a banner until the next message
function showMessage(msg, type) {
  if (type === 'error' || type === 'warning') { problem.value = msg; return }
  problem.value = ''
  useNotify().snack(msg)
}

onMounted(() => {
  loadStatus()
})
</script>

<style>
.m-rtss { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; width: 100%; color: var(--md-sys-color-on-surface); }
.m-rtss__head { display: flex; align-items: center; justify-content: space-between; } .m-rtss__head h2 { margin: 0; font: var(--m-title-large); }
.m-rtss__grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(360px, 100%), 1fr)); gap: 24px 16px; align-items: start; }
.m-rtss__actions { display: flex; justify-content: flex-end; gap: 8px; flex-wrap: wrap; margin-top: 8px; }
</style>
