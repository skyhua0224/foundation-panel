<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { micIsReady, padIsReady, MPage, MIcon, MIconButton, MDialog, MButton, MHomeDash, MSupportSheet, MAnnouncementStrip, MAnnouncementSheet, useAnnouncements, useGuideCard, buildHomeModel, buildStartSteps, buildCards, guideCardData, downloadCardData, sponsorCardData, GUIDE_TARGET, homeTexts } from 'foundation-ui/vue'
import { buildCatalogue, buildCatalogueDetails, useJourneyHome, createHistory, createHttpVddAdapter, createPairQr, createVddStore, saveSettingKey, vddStatusOf } from 'foundation-ui/vue'
import QRCode from 'qrcode'
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

const dashTexts = computed(() => ({ ...homeTexts(locale.value), done: h.value.done, hide: h.value.remove }))
const announcements = useAnnouncements({ locale, version: computed(() => state.version) })
const announceOpen = ref(false)
const announceTexts = computed(() => { const x = homeTexts(locale.value); return { open: x.annOpen, dismiss: x.annDismiss, all: x.annAll, read: x.annRead, none: x.annNone } })
const T = computed(() => homeTexts(locale.value))
const SEEN_KEY = 'foundation-ui.guide.panel-home.sessionSeen'
const sessionSeen = ref(localStorage.getItem(SEEN_KEY) === '1')
watch(() => !!session.value || (state.recent?.length || 0) > 0, (v) => { if (v && !sessionSeen.value) { sessionSeen.value = true; try { localStorage.setItem(SEEN_KEY, '1') } catch { /* storage unavailable */ } } }, { immediate: true })
const guide = useGuideCard('panel-home', () => (state.loaded ? buildStartSteps(T.value, { paired: state.clientsTotal, sessionSeen: sessionSeen.value, apps: state.apps }) : []))
const guideStepGo = (step) => { const to = GUIDE_TARGET[step.id]; emit('open', to) }
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
const vddReady = computed(() => !!state.vdd && (state.vdd.installed !== false))

const coverOf = (a) => {
  const p = a['image-path']
  if (!p || /[/\\]/.test(p) || !proxyUrl.value) return ''
  return p === 'desktop' ? `${proxyUrl.value}/boxart/desktop.png` : `${proxyUrl.value}/boxart/${encodeURIComponent(p)}`
}

