<template>
  <MPage :title="title" :wide="wide" :cols="false" :lead="lead">
    <template #actions><DevSimChip :locale="locale" /></template>
    <DevIssues v-if="page === ROUTES.DEV_ISSUES" :locale="locale" :open-url="openUrl" @notify="(text) => $emit('notify', text)" />
    <DevLogs v-else-if="page === ROUTES.DEV_LOGS" :locale="locale" :text="tools.logText" @notify="(text) => $emit('notify', text)" />
    <DevApi v-else-if="page === ROUTES.DEV_API" :locale="locale" :api-base="apiBase" @notify="(text) => $emit('notify', text)" />
    <DevFlags v-else-if="page === ROUTES.DEV_FLAGS" :locale="locale" />
    <DevBuild v-else-if="page === ROUTES.DEV_BUILD" :locale="locale" :environment="tools.environment" @notify="(text) => $emit('notify', text)" />
    <DevTests v-else-if="page === ROUTES.DEV_TESTS" :locale="locale" :tools="tools" :trigger-error="triggerError" :consent-on="consent === 'on'" :consent-ready="consentReady" @notify="(text) => $emit('notify', text)" />
    <MDeveloperPage v-else :locale="locale" :tools="tools" :api-base="apiBase" :omit="['render', 'api', 'env', 'host', 'telemetry']" @disabled="$emit('disabled')" @notify="(text) => $emit('notify', text)" />
  </MPage>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MPage, MDeveloperPage, DevIssues, DevLogs, DevApi, DevFlags, DevBuild, DevTests, DevSimChip, devmodeTexts, reportError } from 'foundation-ui/vue'
import { developerTexts } from 'foundation-ui/developer'
import { buildReport, reportToDocument, sendDocument, loadQueue } from 'foundation-ui/telemetry'
import { sunshine, openExternalUrl } from '../../tauri-adapter.js'
import { desktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'
import { createDeveloperTools } from '../../composables/developerTools.js'
import { ROUTES } from '../../composables/useRouter.js'

// The pages of the 开发者 rail: Issue 与 PR, 实时日志, API 检查器, 功能开关, 构建与组件, 测试工具 and 开发者选项 (the former page of that name, without what has a page of its own now).
// They are reached from the rail while developer mode is on (seven taps on the version) and from 系统 and 关于.
const props = defineProps({
  page: { type: String, default: ROUTES.DEV_ISSUES },
  locale: { type: String, default: 'zh' },
  version: { type: String, default: '' },
  consent: { type: String, default: 'on' },
  /** whether the answer about telemetry is known (the cache of this window, or the host's answer) */
  consentReady: { type: Boolean, default: true },
  errorContext: { type: Function, default: null },
  /** show the surfaces of a host that is away: '' (back to normal) | 'offline' | 'stopped' | 'unresponsive' */
  simulateHost: { type: Function, default: null },
})
defineEmits(['disabled', 'notify'])

const D = computed(() => devmodeTexts(props.locale))
const title = computed(() => ({ [ROUTES.DEV_ISSUES]: D.value.pages.issues, [ROUTES.DEV_LOGS]: D.value.pages.logs, [ROUTES.DEV_API]: D.value.pages.api, [ROUTES.DEV_FLAGS]: D.value.pages.flags, [ROUTES.DEV_BUILD]: D.value.pages.build, [ROUTES.DEV_TESTS]: D.value.pages.tests }[props.page] || developerTexts(props.locale).title))
const wide = computed(() => props.page === ROUTES.DEV_ISSUES || props.page === ROUTES.DEV_LOGS || props.page === ROUTES.DEV_API)
const lead = computed(() => '')

const apiBase = ref('')
onMounted(async () => { apiBase.value = String((await sunshine.getProxyUrl().catch(() => '')) || '').replace(/\/$/, '') })
const openUrl = (url) => { openExternalUrl(url).catch(() => {}) }
const triggerError = () => { reportError(new Error('Developer page: a test error to look at the error dialog'), { kind: 'error', blocking: true }) }

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
