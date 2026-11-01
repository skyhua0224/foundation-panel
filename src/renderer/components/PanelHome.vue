<script setup>
import { computed, ref } from 'vue'
import { MPage, MIcon, MIconButton, MDialog, MButton, MHomeDash, MSupportSheet, buildHomeModel, homeTexts } from 'foundation-ui/vue'
import { usePanelHome } from '../composables/usePanelHome.js'
import { useGamepadType } from '../composables/useGamepadType.js'
import { SPONSORS } from '../config/sponsors.js'
import { openExternalUrl, vdd as vddApi } from '../tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'

const props = defineProps({
  runtime: { type: Object, required: true },
  updateAvailable: { type: Boolean, default: false },
})
const emit = defineEmits(['open', 'webui-navigate'])
const { t, locale } = useI18n()
const h = computed(() => t.value.m3e.home)
const { state, proxyUrl, session, uptimeMs, perfSession, restart, stopAll, openTool } = usePanelHome()
const pad = useGamepadType()

const dashTexts = computed(() => ({ done: h.value.done, reset: h.value.reset, earlier: h.value.earlier, later: h.value.later, addTitle: h.value.addTitle, nothing: h.value.nothing, hide: h.value.remove }))
const edit = ref(false)
const sponsorOpen = ref(false)
const confirmRestart = ref(false)
const confirmStop = ref(false)
const busy = ref(false)
const stripBusy = ref(false)

const dsLabel = computed(() => { const s = props.runtime.ds?.state; return s ? (t.value.dualSense.status.states[s] || s) : '' })
const micText = computed(() => ({ active: h.value.micOn, streaming: h.value.micOn, capturing: h.value.micOn, idle: h.value.ready, waiting: h.value.ready }[props.runtime.mic?.state] ?? ''))
const modeLabel = computed(() => pad.modeOptions.value.find((o) => o.value === (pad.selectedMode.value || 'auto'))?.label || '')
const vddKeep = computed(() => state.config?.vdd_keep_enabled === 'enabled')
const vddReady = computed(() => !!state.vdd && (state.vdd.installed !== false))

const coverOf = (a) => {
  const p = a['image-path']
  if (!p || /[/\\]/.test(p) || !proxyUrl.value) return ''
  return p === 'desktop' ? `${proxyUrl.value}/boxart/desktop.png` : `${proxyUrl.value}/boxart/${encodeURIComponent(p)}`
}

const model = computed(() => {
  const s = session.value
  const sys = state.sys.cpu !== null || state.sys.gpu !== null || state.memory?.system_total_memory ? {
    cpu: state.sys.cpu, gpu: state.sys.gpu, gpuName: state.gpus[0] || '',
    memTotal: state.memory?.system_total_memory || 0, memAvail: state.memory?.system_available_memory || 0, memProcess: state.memory?.total_working_set,
  } : null
  const base = buildHomeModel({
    host: { name: state.config?.sunshine_name || 'Sunshine', version: state.version, uptimeMs: uptimeMs.value, running: state.loaded },
    session: s ? { ...s, fpsMeasured: perfSession.value?.frame_rates?.encoded?.fps ?? perfSession.value?.host_latency?.recent_fps, latencyMs: perfSession.value?.host_latency?.avg_ms } : null,
    series: state.series, sys, encoder: state.config?.active_encoder || '',
    pads: { text: [modeLabel.value, dsLabel.value].filter(Boolean).join(' · ') || '–' },
    mic: micText.value ? { text: micText.value } : null,
    clients: state.clients ? state.clients.map((c) => ({ ...c, icon: 'devices' })) : null,
    apps: state.apps ? state.apps.map((a) => ({ id: a.name, name: a.name, cover: coverOf(a) })) : null,
    logs: state.errorLog, recent: state.recent, busy: busy.value,
  }, homeTexts(locale.value))
  base.strip = {
    label: h.value.gamepadType, icon: 'sports_esports', value: pad.selectedMode.value || 'auto', busy: stripBusy.value || !!pad.busyKey.value,
    options: pad.modeOptions.value.map((o) => ({ value: o.value, label: h.value.pad[o.value] || o.label })),
    switches: vddReady.value ? [{ id: 'vdd-keep', label: h.value.vddKeep, sub: h.value.vddKeepSub, value: vddKeep.value, busy: stripBusy.value }] : [],
  }
  return base
})

async function onStripGroup(v) { pad.selectedMode.value = v }
async function onStripSwitch({ id, value }) {
  if (id !== 'vdd-keep') return
  stripBusy.value = true
  try { const r = await vddApi.setKeepEnabled(value); if (r?.success && state.config) state.config = { ...state.config, vdd_keep_enabled: value ? 'enabled' : 'disabled' } } finally { stripBusy.value = false }
}
function onHero() { if (session.value) confirmStop.value = true; else confirmRestart.value = true }
const doRestart = async () => { confirmRestart.value = false; busy.value = true; try { await restart() } finally { busy.value = false } }
const doStop = async () => { confirmStop.value = false; busy.value = true; try { await stopAll() } finally { busy.value = false } }
const onTableAction = () => emit('open', 'clients')
const onLink = (to) => emit('open', to)

const supportSections = computed(() => [
  { title: h.value.sponsorTitle, items: SPONSORS.map((s) => ({ id: s.id, icon: 'favorite', title: s.label, url: s.url })) },
  { items: [
    { id: 'star', icon: 'star', title: h.value.star, url: 'https://github.com/AlkaidLab/foundation-sunshine' },
    { id: 'qq', icon: 'groups', title: h.value.qq, url: 'https://qm.qq.com/q/AfMQoyKrkc' },
  ] },
])
</script>

<template>
  <MPage :title="t.m3e.nav.overview" wide>
    <template #actions>
      <button type="button" class="m-dpill m-dpill--head" @click="emit('open', 'downloads')"><MIcon name="download" /><span>{{ h.download }}</span></button>
      <MIconButton icon="favorite" :label="h.sponsor" @click="sponsorOpen = true" />
      <MIconButton :icon="edit ? 'check' : 'edit'" :label="edit ? h.done : h.edit" @click="edit = !edit" />
    </template>
    <MHomeDash v-model:edit="edit" :model="model" storage-key="panel-home" :texts="dashTexts"
      @hero-action="onHero" @strip-group="onStripGroup" @strip-switch="onStripSwitch" @table-action="onTableAction" @link="onLink"
      @table-row="() => emit('open', 'clients')" @app="() => emit('open', 'apps')" />
    <MSupportSheet v-model="sponsorOpen" :title="h.sponsor" :sections="supportSections" :close-label="h.done" @open="(it) => openExternalUrl(it.url)" />
    <MDialog v-model="confirmRestart" :title="h.restartTitle" icon="restart_alt" tone="info">
      <p>{{ h.restartText }}</p>
      <template #actions><MButton variant="text" @click="confirmRestart = false">{{ h.cancel }}</MButton><MButton @click="doRestart">{{ h.restart }}</MButton></template>
    </MDialog>
    <MDialog v-model="confirmStop" :title="h.stopTitle" icon="stop" tone="danger">
      <p>{{ h.stopText }}</p>
      <template #actions><MButton variant="text" @click="confirmStop = false">{{ h.cancel }}</MButton><MButton variant="danger" @click="doStop">{{ h.stop }}</MButton></template>
    </MDialog>
  </MPage>
</template>
