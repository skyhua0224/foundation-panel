<template>
  <MBanner v-if="loading" tone="info" icon="refresh">{{ t.rtssTool.checking }}</MBanner>
  <MBanner v-else-if="status.installed === false" tone="info" icon="info">{{ t.rtssTool.notInstalled }}</MBanner>
  <MBanner v-else :tone="status.running ? 'info' : 'warn'" :icon="status.running ? 'check_circle' : 'warning'">
    {{ status.running ? `RTSS v${status.version} · ${t.rtssTool.running}` : t.rtssTool.notRunning }}
    <template #actions><MButton variant="text" size="sm" icon="refresh" :disabled="loading" @click="$emit('refresh')">{{ t.rtssTool.refresh }}</MButton></template>
  </MBanner>

  <!-- rtss-cli 检测 -->
  <MBanner v-if="status.installed !== false && status.running && !status.cli_path" tone="warn" icon="download">
    {{ t.rtssTool.cliNotFound }}
    <template #actions><MButton size="sm" icon="download" :loading="downloading" :disabled="downloading" @click="downloadCli">{{ downloading ? t.rtssTool.downloading : t.rtssTool.autoDownload }}</MButton></template>
  </MBanner>
</template>

<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MBanner, MButton } from 'foundation-ui/vue'
import { useI18n } from '../../../desktop/i18n/index.js'

const { t } = useI18n()

defineProps({
  status: { type: Object, required: true },
  loading: { type: Boolean, default: false },
})
const emit = defineEmits(['refresh', 'message'])

const downloading = ref(false)

async function downloadCli() {
  downloading.value = true
  try {
    await invoke('rtss_download_cli')
    emit('message', t.value.rtssTool.cliDownloaded, 'success')
    // 刷新状态以检测新下载的 cli
    emit('refresh')
  } catch (e) {
    emit('message', String(e), 'error')
  } finally {
    downloading.value = false
  }
}
</script>
