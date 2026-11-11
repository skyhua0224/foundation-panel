<template>
  <MServiceStatus :status="status" :locale="locale" :busy="busy" :refreshing="refreshing" :read-at="readAt" @refresh="refresh()" @action="runAction" @start-type="setStartType" />
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { MServiceStatus, useNotify } from 'foundation-ui/vue'
import { serviceTexts } from 'foundation-ui/service'
import { sunshineService } from '../../tauri-adapter.js'

// 系统 › Sunshine 服务: the shared page with the panel's back end (src-tauri/src/sunshine_service). The tray's start-up item and this page
// change the startup type through the same command; every change runs in the elevated helper after the page's confirmation, and the
// back end refuses Manual, Disabled, stop and restart without the acknowledgements the confirmations give.
const props = defineProps({ locale: { type: String, default: 'zh' } })
const status = ref(null)
const busy = ref('')
const refreshing = ref(false)
const readAt = ref(0)
const T = () => serviceTexts(props.locale)

// `quiet`: the background read shows no spinner and no error snack (the page keeps the last state)
async function refresh(quiet = false) {
  if (!quiet) refreshing.value = true
  try { status.value = await sunshineService.status(); readAt.value = Date.now() } catch (e) { if (!quiet) useNotify().snack(`${T().failed} ${e?.message || e || ''}`.trim()) } finally { if (!quiet) refreshing.value = false }
}
async function change(kind, call) {
  busy.value = kind
  try { await call(); useNotify().snack(T().done) } catch (e) { useNotify().snack(`${T().failed} ${e?.message || e || ''}`.trim()) } finally { busy.value = ''; await refresh(true) }
}
const runAction = (kind, acknowledgeLiveStream) => change(kind, () => sunshineService.control(kind, acknowledgeLiveStream))
const setStartType = (type, acknowledged) => change('start-type', () => sunshineService.setStartType(type, acknowledged))

// the state can change outside the page (the tray, services.msc, a crash): read it again every few seconds while the page is open
let timer = 0
onMounted(() => { refresh(); timer = setInterval(() => { if (!busy.value) refresh(true) }, 5000) })
onBeforeUnmount(() => clearInterval(timer))
</script>
