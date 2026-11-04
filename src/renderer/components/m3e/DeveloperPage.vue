<template>
  <MPage :title="developerTexts(locale).title">
    <MDeveloperPage :locale="locale" :tools="tools" :api-base="apiBase" @disabled="$emit('disabled')" @notify="(text) => $emit('notify', text)" />
  </MPage>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MPage, MDeveloperPage } from 'foundation-ui/vue'
import { developerTexts } from 'foundation-ui/developer'
import { buildReport, reportToDocument, sendDocument, loadQueue } from 'foundation-ui/telemetry'
import { sunshine } from '../../tauri-adapter.js'
import { desktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'
import { createDeveloperTools } from '../../composables/developerTools.js'

// 开发者选项: the page of foundation-ui with the tools of this panel. It is reached from 系统 and from 关于 once developer mode is on (seven taps on the version).
const props = defineProps({
  locale: { type: String, default: 'zh' },
  version: { type: String, default: '' },
  consent: { type: String, default: 'ask' },
  errorContext: { type: Function, default: null },
  /** show the surfaces of a host that is away: '' (back to normal) | 'offline' | 'stopped' */
  simulateHost: { type: Function, default: null },
})
defineEmits(['disabled', 'notify'])

const apiBase = ref('')
onMounted(async () => { apiBase.value = String((await sunshine.getProxyUrl().catch(() => '')) || '').replace(/\/$/, '') })

const tools = createDeveloperTools({
  invoke,
  settings: { get: () => desktopSettings.value, save: saveDesktopSettings },
  locale: props.locale,
  version: props.version,
  hostVersion: () => sunshine.getVersion(),
  base: () => sunshine.getProxyUrl(),
  kit: { consent: () => props.consent, queue: () => loadQueue().length, report: buildReport, document: reportToDocument, send: sendDocument },
  errorContext: () => props.errorContext?.() || {},
  simulateHost: (kind) => props.simulateHost?.(kind),
  save: (name, text) => {
    const link = document.createElement('a')
    link.href = URL.createObjectURL(new Blob([text], { type: 'text/plain;charset=utf-8' }))
    link.download = name
    link.click()
    setTimeout(() => URL.revokeObjectURL(link.href), 4000)
  },
})
</script>
