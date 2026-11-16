<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { firstImage, micIsReady, padIsReady, MPage, MIcon, MIconButton, MDialog, MButton, MHomeDash, MSupportSheet, coverKnown, heroArtReady, MAnnouncementStrip, MAnnouncementSheet, useAnnouncements, useGuideCard, useNotify, buildHomeModel, buildStartSteps, buildCards, guideCardData, downloadCardData, sponsorCardData, GUIDE_TARGET, homeTexts } from 'foundation-ui/vue'
import { buildCatalogue, buildCatalogueDetails, useJourneyHome, useReleaseAssets, useStarCard, useSessionClock, useSystemInfo, reachProblemOf, registeredComponentStatus, createHistory, createHttpVddAdapter, createPairQr, createVddStore, saveSettingKey, vddStatusOf, recentKeys, rememberRecent, isLoopbackHost, createQueuedCalls, createSerialCalls } from 'foundation-ui/vue'
import QRCode from 'qrcode'
import { qrSvgDataUrl } from '../composables/qrSvg.js'
import { usePanelHome, seen, SETTLE_CAP_MS } from '../composables/usePanelHome.js'
import { useHostReach } from '../composables/useHostReach.js'
import { useGamepadType } from '../composables/useGamepadType.js'
import { SPONSORS } from '../config/sponsors.js'
import { openExternalUrl, sunshine, vdd as vddApi, virtualMicrophone } from '../tauri-adapter.js'
import { hostBase, hostFetch } from '../composables/hostBase.js'
import { useI18n } from '../desktop/i18n/index.js'
import { openBigScreenWithNotice } from '../composables/openBigScreen.js'
import { pipelineAverages } from '../composables/liveSession.js'

