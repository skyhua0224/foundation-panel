<template>
  <MPage :title="l.title" :updated="readAt" :refreshing="refreshing" refreshable :locale="String(locale)" @refresh="refresh(true)">

    <MPageLoading v-if="!loaded" :rows="4" />
    <template v-else>
      <MBanner v-if="loadError" tone="error" icon="error">{{ l.loadFailed.replace('{error}', loadError) }}</MBanner>
      <MEmpty v-if="!live.length" icon="cast" :title="l.noStream" :text="idleCount ? l.idle.replace('{count}', idleCount) : ''" />
      <template v-else>
        <!-- 码率 -->
        <MSection :title="l.bitrate">
          <MGroup>
            <MChoiceItem v-if="live.length > 1" data-redact="client" v-model="client" icon="devices" :title="l.client" :options="live.map((s) => ({ value: s.client_name, label: sessionLabel(s) }))" :disabled="applying" :cancel-label="l.cancel" />
            <MItem v-else data-redact="client" icon="devices" :title="live[0].client_name" :value="modeOf(live[0])" />
            <MItem v-if="current?.bitrate" icon="network_check" :title="l.current" :value="formatBitrate(current.bitrate)" />
            <MSliderItem v-model="bitrateProgress" icon="speed" :title="l.target" :min="bitrateSpan.min" :max="bitrateSpan.max" :step="1" :decimals="0" :marks="bitrateMarks" size="m" icon-start="signal_cellular_alt_2_bar" icon-end="signal_cellular_alt" :readout-text="formatBitrate(bitrate)" :disabled="applying" />
            <MItem stack :title="l.presets"><template #control><MChipSet v-model="bitrate" :label="l.presets" :options="BITRATE_PRESETS.map((v) => ({ value: v, label: formatBitrate(v) }))" :disabled="applying" /></template></MItem>
            <MValueItem v-model.number="bitrate" icon="edit_note" :label="l.input" type="number" suffix="Kbps" :min="BITRATE.min" :max="BITRATE.max" :step="BITRATE.step" :check="(v) => (validBitrate(Number(v)) ? '' : rangeText)" :disabled="applying" :ok-label="l.ok" :cancel-label="l.cancel" />
          </MGroup>
          <div class="m-live__actions"><MButton icon="check" :loading="applying" :disabled="applying || !client" @click="applyBitrate">{{ applying ? l.applying : l.apply }}</MButton></div>
        </MSection>
        <!-- what the stream runs on now, read from the session; the settings behind each value are one press away (the same settings as 显示与视频, not a copy of them) -->
        <MSection :title="l.values">
          <MGroup>
            <MItem action icon="monitor" :title="l.display" :value="modeOf(live[0])" @click="emit('webui-navigate', '/config#tab=display&sub=output')"><template #trail><span class="m-item__count">{{ l.toSettings }}</span></template></MItem>
            <MItem v-if="live[0].codec" action icon="memory" :title="l.encoding" :value="String(live[0].codec).toUpperCase()" @click="emit('webui-navigate', '/config#tab=display&sub=encoders')"><template #trail><span class="m-item__count">{{ l.toSettings }}</span></template></MItem>
          </MGroup>
          <MFooter>{{ l.bitrateTemporary }}</MFooter>
        </MSection>
      </template>

      <!-- 画质增强: the NR live controls of the stream (NrSessionControls, as on 画质增强 › 当前会话); without a stream its state and one action -->
      <MSection v-if="nrReady" :title="t.m3e.display.quality">
        <MBanner v-if="nrError" tone="error" icon="error">{{ enhancementError(nrText, nrError) }}</MBanner>
        <MGroup v-if="nr.list.length > 1">
          <MChoiceItem icon="cast_connected" :title="nrText.choose" :model-value="nr.selected ?? ''" :options="nr.list.map((p) => ({ value: p.id, label: `${nrText.stream} #${p.id} · ${nrOutputLabel(p)}` }))" :cancel-label="l.cancel" @update:model-value="(v) => selectNr(Number(v))" />
        </MGroup>
        <NrSessionControls v-if="nrPipeline" :pipeline="nrPipeline" :online="nr.online" @changed="pollNr" @error="nrError = $event" />
        <MGroup v-else>
          <MItem icon="wand_stars" title="DLSS NR" :value="nrOverlayText.states[nrOverlayState(null, nr.online)]">
            <template #trail><MButton variant="text" size="sm" @click="nr.online ? emit('open-quality') : pollNr()">{{ nr.online ? l.nrOpen : l.refresh }}</MButton></template>
          </MItem>
        </MGroup>
      </MSection>

      <!-- 主机性能 -->
      <MSection :title="l.host">
        <MBanner v-if="perfError" tone="warn" icon="warning">{{ l.perfFailed.replace('{error}', perfError) }}</MBanner>
        <template v-if="perf">
          <div class="m-live__stats">
            <MStat icon="timer" :label="l.p95" :value="formatMs(perf.host_latency?.p95_ms)" />
            <MStat icon="speed" :label="l.fps" :value="formatNumber(perf.host_latency?.recent_fps)" />
            <MStat icon="pie_chart" :label="l.budget" :value="formatPercent(budgetUsage(perf))" />
          </div>
          <MGroup>
            <MItem icon="monitor_heart" :title="l.health" :value="l.status[status]" />
            <MItem icon="show_chart" stack :title="l.latency" :value="l.samples.replace('{count}', perf.host_latency?.samples ?? 0)">
              <template #control><MSparkline v-if="(perf.host_latency?.series_ms || []).length > 1" :points="perf.host_latency.series_ms.slice(-80)" :width="320" :height="48" /></template>
            </MItem>
          </MGroup>
          <MSection :title="l.window">
            <MGroup>
              <MItem v-for="row in windowRows" :key="row.label" :title="row.label" :value="row.value" />
            </MGroup>
          </MSection>
          <MSection v-if="segments.length" :title="l.pipeline">
            <MGroup>
              <MItem v-for="seg in segments" :key="seg.key" :title="l.segments[seg.key]" :text="`${l.avg} ${formatMs(seg.stats.avg_ms)} · ${l.last} ${formatMs(seg.stats.last_ms)}`" :value="`P95 ${formatMs(seg.stats.p95_ms)}`" />
            </MGroup>
          </MSection>
          <MSection :title="l.session">
            <MGroup>
              <MItem icon="memory" :title="l.encoderCapture" :value="`${perf.encoder || 'auto'} / ${perf.capture || 'auto'}`" />
              <MItem icon="speed" :title="l.bitrate" :value="perf.bitrate_kbps ? `${Math.round(perf.bitrate_kbps / 1000)} Mbps` : '--'" />
              <MItem icon="schedule" :title="l.uptime" :value="formatUptime(perf.uptime_ms)" />
            </MGroup>
          </MSection>
        </template>
        <MFooter v-else-if="!perfError">{{ l.status.idle }}</MFooter>
        <div v-if="live.length" class="m-live__actions"><MButton variant="danger" icon="stop" :disabled="stopping" @click="confirmStop = true">{{ l.stopAll }}</MButton></div>
      </MSection>
    </template>

    <MDialog v-model="confirmStop" :title="l.stopTitle" icon="stop" tone="danger">
      <p>{{ l.stopText }}</p>
      <template #actions><MButton variant="text" @click="confirmStop = false">{{ l.cancel }}</MButton><MButton variant="danger" @click="stopAll">{{ l.stop }}</MButton></template>
    </MDialog>
  </MPage>
