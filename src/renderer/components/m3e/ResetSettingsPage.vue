<script setup>
import { onMounted, ref } from 'vue'
import { MResetSettings, createResetClient } from 'foundation-ui/vue'
import { sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { resetPanelPreferences } from '../../composables/panelPreferences.js'

// 恢复默认设置: foundation-ui's page, with the host's calls through the panel's proxy (it carries the credentials) and the preferences of this panel as the second part.
const { locale } = useI18n()
const emit = defineEmits(['back'])
const base = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const host = createResetClient(async (path, init) => fetch(`${await base()}${path}`, init))
const panel = { reset: () => resetPanelPreferences() }
// a stream that is running ends when the host restarts: the question says so
const streaming = ref(false)
onMounted(async () => {
  try {
    const response = await fetch(`${await base()}/api/runtime/sessions`)
    if (response.ok) streaming.value = ((await response.json())?.total_sessions || 0) > 0
  } catch { /* no host: the question does not mention a stream */ }
})
</script>

<template>
  <MResetSettings :locale="locale" :host="host" :panel="panel" :streaming="streaming" @back="emit('back')" />
</template>
