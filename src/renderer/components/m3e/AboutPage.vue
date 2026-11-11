<template>
  <MAbout :version="currentVersion" :channel="channel" :build="build" :update="update" :sponsors="SPONSORS" :locale="locale" :on-refresh="onRefresh" :consent="consent" :errors="errors" :support-link="supportLink" @consent="(v) => emit('consent', v)" @errors="(v) => emit('errors', v)" @open-update-source="emit('open-update-source')" :update-channel="includePrerelease ? 'prerelease' : 'stable'" @channel-change="(value) => emit('channel-change', value === 'prerelease')" @check="runCheck" @open-update="emit('open-update')" @open="(url) => emit('open', url)" @developer-off="options?.off()">
    <template #developer><DeveloperOptions ref="options" :locale="locale" @open="emit('open-developer')" /></template>
  </MAbout>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { MAbout } from 'foundation-ui/vue'
import { channelOfVersion, fetchReleaseMetadata } from 'foundation-ui/update'
import { SPONSORS } from '../../config/sponsors.js'
import DeveloperOptions from './DeveloperOptions.vue'
import { sunshine } from '../../tauri-adapter.js'

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
  consent: { type: String, default: '' },
  /** error reports ('on' | 'off'): a separate switch from the usage statistics */
  errors: { type: String, default: '' },
  supportLink: { type: Boolean, default: false },
  /** the person's update channel (测试版 when true): the same preference as 面板设置; 更新通道 of the page changes it through `channel-change` */
  includePrerelease: { type: Boolean, default: false },
})
const options = ref(null)
const emit = defineEmits(['open-update', 'open', 'consent', 'errors', 'open-update-source', 'channel-change', 'open-developer'])

const checking = ref(false)
const checkedAt = ref(0)
const failed = ref(false)
const channel = ref('')
const build = ref(null)
const update = computed(() => {
  if (checking.value) return { state: 'checking', version: '', checkedAt: checkedAt.value }
  if (failed.value) return { state: 'error', version: '', checkedAt: checkedAt.value }
  const info = props.updateInfo
  if (info && !info.is_latest) return { state: 'available', version: info.version || '', checkedAt: checkedAt.value }
  if (info?.is_latest) return { state: 'latest', version: '', checkedAt: checkedAt.value }
  return { state: 'idle', version: '', checkedAt: checkedAt.value }
})
async function runCheck() {
  checking.value = true
  failed.value = false
  try { failed.value = !(await props.check()); if (!failed.value) checkedAt.value = Date.now() } finally { checking.value = false }
}
// The channel and the build time are what the host recorded when it was built (its build identity, read from sunshine.exe without a
// request). Only a host that predates the identity falls back to the public release metadata for its version; then unknown builds show none.
onMounted(async () => {
  try { build.value = await sunshine.getBuildInfo() } catch { build.value = null }
  if (build.value) return
  try { const result = await fetchReleaseMetadata(); if (result.ok) channel.value = channelOfVersion(result.metadata, props.currentVersion) } catch { /* no channel is shown */ }
})
defineExpose({ runCheck })
</script>