// the virtual display: its driver state from the panel's vdd state, the largest configured mode from its settings
const vddInput = computed(() => {
  const v = state.vdd
  if (!v) return null
  const set = state.vddSettings
  const modes = (set?.resolutions?.resolution || []).filter((r) => r.width && r.height).sort((a, b2) => b2.width * b2.height - a.width * a.height)
  const rates = (set?.global?.g_refresh_rate || []).map(Number).filter(Number.isFinite)
  return { state: v.installed === false ? 'missing' : (v.running || v.state === 'ready') ? 'ready' : 'idle', text: '', width: modes[0]?.width, height: modes[0]?.height, hz: rates.length ? Math.max(...rates) : undefined, hdr: set ? !!set.colour?.HDRPlus : undefined }
})
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
    codecModes: state.config && state.config.hevc_mode !== undefined ? { hevc: state.config.hevc_mode, av1: state.config.av1_mode } : null,
    pads: { text: [modeLabel.value, dsLabel.value].filter(Boolean).join(' · ') || '–', word: dsLabel.value || modeLabel.value, ready: padIsReady(props.runtime.ds?.state) },
    mic: micText.value ? { text: micText.value, ready: micIsReady({ state: props.runtime.mic?.state }) } : null,
    clients: state.clients ? state.clients.map((c) => ({ ...c, icon: 'devices' })) : null,
    apps: state.apps ? state.apps.map((a) => ({ id: a.name, name: a.name, cover: coverOf(a) })) : null,
    logs: state.errorLog, recent: state.recent, busy: busy.value,
    usb: props.runtime.usb?.attached_devices ? props.runtime.usb.attached_devices.length : null,
    vdd: vddInput.value,
  }, homeTexts(locale.value))
  base.strip = {
    label: h.value.gamepadType, icon: 'sports_esports', value: pad.selectedMode.value || 'auto', busy: stripBusy.value || !!pad.busyKey.value,
    options: pad.modeOptions.value.map((o) => ({ value: o.value, label: h.value.pad[o.value] || o.label })),
    switches: [
      ...(micConfigured.value ? [{ id: 'mic', label: h.value.mic, sub: micEnabled.value ? (micText.value || h.value.ready) : h.value.off, value: micEnabled.value, busy: stripBusy.value }] : []),
    ],
  }
  return base
})
// history this interface keeps itself (the host has none): finished sessions and minutes per day; the curves reuse the live series
const history = createHistory(localStorage, 'foundation-ui.history.panel-home')
const historyTick = ref(0)
watch(session, (s) => { if (history.observe(s ? { ...s, fpsMeasured: perfSession.value?.frame_rates?.encoded?.fps, latencyP95Ms: perfSession.value?.host_latency?.p95_ms } : null)) historyTick.value += 1 }, { immediate: true })
// one sample of every live number every three seconds, kept for an hour (the sheets' time ranges read it)
const SAMPLE_SEC = 3
const sampleAll = () => {
  const s = session.value, p = perfSession.value
  history.sample('cpu', state.sys.cpu); history.sample('gpu', state.sys.gpu)
  if (state.memory?.system_total_memory) history.sample('mem', ((state.memory.system_total_memory - (state.memory.system_available_memory || 0)) / state.memory.system_total_memory) * 100)
  if (s) { history.sample('bitrate', s.bitrate); history.sample('fps', p?.frame_rates?.encoded?.fps ?? s.fps); history.sample('latency', p?.host_latency?.avg_ms); history.sample('loss', s.packet_loss); history.sample('dropped', s.dropped_frames) } else for (const k of ['bitrate', 'fps', 'latency', 'loss', 'dropped']) history.clear(k)
  historyTick.value += 1
}
let sampler = 0
onMounted(() => { sampleAll(); sampler = setInterval(sampleAll, SAMPLE_SEC * 1000) })
onBeforeUnmount(() => clearInterval(sampler))
// the shared virtual display control: the same store the settings use, here over the host API through the proxy and the tray commands
const vddAdapter = createHttpVddAdapter({
  fetchJson: async (path, init) => (await fetch(`${proxyUrl.value}${path}`, init)).json(),
  setProtected: async (key, on) => { const r = key === 'vdd_keep_enabled' ? await vddApi.setKeepEnabled(on) : await vddApi.setHeadlessCreateEnabled(on); if (r && r.success === false) throw new Error(r.error || 'failed') },
  status: async () => vddStatusOf(state.vdd),
})
const vddStore = createVddStore(vddAdapter, { autosave: true })
onMounted(async () => { try { await vddAdapter.load() } catch { /* the card then shows the defaults */ } vddStore.refresh() })
const catalogueInput = computed(() => {
  void historyTick.value
  const s = session.value
  const p = perfSession.value
  return {
    sampleSec: SAMPLE_SEC, vddStore: state.vdd && state.vdd.installed !== false ? vddStore : null,
    host: { version: state.version, update: props.updateAvailable ? h.value.update : '' },
    session: s ? { ...s, fpsMeasured: p?.frame_rates?.encoded?.fps, latencyMs: p?.host_latency?.avg_ms, latencyP95Ms: p?.host_latency?.p95_ms, uptimeMs: p?.uptime_ms ?? s.uptimeMs } : null,
    history: { samples: history.samples, sessions: history.sessions, days: history.days, last: () => history.last() },
    sys: model.value.info.some((i) => i.id === 'memory') && state.memory ? { memTotal: state.memory.system_total_memory, memAvail: state.memory.system_available_memory } : null,
    config: state.config, logText: state.logText, logs: state.errorLog, clients: state.clients ? state.clients.map((c) => ({ ...c, icon: 'devices' })) : null,
    apps: state.apps ? state.apps.map((a) => ({ id: a.name, name: a.name, cover: coverOf(a) })) : null,
    codecModes: state.config && state.config.hevc_mode !== undefined ? { hevc: state.config.hevc_mode, av1: state.config.av1_mode } : null,
    pads: { text: [modeLabel.value, dsLabel.value].filter(Boolean).join(' · ') || '–' }, usb: props.runtime.usb?.attached_devices ? props.runtime.usb.attached_devices.length : null,
    display: vddInput.value && vddInput.value.hdr !== undefined ? { hdr: vddInput.value.hdr } : null,
    temps: state.hw.available ? { cpu: state.hw.cpuTemp, gpu: state.hw.gpuTemp } : null,
    pairQr: pairQr.view,
    announcements: announcements.list.value.map((a) => ({ id: a.id, title: a.title || a.text || '' })),
  }
})
const catalogue = computed(() => buildCatalogue(catalogueInput.value, T.value).data)
const allDetails = computed(() => ({ ...model.value.details, ...buildCatalogueDetails(catalogueInput.value, T.value) }))
const isNew = computed(() => guide.visible.value)
const cards = computed(() => buildCards(model.value, { texts: T.value, catalogue: catalogue.value, guide: guideCardData(T.value, guide.steps.value, guide.visible.value), download: downloadCardData(T.value), sponsor: sponsorCardData(T.value, SPONSORS), journey: journey.data.value }))
const dashCtx = computed(() => ({ live: !!session.value, isNew: isNew.value, ...journey.ctx.value }))