</template>

<script setup>
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { MPage, MGroup, MSection, MItem, MSliderItem, MChoiceItem, MValueItem, MChipSet, MStat, MSparkline, MBanner, MButton, MIconButton, MFooter, MEmpty, MDialog, MPageLoading, useNotify } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { sunshine } from '../../tauri-adapter.js'
import NrSessionControls from '../NrSessionControls.vue'
import { useDlssNrReady } from '../../composables/useDlssNrReady.js'
import { enhancementControlsText, enhancementError } from '../../composables/enhancementControls.js'
import { nrOverlayText as overlayTexts } from '../../composables/nrOverlayMessages.js'
import { nrOutputLabel, nrOverlayState, nrPipelines } from '../../composables/nrOverlayState.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { bitrateChipMarks, bitrateFromProgress, bitrateSliderProgress, bitrateSliderSpan } from 'foundation-ui/settings/index.js'
import { BITRATE, BITRATE_PRESETS, DEFAULT_BITRATE, formatBitrate, validBitrate, liveSessions, perfSessionOf, budgetUsage, perfStatus, pipelineSegments, formatMs, formatNumber, formatPercent, formatInteger, formatUptime } from '../../composables/liveSession.js'

// 当前串流: the old 码率调整 tool (a live session's bitrate: change_bitrate) and the old 主机性能 window (/api/perf/current) on one page of 串流.
const { t, locale } = useI18n()
const l = computed(() => t.value.m3e.live)
const emit = defineEmits(['open-quality', 'webui-navigate'])
const say = (text) => useNotify().snack(text)

const sessions = ref([]), loaded = ref(false), refreshing = ref(false), loadError = ref('')
const readAt = ref(0)
const live = computed(() => liveSessions(sessions.value))
const idleCount = computed(() => (sessions.value.length && !live.value.length ? sessions.value.length : 0))
const client = ref(''), bitrate = ref(DEFAULT_BITRATE), applying = ref(false)
// the target slider runs on the shared segmented bitrate scale (fine steps where the usual values are, as the 串流 page and the Web UI draw it) between the limits of the host; the marks stand under the 常用码率 chips
const bitrateSpan = bitrateSliderSpan(BITRATE.min, BITRATE.max)
const bitrateMarks = bitrateChipMarks(BITRATE_PRESETS.map((kbps) => kbps / 1000), BITRATE.min, BITRATE.max)
const bitrateProgress = computed({
  get: () => Math.max(bitrateSpan.min, bitrateSliderProgress(bitrate.value)),
  set: (position) => { bitrate.value = Math.max(BITRATE.min, bitrateFromProgress(position)) },
})
const current = computed(() => live.value.find((s) => s.client_name === client.value) || null)
const modeOf = (s) => `${s.width}×${s.height}@${s.fps}fps`
const sessionLabel = (s) => `${s.client_name} (${modeOf(s)})`
const rangeText = computed(() => l.value.rangeError.replace('{min}', formatBitrate(BITRATE.min)).replace('{max}', formatBitrate(BITRATE.max)))
// a new client brings its own bitrate into the slider; a refresh of the same client keeps what is being drafted
watch(client, () => { bitrate.value = current.value?.bitrate ?? DEFAULT_BITRATE })