const props = defineProps({
  runtime: { type: Object, required: true },
  updateAvailable: { type: Boolean, default: false },
})
const emit = defineEmits(['open', 'webui-navigate'])
const { t, locale } = useI18n()
const h = computed(() => t.value.m3e.home)
const { state, proxyUrl, session: rawSession, uptimeMs, perfSession, restart, stopAll, openTool } = usePanelHome()
// The session the overview is drawn from. It follows the host at once, except for one moment: a stream that STARTS while the page is open is held back until what its tiles are drawn from (the DualSense keys, the cover and
// the hero's colours from it) is known, so the page changes once, with the motion of a change, instead of in four steps. From the kept data of an earlier visit it is the host's from the first frame.
const session = ref(state.settled ? rawSession.value : null)
// the running time of the stream ticks every second from the session's start (a poll of three seconds only corrects it)
const sessionClock = useSessionClock({ live: () => !!session.value, report: () => { const u = perfSession.value?.uptime_ms; return Number.isFinite(u) ? { uptimeMs: u, at: state.perfAt || Date.now() } : null } })
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
    session: s ? { ...s, fpsMeasured: perfSession.value?.frame_rates?.encoded?.fps ?? perfSession.value?.host_latency?.recent_fps, latencyMs: perfSession.value?.host_latency?.avg_ms, ...pipelineAverages(perfSession.value) } : null,
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
  if (s) { history.sample('bitrate', Number.isFinite(s.measured_bitrate_kbps) ? s.measured_bitrate_kbps : s.bitrate); history.sample('fps', p?.frame_rates?.encoded?.fps ?? s.fps); history.sample('latency', p?.host_latency?.avg_ms); history.sample('loss', s.packet_loss); history.sample('dropped', s.dropped_frames) } else for (const k of ['bitrate', 'fps', 'latency', 'loss', 'dropped']) history.clear(k)
  historyTick.value += 1
}
let sampler = 0
onMounted(() => { sampleAll(); sampler = setInterval(sampleAll, SAMPLE_SEC * 1000) })
onBeforeUnmount(() => clearInterval(sampler))
// the address of the host API: a request waits for it (hostBase.js) instead of going to the panel's own origin with an empty prefix
const base = () => hostBase(() => proxyUrl.value, () => sunshine.getProxyUrl())
// the shared virtual display control: the same store the settings use, here over the host API through the proxy and the tray commands
const vddAdapter = createHttpVddAdapter({
  fetchJson: async (path, init) => (await hostFetch(base, path, init)).json(),
  setProtected: async (key, on) => { const r = key === 'vdd_keep_enabled' ? await vddApi.setKeepEnabled(on) : await vddApi.setHeadlessCreateEnabled(on); if (r && r.success === false) throw new Error(r.error || 'failed') },
  status: async () => vddStatusOf(state.vdd),
})
const vddStore = createVddStore(vddAdapter, { autosave: true })
onMounted(async () => { try { await vddAdapter.load() } catch { /* the card then shows the defaults */ } vddStore.refresh() })
const sysJson = async (path) => { const r = await hostFetch(base, path); if (!r.ok) throw Object.assign(new Error(`HTTP ${r.status}`), { status: r.status }); return r.json() }
const nics = useSystemInfo({ fetchJson: sysJson })
// the system information is kept between visits too (the network and system tiles, the Wi-Fi and slow-link signals of the smart plan come from it): a visit starts from it and the read replaces it in place
if (seen.nics) nics.info.value = seen.nics
watch(nics.info, (v) => { if (v) seen.nics = v })
const hostReach = useHostReach()
const reachProblem = computed(() => reachProblemOf(hostReach.view.report))
onMounted(() => { void hostReach.refresh(); nics.start() })
const catalogueInput = computed(() => {
  void historyTick.value
  const s = session.value
  const p = perfSession.value
  return {
    sampleSec: SAMPLE_SEC, vddStore: state.vdd && state.vdd.installed !== false ? vddStore : null,
    reach: hostReach.view.report, reachChecking: hostReach.view.status === 'loading',
    sysinfo: nics.info.value,
    host: { version: state.version, update: props.updateAvailable ? h.value.update : '' },
    session: s ? { ...s, fpsMeasured: p?.frame_rates?.encoded?.fps, latencyMs: p?.host_latency?.avg_ms, latencyP95Ms: p?.host_latency?.p95_ms, ...pipelineAverages(p), uptimeMs: sessionClock.uptimeMs.value ?? p?.uptime_ms ?? s.uptimeMs } : null,
    history: { samples: history.samples, sessions: history.sessions, days: history.days, last: () => history.last() },
    sys: model.value.info.some((i) => i.id === 'memory') && state.memory ? { memTotal: state.memory.system_total_memory, memAvail: state.memory.system_available_memory } : null,
    recent: recentKeys.value, canRuntime: canRuntime.value, ds5Keys: ds5Keys.value,
    config: state.config, logText: state.logText, logs: state.errorLog, clients: state.clients ? state.clients.map((c) => ({ ...c, icon: 'devices' })) : null,
    apps: state.apps ? state.apps.map((a) => ({ id: a.name, name: a.name, cover: coverOf(a) })) : null,
    codecModes: state.config && state.config.hevc_mode !== undefined ? { hevc: state.config.hevc_mode, av1: state.config.av1_mode } : null,
    pads: { text: [modeLabel.value, dsLabel.value].filter(Boolean).join(' · ') || '–' }, usb: props.runtime.usb?.attached_devices ? props.runtime.usb.attached_devices.length : null,
    display: vddInput.value && vddInput.value.hdr !== undefined ? { hdr: vddInput.value.hdr } : null,
    temps: state.hw.available ? { cpu: state.hw.cpuTemp, gpu: state.hw.gpuTemp } : null,
    pairQr: pairQr.view,
    // the panel runs on the host: the big screen opens on this machine
    canOpenDesktop: true,
    announcements: announcements.list.value.map((a) => ({ id: a.id, title: a.title || a.text || '', category: a.category, banner: a.banner || firstImage(a.body), date: dateOfPost(a.published), read: a.read || a.snoozed, hasText: !!a.body, url: a.url || '' })),
  }
})
const catalogue = computed(() => buildCatalogue(catalogueInput.value, T.value).data)
const allDetails = computed(() => ({ ...model.value.details, ...buildCatalogueDetails(catalogueInput.value, T.value) }))
const isNew = computed(() => guide.visible.value)
const cards = computed(() => buildCards(model.value, { texts: T.value, catalogue: catalogue.value, guide: guideCardData(T.value, guide.steps.value, guide.visible.value), download: downloadCardData(T.value, { locale: locale.value, assetsOf: releases.assetsOf, versionOf: releases.versionOf, qr: (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules }), sponsor: sponsorCardData(T.value, SPONSORS), journey: journey.data.value, sysinfo: nics.info.value, temps: state.hw.available ? { cpu: state.hw.cpuTemp, gpu: state.hw.gpuTemp } : null }))
const compRows = () => registeredComponentStatus()?.rows?.value || []
const smartSignals = computed(() => ({
  pairedCount: state.clients ? state.clients.length : null, streaming: !!session.value, lastStreamAt: (void historyTick.value, history.last()?.end || 0), // ms since epoch of the end of the last finished session this interface saw (0: none): the smart overview's after-stream scenario runs 30 minutes from it
  problems: 0, problemComponents: compRows().filter((r) => r.state === 'error' || r.state === 'warn').map((r) => r.id),
  recentErrors: (state.errorLog?.rows || state.errorLog || []).length || 0, isNew: isNew.value,
  wifiHost: nics.defaultNic.value?.type === 'wifi', linkSlowMbps: nics.defaultNic.value?.link_speed_mbps && nics.defaultNic.value.link_speed_mbps < 1000 ? nics.defaultNic.value.link_speed_mbps : 0,
  reachBad: reachProblem.value, updateAvailable: props.updateAvailable,
}))
// the overview is drawn (not its skeleton) once what it is drawn from is known: the session and the clients of the host, the DualSense keys of a running stream, the covers the tiles take a class from
// (has-art) and the colours the host card takes from the game's cover. From the kept data of an earlier visit that is the first frame; a first visit waits for the reads (they go out together) and for at most
// SETTLE_CAP_MS, so a host that answers slowly does not hold the page. The system information is not waited for: it only adds a tile when the link is bad, which then comes in with the motion of a change.
// The covers are read from the catalogue's cards, which are built further down: the computeds below are only read once the page is drawn.
const ds5Keys = ref(seen.ds5Keys || [])
const ds5Known = ref(!!seen.ds5Keys)
const capped = ref(false)
let capTimer = 0
onMounted(() => { capTimer = setTimeout(() => { capped.value = true }, SETTLE_CAP_MS) })
onBeforeUnmount(() => clearTimeout(capTimer))
const liveArt = computed(() => { const app = (state.apps || []).find((a) => a.name === rawSession.value?.app_name); return app ? coverOf(app) : '' })
const startReady = computed(() => ds5Known.value && coverKnown(liveArt.value) && heroArtReady(liveArt.value))
const pending = computed(() => !(capped.value || (state.settled && (!rawSession.value || startReady.value) && coverKnown(catalogue.value['last-session']?.cover || ''))))
let startTimer = 0
watch([rawSession, startReady], () => {
  const raw = rawSession.value
  clearTimeout(startTimer)
  if (!raw || session.value || startReady.value) { session.value = raw; return }
  startTimer = setTimeout(() => { session.value = rawSession.value }, SETTLE_CAP_MS)
}, { immediate: true })
onBeforeUnmount(() => clearTimeout(startTimer))
const dashCtx = computed(() => ({ pending: pending.value, live: !!session.value, isNew: isNew.value, ...journey.ctx.value, star: star.visible.value, starNames: (state.clients || []).map((c) => c.name), clientList: (state.clients || []).map((c) => ({ name: c.name, uuid: c.uuid, family: c.family, official: c.official, os: c.os, version: c.version, last_seen: c.last_seen })), sessionClient: session.value ? { uuid: session.value.client_cert_uuid || session.value.uuid || session.value.client_uuid || '', name: session.value.client_name || '', family: session.value.client_family || '', os: session.value.client_os || '', official: session.value.client_official } : null, signals: smartSignals.value }))

// the date of a post in the 资讯 tile, YYYY-MM-DD
const dateOfPost = (at) => { if (!at) return ''; const d = new Date(at); const p = (n) => String(n).padStart(2, '0'); return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}` }
function onAction(a) {
  switch (a.type) {
    case 'news-open': emit('open', `announcement:${a.value?.id}`); break
    case 'hero': onHero(); break
    case 'group': onStripGroup(a.value); break
    case 'switch': onStripSwitch(a.value); break
    case 'table-action': emit('open', 'pair'); break
    case 'pair-pin': pairPin(a.value?.pin, a.value?.name); break
    case 'pair-qr': emit('open', 'pair:qr'); break
    case 'star-dismiss': star.dismiss(); break
    case 'address-save': saveSetting({ key: 'external_endpoints', value: a.value?.value ?? '' }).then(() => saveSetting({ key: 'external_ip', value: '' })).then(() => hostReach.refresh(true)).catch(() => {}); break
    case 'reach-check': hostReach.refresh(true); break
    case 'pin-open': emit('open', `setting:${a.value?.key}`); break
    case 'detail-external': emit('open', 'check-update'); break
    case 'open-url': openExternalUrl(a.value); break
    case 'open-desktop': void openBigScreenWithNotice((m) => notify.snack(m), t.value.sidebar.openBigScreenFailed); break
    case 'pair-qr-begin': pairQr.begin(); break
    case 'pair-qr-stop': pairQr.stop(); break
    case 'table-row': onTableRow(a.value); break
    case 'app': emit('open', 'apps'); break
    case 'add-app': emit('open', 'apps:add'); break
    case 'scan-games': emit('open', 'apps:scan'); break
    case 'open': emit('open', a.value); break
    case 'guide-go': guideStepGo(a.value); break
    case 'guide-skip': guide.toggle(a.value); break
    case 'download': emit('open', `downloads:${a.value}`); break
    case 'link': openExternalUrl(a.value); break
    case 'setting': saveSetting(a.value); break
    case 'runtime-bitrate': runtimeBitrate(a.value); break
    case 'ds5-tune': ds5Tune(a.value); break
    case 'sponsor': { const target = SPONSORS.find((x) => x.id === a.value); if (a.value === 'star') openExternalUrl('https://github.com/AlkaidLab/foundation-sunshine'); else if (target) openExternalUrl(target.url); else sponsorOpen.value = true; break }
    default: break
  }
}
// a control card saves its setting the way the settings do: read the configuration, change the key, write it back
const hostApi = {
  getJson: async (path) => (await hostFetch(base, path)).json(),
  postJson: async (path, body) => (await hostFetch(base, path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json(),
}
// the QR code of the pairing card: a pairing window is open on the host only while the card shows it (the card asks and gives it back)
const pairQr = createPairQr({
  start: async () => { const d = await hostApi.postJson('/api/qr-pair', {}); if (String(d.status) !== 'true') throw new Error(d.error || 'qr'); return d },
  poll: () => hostApi.getJson('/api/qr-pair'),
  cancel: () => hostApi.postJson('/api/qr-pair/cancel', {}),
  render: (url) => qrSvgDataUrl(QRCode.create(url, { errorCorrectionLevel: 'M' }).modules),
})
onBeforeUnmount(() => pairQr.stop())
// the connect journey card: on by default while no client is paired (choose the device, install, pair, done)
const releases = useReleaseAssets()
const journey = useJourneyHome({ storageKey: 'panel-home', assetsOf: releases.assetsOf, versionOf: releases.versionOf, count: () => (state.clients ? state.clients.length : null), latest: () => (state.clients?.length ? state.clients[state.clients.length - 1].name : ''), locale: () => locale.value, qr: (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules, pairQr: pairQr.view })
// the star card: due after the first pairing or the first stream; while a stream runs the star of the client in use, otherwise the host and the sponsor entry; 以后再说 hides it until the next stream
const star = useStarCard({ storageKey: 'panel-home', triggered: () => journey.done.value || sessionSeen.value, paired: () => state.clients?.length || 0, streaming: () => !!session.value })
// the panel's snackbars: the one notification host of this window (MNotifyHost in PanelPending); the overview says its results with it, like the Web UI says them with its own snackbar
const notify = useNotify()
const say = (text) => notify.snack(String(text))
// the PIN typed on the pairing card goes to the host like the pairing sheet sends it
async function pairPin(pin, name = '') {
  if (!/^\d{4}$/.test(String(pin || ''))) return
  try { const data = await hostApi.postJson('/api/pin', { pin: String(pin), name: String(name || '').trim() || dashTexts.value.pairDefaultName }); say(String(data.status) === 'true' ? dashTexts.value.pairSent : dashTexts.value.pairFailed) } catch { say(dashTexts.value.pairFailed) }
}
// a control the keyboard repeats (a slider held on an arrow key) posts one command per destination at a time; the value that is sent is always the last one chosen
const saveSetting = createSerialCalls(async ({ key, value, failed }) => { const r = await saveSettingKey(hostApi, key, value); if (r.ok) { state.config = r.config; rememberRecent(key) } else { say(T.value.saveFailed); failed?.() } }, (a) => a.key)
// what acts on a running stream: the host takes the bitrate command from this computer only; the DualSense tuning applies to the controller in use
const canRuntime = computed(() => { try { return !proxyUrl.value || isLoopbackHost(new URL(proxyUrl.value).hostname) } catch { return true } })
async function loadDs5() { try { ds5Keys.value = (await hostApi.getJson('/api/dualsense/tuning'))?.keys || [] } catch { ds5Keys.value = [] } seen.ds5Keys = ds5Keys.value; ds5Known.value = true }
watch(() => !!rawSession.value, (live) => { if (live) loadDs5() }, { immediate: true })
const runtimeBitrate = createQueuedCalls(async ({ client, kbps }) => {
  try { const r = await hostFetch(base, `/api/runtime/bitrate?bitrate=${kbps}&clientname=${encodeURIComponent(client)}`); const d = await r.json().catch(() => ({})); say(r.ok && d.success !== false ? T.value.lbSent.replace('{client}', client) : T.value.lbFailed) } catch { say(T.value.lbFailed) }
}, (a) => a.client)
const ds5Tune = createQueuedCalls(async ({ key, value }) => {
  try { const d = await hostApi.postJson('/api/dualsense/tuning', { schema: 'ALK-DS5T/1', values: { [key]: value } }); if (d?.keys) ds5Keys.value = d.keys; else if (d?.status === false) say(T.value.dsFailed) } catch { say(T.value.dsFailed) }
}, (a) => a.key)
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
  <MPage :title="t.m3e.nav.overview" wide full>
    <template #actions>
      <MButton v-if="edit" variant="tonal" icon="check" @click="edit = false">{{ h.done }}</MButton>
      <MIconButton v-else icon="edit" data-coach="home-edit" :label="h.edit" @click="edit = true" />
    </template>
    <MAnnouncementStrip v-if="announcements.top.value" :item="announcements.top.value" :total="announcements.unread.value" :texts="announceTexts" @dismiss="announcements.dismiss" @open-url="openExternalUrl" @open-list="announceOpen = true" />
    <MHomeDash v-model:edit="edit" :cards="cards" :details="allDetails" :ctx="dashCtx" storage-key="panel-home" :texts="dashTexts" :config="state.config" :locale="locale" @action="onAction" />
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
