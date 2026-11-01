<script setup>
import { computed, ref, watch } from 'vue'
import { MPage, MIcon, MIconButton, MDialog, MButton, MHomeDash, MSupportSheet, MAnnouncementStrip, MAnnouncementSheet, MHomeGuide, useAnnouncements, useGuideCard, buildHomeModel, buildStartSteps, guidePlatforms, guideRecommended, homeTexts } from 'foundation-ui/vue'
import { usePanelHome } from '../composables/usePanelHome.js'
import { useGamepadType } from '../composables/useGamepadType.js'
import { SPONSORS } from '../config/sponsors.js'
import { openExternalUrl, vdd as vddApi, virtualMicrophone } from '../tauri-adapter.js'
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
const announcements = useAnnouncements({ locale, version: computed(() => state.version) })
const announceOpen = ref(false)
const announceTexts = computed(() => { const x = homeTexts(locale.value); return { open: x.annOpen, dismiss: x.annDismiss, all: x.annAll, read: x.annRead, none: x.annNone } })
const T = computed(() => homeTexts(locale.value))
const SEEN_KEY = 'foundation-ui.guide.panel-home.sessionSeen'
const sessionSeen = ref(localStorage.getItem(SEEN_KEY) === '1')
watch(() => !!session.value || (state.recent?.length || 0) > 0, (v) => { if (v && !sessionSeen.value) { sessionSeen.value = true; try { localStorage.setItem(SEEN_KEY, '1') } catch { /* storage unavailable */ } } }, { immediate: true })
const guide = useGuideCard('panel-home', () => (state.loaded ? buildStartSteps(T.value, { paired: state.clientsTotal, sessionSeen: sessionSeen.value, apps: state.apps }) : []))
const guideTexts = computed(() => ({ title: T.value.guideTitle, sub: T.value.guideSub, continue: T.value.guideContinue, dismiss: T.value.guideDismiss, downloadTitle: T.value.downloadTitle }))
const STEP_TARGET = { download: 'downloads', pair: 'clients', game: 'apps' }
const onGuideStep = (s) => { if (STEP_TARGET[s.id]) emit('open', STEP_TARGET[s.id]); else guide.toggle(s.id) }
const edit = ref(false)
const sponsorOpen = ref(false)
const confirmRestart = ref(false)
const confirmStop = ref(false)
const busy = ref(false)
const stripBusy = ref(false)

const dsLabel = computed(() => { const s = props.runtime.ds?.state; return s ? (t.value.dualSense.status.states[s] || s) : '' })
const micText = computed(() => ({ active: h.value.micOn, streaming: h.value.micOn, capturing: h.value.micOn, idle: h.value.ready, waiting: h.value.ready }[props.runtime.mic?.state] ?? ''))
const modeLabel = computed(() => pad.modeOptions.value.find((o) => o.value === (pad.selectedMode.value || 'auto'))?.label || '')
// the microphone switch: the host's backend choice, saved at once like on the microphone page ('disabled' off, 'auto' on)
const micBackend = ref('')
const micConfigured = computed(() => micBackend.value || props.runtime.mic?.configured_backend || '')
const micEnabled = computed(() => micConfigured.value !== 'disabled')
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
    host: { name: state.config?.sunshine_name || 'Sunshine', version: state.version, uptimeMs: uptimeMs.value, running: state.loaded, update: props.updateAvailable ? h.value.update : '' },
    session: s ? { ...s, fpsMeasured: perfSession.value?.frame_rates?.encoded?.fps ?? perfSession.value?.host_latency?.recent_fps, latencyMs: perfSession.value?.host_latency?.avg_ms } : null,
    series: state.series, sys, encoder: state.config?.active_encoder || '',
    pads: { text: [modeLabel.value, dsLabel.value].filter(Boolean).join(' · ') || '–' },
    mic: micText.value ? { text: micText.value } : null,
    clients: state.clients ? state.clients.map((c) => ({ ...c, icon: 'devices' })) : null,
    apps: state.apps ? state.apps.map((a) => ({ id: a.name, name: a.name, cover: coverOf(a) })) : null,
    logs: state.errorLog, recent: state.recent, busy: busy.value,
    usb: props.runtime.usb?.attached_devices ? props.runtime.usb.attached_devices.length : null,
    display: state.vdd ? { vddText: state.vdd.status_text || (state.vdd.running ? h.value.on : h.value.off), vddTone: state.vdd.running ? 'ok' : '' } : null,
  }, homeTexts(locale.value))
  base.strip = {
    label: h.value.gamepadType, icon: 'sports_esports', value: pad.selectedMode.value || 'auto', busy: stripBusy.value || !!pad.busyKey.value,
    options: pad.modeOptions.value.map((o) => ({ value: o.value, label: h.value.pad[o.value] || o.label })),
    switches: [
      ...(vddReady.value ? [{ id: 'vdd-keep', label: h.value.vddKeep, sub: h.value.vddKeepSub, value: vddKeep.value, busy: stripBusy.value }] : []),
      ...(micConfigured.value ? [{ id: 'mic', label: h.value.mic, sub: micEnabled.value ? (micText.value || h.value.ready) : h.value.off, value: micEnabled.value, busy: stripBusy.value }] : []),
    ],
  }
  return base
})

async function onStripGroup(v) { pad.selectedMode.value = v }
async function onStripSwitch({ id, value }) {
  stripBusy.value = true
  try {
    if (id === 'vdd-keep') { const r = await vddApi.setKeepEnabled(value); if (r?.success && state.config) state.config = { ...state.config, vdd_keep_enabled: value ? 'enabled' : 'disabled' } }
    else if (id === 'mic') { const r = await virtualMicrophone.saveBackend(value ? 'auto' : 'disabled'); if (r?.success) micBackend.value = value ? 'auto' : 'disabled' }
  } finally { stripBusy.value = false }
}
function onHero() { if (session.value) confirmStop.value = true; else confirmRestart.value = true }
const doRestart = async () => { confirmRestart.value = false; busy.value = true; try { await restart() } finally { busy.value = false } }
const doStop = async () => { confirmStop.value = false; busy.value = true; try { await stopAll() } finally { busy.value = false } }
const onTableAction = () => emit('open', 'clients')
const onTableRow = (row) => { if (row.tab === 'errors' || row.tab === 'sessions') openTool('logs'); else emit('open', 'clients') }
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
    <MAnnouncementStrip v-if="announcements.top.value" :item="announcements.top.value" :total="announcements.list.value.length" :texts="announceTexts" @dismiss="announcements.dismiss" @open-url="openExternalUrl" @open-list="announceOpen = true" />
    <MHomeDash v-model:edit="edit" :guide="guide.visible.value" :model="model" storage-key="panel-home" :texts="dashTexts"
      @hero-action="onHero" @strip-group="onStripGroup" @strip-switch="onStripSwitch" @table-action="onTableAction" @link="onLink"
      @table-row="onTableRow" @app="() => emit('open', 'apps')">
      <template #guide>
        <MHomeGuide :steps="guide.steps.value" :platforms="guidePlatforms(T)" :recommended="guideRecommended(T)" :texts="guideTexts" @toggle="guide.toggle" @step="onGuideStep" @continue="emit('open', 'welcome')" @dismiss="guide.dismiss" @download="(id) => emit('open', `downloads:${id}`)" />
      </template>
    </MHomeDash>
    <MAnnouncementSheet v-model="announceOpen" :items="announcements.list.value" :title="homeTexts(locale).annTitle" :texts="announceTexts" :close-label="h.done" @dismiss="announcements.dismiss" @open-url="openExternalUrl" />
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
