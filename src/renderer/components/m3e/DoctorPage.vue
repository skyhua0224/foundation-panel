<template>
  <MDoctor
    :request="request"
    :collect="diag.collect"
    :save="diag.save"
    panel
    :host-names="diag.hostNames.value"
    :client-names="diag.clientNames.value"
    :locale="locale"
    :close-label="t.m3e.home.done"
    :autostart="autostart"
    @go="(target) => emit('go', target)"
    @open="openExternalUrl"
    @restart="emit('restart')"
    @ai="({ prompt }) => emit('ai', prompt)"
  />
</template>

<script setup>
import { onMounted } from 'vue'
import { MDoctor } from 'foundation-ui/vue'
import { useDiagnostics } from '../../composables/useDiagnostics.js'
import { openExternalUrl, sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 体检 of the control panel: the shared page (MDoctor) through the panel's proxy to the host; the bundle is the one of 日志与诊断 (with
// the panel log), 米塔 AI is the panel's chat. Where a fix leads is the shell's business (go).
defineProps({ autostart: { type: Boolean, default: false } })
const emit = defineEmits(['go', 'restart', 'ai'])
const { t, locale } = useI18n()

const request = async (path) => {
  const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
  const response = await fetch(`${base}${path}`)
  return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
}
const diag = useDiagnostics()
onMounted(() => diag.loadFacts())
</script>