function onAction(a) {
  switch (a.type) {
    case 'hero': onHero(); break
    case 'group': onStripGroup(a.value); break
    case 'switch': onStripSwitch(a.value); break
    case 'table-action': emit('open', 'pair'); break
    case 'pair-pin': pairPin(a.value?.pin); break
    case 'pair-qr': emit('open', 'pair:qr'); break
    case 'pair-qr-begin': pairQr.begin(); break
    case 'pair-qr-stop': pairQr.stop(); break
    case 'table-row': onTableRow(a.value); break
    case 'app': emit('open', 'apps'); break
    case 'open-url': openExternalUrl(a.value); break
    case 'add-app': emit('open', 'apps:add'); break
    case 'open': emit('open', a.value); break
    case 'detail-external': break
    case 'guide-go': guideStepGo(a.value); break
    case 'guide-skip': guide.toggle(a.value); break
    case 'download': emit('open', `downloads:${a.value}`); break
    case 'link': openExternalUrl(a.value); break
    case 'setting': saveSetting(a.value); break
    case 'sponsor': { const target = SPONSORS.find((x) => x.id === a.value); if (a.value === 'star') openExternalUrl('https://github.com/AlkaidLab/foundation-sunshine'); else if (target) openExternalUrl(target.url); else sponsorOpen.value = true; break }
    default: break
  }
}
// a control card saves its setting the way the settings do: read the configuration, change the key, write it back
const hostApi = {
  getJson: async (path) => (await fetch(`${proxyUrl.value}${path}`)).json(),
  postJson: async (path, body) => (await fetch(`${proxyUrl.value}${path}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json(),
}
// the QR code of the pairing card: a pairing window is open on the host only while the card shows it (the card asks and gives it back)
const pairQr = createPairQr({
  start: async () => { const d = await hostApi.postJson('/api/qr-pair', {}); if (String(d.status) !== 'true') throw new Error(d.error || 'qr'); return d },
  poll: () => hostApi.getJson('/api/qr-pair'),
  cancel: () => hostApi.postJson('/api/qr-pair/cancel', {}),
  render: (url) => QRCode.toDataURL(url, { width: 280, margin: 2, color: { dark: '#000000', light: '#ffffff' } }),
})
onBeforeUnmount(() => pairQr.stop())
// the PIN typed on the pairing card goes to the host like the pairing sheet sends it
async function pairPin(pin) {
  if (!/^\d{4}$/.test(String(pin || ''))) return
  try { const data = await hostApi.postJson('/api/pin', { pin: String(pin), name: dashTexts.value.pairDefaultName }); say(String(data.status) === 'true' ? dashTexts.value.pairSent : dashTexts.value.pairFailed) } catch { say(dashTexts.value.pairFailed) }
// the connect journey card: on by default while no client is paired (choose the device, install, pair, done)
const journey = useJourneyHome({ storageKey: 'panel-home', count: () => (state.clients ? state.clients.length : null), latest: () => (state.clients?.length ? state.clients[state.clients.length - 1].name : ''), locale: () => locale.value, qr: (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules, pairQr: pairQr.view })
}
async function saveSetting({ key, value }) { const r = await saveSettingKey(hostApi, key, value); if (r.ok) state.config = r.config }
async function onStripGroup(v) { pad.selectedMode.value = v }
async function onStripSwitch({ id, value }) {
  stripBusy.value = true
  try {
    if (id === 'mic') { const r = await virtualMicrophone.saveBackend(value ? 'auto' : 'disabled'); if (r?.success) micBackend.value = value ? 'auto' : 'disabled' }
  } finally { stripBusy.value = false }
}
function onHero() { if (session.value) confirmStop.value = true; else confirmRestart.value = true }
const doRestart = async () => { confirmRestart.value = false; busy.value = true; try { await restart() } finally { busy.value = false } }
const doStop = async () => { confirmStop.value = false; busy.value = true; try { await stopAll() } finally { busy.value = false } }
const onTableAction = () => emit('open', 'pair')
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
      <MButton v-if="edit" variant="tonal" icon="check" @click="edit = false">{{ h.done }}</MButton>
      <MIconButton v-else icon="edit" :label="h.edit" @click="edit = true" />
    </template>
    <MAnnouncementStrip v-if="announcements.top.value" :item="announcements.top.value" :total="announcements.list.value.length" :texts="announceTexts" @dismiss="announcements.dismiss" @open-url="openExternalUrl" @open-list="announceOpen = true" />
    <MHomeDash v-model:edit="edit" :cards="cards" :details="allDetails" :ctx="dashCtx" storage-key="panel-home" :texts="dashTexts" @action="onAction" />
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
