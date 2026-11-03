<template>
  <MAbout :version="currentVersion" :channel="channel" :update="update" :sponsors="SPONSORS" :locale="locale" :on-refresh="onRefresh" @check="runCheck" @open-update="emit('open-update')" @open="(url) => emit('open', url)" @developer-off="options?.off()">
    <template #developer><DeveloperOptions ref="options" /></template>
  </MAbout>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { MAbout } from 'foundation-ui/vue'
import { channelOfVersion, fetchReleaseMetadata } from 'foundation-ui/update'
import { SPONSORS } from '../../config/sponsors.js'
import DeveloperOptions from './DeveloperOptions.vue'

// 关于 of the control panel: the shared page with the panel's data. The update check is the panel's own (`check_for_updates`) and its
// answer opens the same update sheet as everywhere else; the page only shows the state of the last answer.
const props = defineProps({
  currentVersion: { type: String, default: '' },
  updateInfo: { type: Object, default: null },
  locale: { type: String, default: 'en' },
  /** the panel's check: resolves true when it produced an answer (the update sheet then opens), false on failure */
  check: { type: Function, required: true },
  /** fetch the announcements again (the developer entry changes their source) */
  onRefresh: { type: Function, default: null },
})
const options = ref(null)
const emit = defineEmits(['open-update', 'open'])

const checking = ref(false)
const failed = ref(false)
const channel = ref('')
const update = computed(() => {
  if (checking.value) return { state: 'checking', version: '' }
  if (failed.value) return { state: 'error', version: '' }
  const info = props.updateInfo
  if (info && !info.is_latest) return { state: 'available', version: info.version || '' }
  if (info?.is_latest) return { state: 'latest', version: '' }
  return { state: 'idle', version: '' }
})
async function runCheck() {
  checking.value = true
  failed.value = false
  try { failed.value = !(await props.check()) } finally { checking.value = false }
}
// The channel of the installed build is what the public release metadata says about its version; unknown builds show no channel.
onMounted(async () => {
  try { const result = await fetchReleaseMetadata(); if (result.ok) channel.value = channelOfVersion(result.metadata, props.currentVersion) } catch { /* no channel is shown */ }
})
defineExpose({ runCheck })
</script>