async function loadSessions() {
  try {
    sessions.value = (await sunshine.getActiveSessions()) || []
    loadError.value = ''
    if (!live.value.some((s) => s.client_name === client.value)) client.value = live.value[0]?.client_name || ''
  } catch (e) { loadError.value = String(e) }
}
async function applyBitrate() {
  if (!client.value) return say(l.value.selectFirst)
  if (!validBitrate(bitrate.value)) return say(rangeText.value)
  applying.value = true
  try {
    await sunshine.changeBitrate(client.value, bitrate.value)
    say(l.value.applied.replace('{value}', formatBitrate(bitrate.value)))
    setTimeout(loadSessions, 1000)
  } catch (e) {
    const text = String(e)
    say(text.includes('身份验证') || text.includes('401') ? l.value.authFailed : l.value.failed.replace('{error}', text))
  } finally { applying.value = false }
}

// 主机性能
const snapshot = ref(null), perfError = ref('')
const perf = computed(() => perfSessionOf(snapshot.value))
const status = computed(() => perfStatus(perf.value, !!perfError.value))
const segments = computed(() => pipelineSegments(perf.value))
const windowRows = computed(() => {
  const h = perf.value?.host_latency || {}
  return [
    [l.value.last, formatMs(h.last_ms)], [l.value.min, formatMs(h.min_ms)], [l.value.avg, formatMs(h.avg_ms)], [l.value.max, formatMs(h.max_ms)],
    [l.value.frameBudget, formatMs(1000 / (perf.value?.fps || 60))], [l.value.sampleAge, h.last_sample_age_ms == null ? '--' : `${Math.round(h.last_sample_age_ms)} ms`],
    [l.value.totalFrames, formatInteger(h.frames_total)], [l.value.totalSamples, formatInteger(h.total_samples)],
  ].map(([label, value]) => ({ label, value }))
})
let proxy = ''
async function loadPerf() {
  try {
    proxy = proxy || (await sunshine.getProxyUrl()) || ''
    const res = await fetch(`${proxy.replace(/\/$/, '')}/api/perf/current`, { signal: AbortSignal.timeout(3000) })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    snapshot.value = await res.json()
    perfError.value = ''
  } catch (e) { perfError.value = e?.message || String(e) }
}

// 画质增强 (DLSS NR) of the live stream: nr_live_status as 画质增强 polls it; a choice of stream is the overlay's target (nr_overlay_select_session), the same as there
const nrText = computed(() => enhancementControlsText(locale.value)), nrOverlayText = computed(() => overlayTexts(locale.value))
// only for an installed and usable DLSS NR: otherwise nothing of it on this page (the install is on 画质增强)
const nrReady = useDlssNrReady()
const nr = reactive({ list: [], selected: null, online: true })
const nrError = ref('')
const nrPipeline = computed(() => nr.list.find((p) => p.id === nr.selected) || null)
async function selectNr(id) {
  try { await invoke('nr_overlay_select_session', { id }); nr.selected = id; nrError.value = '' } catch (e) { nrError.value = String(e) }
}
let targetRead = false
async function pollNr() {
  if (!nrReady.value) return
  try {
    if (!targetRead) { targetRead = true; try { const s = await invoke('nr_overlay_settings'); if (s?.target != null) nr.selected = s.target } catch { /* no saved target */ } }
    nr.list = nrPipelines((await invoke('nr_live_status'))?.pipelines)
    nr.online = true
    if (nr.selected !== null && !nr.list.some((p) => p.id === nr.selected)) nr.selected = null
    if (nr.selected === null && nr.list.length === 1) await selectNr(nr.list[0].id)
  } catch { nr.online = false; nr.list = [] }
}

const confirmStop = ref(false), stopping = ref(false)
async function stopAll() {
  confirmStop.value = false
  stopping.value = true
  try { await sunshine.stopAllSessions(); say(l.value.stopped) } catch (e) { say(l.value.stopFailed.replace('{error}', String(e))) } finally { stopping.value = false; refresh() }
}

let inFlight = false
async function refresh(manual = false) {
  if (inFlight) return
  inFlight = true
  if (manual) refreshing.value = true
  try { await Promise.all([loadSessions(), loadPerf(), pollNr()]); readAt.value = Date.now() } finally { inFlight = false; refreshing.value = false; loaded.value = true }
}
let timer = null
onMounted(() => { refresh(); timer = setInterval(refresh, 2000) })
onUnmounted(() => clearInterval(timer))
</script>

<style>
.m-live__actions { display: flex; flex-wrap: wrap; gap: 8px; padding: 8px 0 4px; }
.m-live__stats { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; margin-bottom: 8px; }
</style>
