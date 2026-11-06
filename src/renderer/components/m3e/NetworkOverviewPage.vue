<template>
  <MPage :title="T.title">
    <MNetworkOverview :report="state.view.report" :status="state.view.status" :locale="String(locale)" :copy="copyText" @recheck="state.refresh(true)" @fix="onFix" />
  </MPage>
</template>

<script setup>
// 系统 › 网络概览: the host's GET /api/network/overview through the panel's proxy (foundation-ui createNetworkOverview, the same state as the Web UI page). The host asks the
// STUN servers only when this page opens or 重新检测 is pressed and keeps the result 10 minutes; a host without the API answers 404 and the page says it is not available.
// A fix the page cannot do itself opens 串流设置 › 网络, where the settings of each fix are.
import { computed, onMounted } from 'vue'
import { MPage, MNetworkOverview, createNetworkOverview, networkOverviewTexts } from 'foundation-ui/vue'
import { sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['open-network'])
const { locale } = useI18n()
const T = computed(() => networkOverviewTexts(String(locale.value)))
// a new mount reads the host's cache again (10 minutes): no second state to keep in step
const state = createNetworkOverview({
  request: async (path) => {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const response = await fetch(`${base}${path}`)
    return { ok: response.ok, status: response.status, json: await response.json().catch(() => null) }
  },
})
const copyText = (text) => globalThis.navigator?.clipboard?.writeText(text)
const onFix = () => emit('open-network')
onMounted(() => { void state.refresh() })
</script>
