<template>
  <MPage :title="T.title">
    <MNetworkOverview :report="report" :status="status" :locale="String(locale)" :copy="copyText" @recheck="load(true)" @fix="onFix" />
  </MPage>
</template>

<script setup>
// 系统 › 网络概览: the host's GET /api/network/overview through the panel's proxy. The host asks the STUN servers itself (and only when this page asks); a host without the API
// answers 404 and the page says it is not available. A fix the page cannot do itself opens 串流设置 › 网络, where the settings of each fix are.
import { computed, onMounted, ref } from 'vue'
import { MPage, MNetworkOverview, normalizeOverview, networkOverviewTexts } from 'foundation-ui/vue'
import { sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['open-network'])
const { locale } = useI18n()
const T = computed(() => networkOverviewTexts(String(locale.value)))
const report = ref(null)
const status = ref('idle')
let request = 0

async function load(refresh = false) {
  const id = ++request
  status.value = 'loading'
  try {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const response = await fetch(`${base}/api/network/overview${refresh ? '?refresh=1' : ''}`)
    if (id !== request) return
    if (response.status === 404) { status.value = 'unavailable'; return }
    const parsed = response.ok ? normalizeOverview(await response.json().catch(() => null)) : null
    if (id !== request) return
    if (parsed) { report.value = parsed; status.value = 'ready' }
    else status.value = response.ok ? 'unavailable' : 'error'
  } catch {
    if (id === request) status.value = 'error'
  }
}

const copyText = (text) => globalThis.navigator?.clipboard?.writeText(text)
const onFix = () => emit('open-network')
onMounted(() => { void load() })
</script>
