<template>
  <MPage :title="r.title">
    <template #actions><MIconButton icon="refresh" :label="r.refresh" :disabled="loading" @click="loadAll" /></template>

    <MPageLoading v-if="loading && !loaded" :rows="4" />
    <template v-else>
      <MBanner v-if="!status.running" tone="warn" icon="warning">{{ r.notRunning }}<template #actions><MButton variant="text" size="sm" @click="loadAll">{{ r.refresh }}</MButton></template></MBanner>
      <MGroup v-else>
        <MItem icon="speed" :title="`RTSS v${status.version}`" :value="r.running" />
      </MGroup>
      <MBanner v-if="status.running && !status.cli_path" tone="info" icon="download">{{ r.cliMissing }}<template #actions><MButton variant="text" size="sm" :loading="downloading" @click="downloadCli">{{ downloading ? r.downloading : r.download }}</MButton></template></MBanner>

      <template v-if="status.running">
        <!-- 帧率限制 -->
        <MSection v-if="status.cli_path" :title="r.fps">
          <MGroup>
            <MSwitchItem :model-value="limiterOn" icon="speed" :title="r.limiter" @update:model-value="toggleLimiter" />
            <MSliderItem v-model="fps" icon="timer" :title="r.limit" :min="FPS_LIMITS.min" :max="FPS_LIMITS.max" :step="1" :decimals="0" unit=" FPS" size="m" icon-start="slow_motion_video" icon-end="speed" :readout-text="fps ? '' : r.unlimited" :disabled="fpsBusy" />
            <MItem stack :title="r.presets"><template #control><MChipSet v-model="fps" :label="r.presets" :options="FPS_PRESETS.map((v) => ({ value: v, label: String(v) }))" :disabled="fpsBusy" /></template></MItem>
          </MGroup>
          <div class="m-rtss__actions">
            <MButton icon="check" :loading="fpsBusy" :disabled="fpsBusy || !fps" @click="applyLimit">{{ r.applyFps }}</MButton>
            <MButton variant="text" :disabled="fpsBusy" @click="removeLimit">{{ r.removeFps }}</MButton>
          </div>
        </MSection>

        <!-- OSD 文本 -->
        <MSection :title="r.osd">
          <div class="m-rtss__toolbar" role="toolbar" :aria-label="r.osd">
            <MIconButton icon="format_bold" :label="r.bold" @click="wrap('B')" />
            <MIconButton icon="format_italic" :label="r.italic" @click="wrap('I')" />
            <label class="m-rtss__color" :title="r.color"><span :style="{ background: '#' + color }" /><input type="color" :value="'#' + color" :aria-label="r.color" @input="color = $event.target.value.slice(1).toUpperCase()" /></label>
            <MIconButton icon="format_color_text" :label="r.insertColor" @click="insert(colorTag(color))" />
            <MIconButton icon="code" :label="r.colorTag" @click="insert('<C=>')" />
            <MIconButton icon="format_size" :label="r.sizeTag" @click="insert('<S=>')" />
            <MIconButton icon="keyboard_return" :label="r.newline" @click="insert('\\n')" />
          </div>
          <MChipSet v-model="fontSize" class="m-rtss__sizes" :label="r.size" :options="FONT_SIZES.map((v) => ({ value: v, label: v ? `${v}px` : r.defaultSize }))" />
          <md-outlined-text-field ref="osdField" class="m-rtss__text" type="textarea" rows="4" :label="r.text" :value="osdText" :placeholder="r.placeholder" :supporting-text="r.hint" @input="osdText = $event.target.value"><span slot="container" class="m-field__fill" aria-hidden="true" /></md-outlined-text-field>
          <div class="m-rtss__actions">
            <MButton icon="send" :loading="osdBusy" :disabled="osdBusy" @click="sendOsd">{{ r.send }}</MButton>
            <MButton variant="text" :disabled="osdBusy" @click="clearOsd">{{ r.clear }}</MButton>
            <MButton v-if="status.cli_path" variant="tonal" icon="visibility" @click="toggleOverlay">{{ r.toggleOverlay }}</MButton>
          </div>
        </MSection>

        <!-- OSD 设置 -->
        <MSection v-if="status.cli_path" :title="r.settings">
          <MGroup>
            <MSwitchItem :model-value="osd.osd_enabled === 1" icon="layers" :title="r.osdOn" @update:model-value="(v) => setProp('OSD', v ? '1' : '0')" />
            <MSwitchItem :model-value="osd.show_own_stats === 1" icon="query_stats" :title="r.ownStats" @update:model-value="(v) => setProp('OSDShowOwnStatistics', v ? '1' : '0')" />
            <MItem icon="zoom_in" :title="r.zoom" stack><template #control><MSeg :model-value="osd.zoom" :label="r.zoom" :options="ZOOMS.map((z) => ({ value: z, label: `${z}×` }))" @update:model-value="(z) => setProp('OnScreenDisplayZoom', String(z))" /></template></MItem>
            <MValueItem :model-value="osd.position_x ?? ''" icon="swap_horiz" :label="r.posX" type="number" :min="0" :ok-label="r.ok" :cancel-label="r.cancel" @update:model-value="(v) => setProp('OnScreenDisplayX', String(v))" />
            <MValueItem :model-value="osd.position_y ?? ''" icon="swap_vert" :label="r.posY" type="number" :min="0" :ok-label="r.ok" :cancel-label="r.cancel" @update:model-value="(v) => setProp('OnScreenDisplayY', String(v))" />
            <MChoiceItem :model-value="osd.coordinate_space ?? ''" icon="grid_on" :title="r.coord" :options="[{ value: 0, label: r.coordFramebuffer }, { value: 1, label: r.coordScreen }]" :cancel-label="r.cancel" @update:model-value="(v) => setProp('OSDCoordinateSpace', String(v))" />
          </MGroup>
        </MSection>

        <!-- 串流监控 -->
        <MSection :title="r.monitor">
          <MGroup>
            <MSwitchItem :model-value="monitoring" icon="monitor_heart" :title="r.monitorOn" @update:model-value="(v) => (v ? startMonitoring() : stopMonitoring())" />
            <MItem v-for="group in groups" :key="group.id" stack :title="group.label">
              <template #control><MChipSet :model-value="config.metrics" multiple :label="group.label" :options="group.metrics" @update:model-value="setMetrics" /></template>
            </MItem>
            <MValueItem v-model="config.header_text" icon="title" :label="r.header" placeholder="☀ Foundation Sunshine" :help="r.cjk" :ok-label="r.ok" :cancel-label="r.cancel" />
            <MChoiceItem v-model="config.interval_ms" icon="schedule" :title="r.interval" :options="INTERVALS.map((ms) => ({ value: ms, label: `${ms / 1000}s` }))" :cancel-label="r.cancel" />
            <MItem v-for="c in COLORS" :key="c.key" :icon="c.icon" :title="r[c.label]" :value="'#' + config[c.key]">
              <template #trail><label class="m-rtss__color"><span :style="{ background: '#' + config[c.key] }" /><input type="color" :value="'#' + config[c.key]" :aria-label="r[c.label]" @input="config[c.key] = $event.target.value.slice(1).toUpperCase()" /></label></template>
            </MItem>
          </MGroup>
          <MFooter v-if="config.metrics.includes('pipeline_fps')">{{ r.pipelineHint }}</MFooter>
          <div v-if="monitoring && snapshot.osd_text" class="m-rtss__preview"><small>{{ r.preview }}</small><pre>{{ snapshot.osd_text }}</pre></div>
        </MSection>

        <!-- HWiNFO -->
        <MSection :title="r.hwinfo">
          <MBanner v-if="hw.state.loaded && !hw.state.available" tone="info" icon="info">{{ r.hwUnavailable }}<template #actions><MButton variant="text" size="sm" :loading="hw.state.loading" @click="hw.load">{{ r.refresh }}</MButton></template></MBanner>
          <template v-else>
            <MGroup>
              <MSwitchItem :model-value="hw.state.injecting" icon="input" :title="r.inject" :text="hw.state.selected.length ? r.selected.replace('{count}', hw.state.selected.length) : r.pick" :disabled="!hw.state.available" @update:model-value="toggleInject" />
              <MItem action icon="refresh" :title="r.refreshSensors" :disabled="hw.state.loading" @click="hw.load" />
            </MGroup>
            <md-outlined-text-field class="m-rtss__search" type="search" :label="r.search" :value="query" @input="query = $event.target.value"><span slot="container" class="m-field__fill" aria-hidden="true" /></md-outlined-text-field>
            <MGroup v-for="group in shownGroups" :key="group.name">
              <MItem action :icon="isOpen(group) ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" :title="group.name" :value="String(group.readings.length)" @click="toggleGroup(group.name)" />
              <template v-if="isOpen(group)">
                <MItem v-for="reading in group.readings" :key="reading.index" action :selected="hw.state.selected.includes(reading.index)" :icon="hw.state.selected.includes(reading.index) ? 'check_box' : 'check_box_outline_blank'" :title="reading.label" :value="`${Number(reading.value).toFixed(1)} ${reading.unit}`" @click="hw.toggleReading(reading.index)" />
              </template>
            </MGroup>
            <MEmpty v-if="hw.state.loaded && !shownGroups.length" icon="sensors" :title="query ? r.noMatch : r.noSensors" />
          </template>
        </MSection>
      </template>
    </template>
  </MPage>
</template>

<script setup>
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MPage, MGroup, MSection, MItem, MSwitchItem, MSliderItem, MChoiceItem, MValueItem, MChipSet, MSeg, MBanner, MButton, MIconButton, MFooter, MEmpty, MPageLoading, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { FPS_LIMITS, FPS_PRESETS, FONT_SIZES, ZOOMS, INTERVALS, OSD_OWNER, insertText, wrapTag, colorTag, sizeTag, metricGroups, filterSensors, loadMonitoringConfig, saveMonitoringConfig } from '../../composables/rtssTools.js'
import { useHwInfoInject } from '../../composables/useHwInfoInject.js'
import { noteRtssStatus } from '../../composables/rtssInstall.js'

// RTSS 叠加: everything of the old RTSS tool window (status, frame-rate limit, OSD text, OSD settings, stream monitoring, HWiNFO) on one page of 系统.
const { t, locale } = useI18n()
const r = computed(() => t.value.m3e.rtss)
const say = (text) => useNotify().snack(text)
const fail = (e) => say(String(e))

const loading = ref(true), loaded = ref(false)
const status = ref({ running: false, version: '', cli_path: '' })
async function loadStatus() {
  try { status.value = (await invoke('get_rtss_status')) || { running: false, version: '', cli_path: '' }; noteRtssStatus(status.value) } catch { status.value = { ...status.value, running: false } }
}
async function loadAll() {
  loading.value = true
  await loadStatus()
  if (status.value.running) await Promise.all([status.value.cli_path ? loadFps() : null, status.value.cli_path ? loadProps() : null, loadMonitoring(), hw.load()])
  loading.value = false
  loaded.value = true
}
const downloading = ref(false)
async function downloadCli() {
  downloading.value = true
  try { await invoke('rtss_download_cli'); say(r.value.downloaded); await loadAll() } catch (e) { fail(e) } finally { downloading.value = false }
}

// 帧率限制
const fps = ref(60), limiterOn = ref(true), fpsBusy = ref(false)
async function loadFps() {
  try {
    const current = await invoke('rtss_get_framerate_limit', { profile: null })
    if (current > 0) fps.value = current
    limiterOn.value = (await invoke('rtss_get_limiter_status')) === 1
  } catch { /* the defaults stay */ }
}
async function applyLimit() {
  fpsBusy.value = true
  try { await invoke('rtss_set_framerate_limit', { fps: fps.value, profile: null }); say(r.value.fpsApplied.replace('{fps}', fps.value)) } catch (e) { fail(e) } finally { fpsBusy.value = false }
}
async function removeLimit() {
  fpsBusy.value = true
  try { await invoke('rtss_set_framerate_limit', { fps: 0, profile: null }); fps.value = 0; say(r.value.fpsRemoved) } catch (e) { fail(e) } finally { fpsBusy.value = false }
}
async function toggleLimiter() {
  try { limiterOn.value = (await invoke('rtss_toggle_limiter')) === '1'; say(limiterOn.value ? r.value.limiterOn : r.value.limiterOff) } catch (e) { fail(e) }
}

// OSD 文本
const osdText = ref(''), osdBusy = ref(false), osdField = ref(null), color = ref('FFFFFF'), fontSize = ref(0)
function place({ text, caret }) {
  osdText.value = text
  nextTick(() => { const f = osdField.value; if (f) { f.value = text; f.focus(); try { f.setSelectionRange(caret, caret) } catch { /* not focusable yet */ } } })
}
const caret = () => { const f = osdField.value; return [f?.selectionStart ?? osdText.value.length, f?.selectionEnd ?? osdText.value.length] }
const wrap = (tag) => { const [s, e] = caret(); place(wrapTag(osdText.value, s, e, tag)) }
const insert = (tag) => { if (!tag) return; const [s] = caret(); place(insertText(osdText.value, s, s, tag)) }
watch(fontSize, (px) => insert(sizeTag(px)))
async function sendOsd() {
  osdBusy.value = true
  try { await invoke('rtss_set_osd', { text: osdText.value, owner: OSD_OWNER }); say(r.value.sent) } catch (e) { fail(e) } finally { osdBusy.value = false }
}
async function clearOsd() {
  osdBusy.value = true
  try { await invoke('rtss_clear_osd', { owner: OSD_OWNER }); osdText.value = ''; say(r.value.cleared) } catch (e) { fail(e) } finally { osdBusy.value = false }
}
async function toggleOverlay() {
  try { say((await invoke('rtss_toggle_overlay')) === '1' ? r.value.overlayShown : r.value.overlayHidden) } catch (e) { fail(e) }
}

// OSD 设置
const osd = reactive({ osd_enabled: null, show_own_stats: null, position_x: null, position_y: null, zoom: null, coordinate_space: null })
const PROP_OF = { OSD: 'osd_enabled', OSDShowOwnStatistics: 'show_own_stats', OnScreenDisplayX: 'position_x', OnScreenDisplayY: 'position_y', OnScreenDisplayZoom: 'zoom', OSDCoordinateSpace: 'coordinate_space' }
async function loadProps() {
  try { Object.assign(osd, await invoke('rtss_get_osd_properties', { profile: null })) } catch { /* unknown values stay empty */ }
}
async function setProp(key, value) {
  try { await invoke('rtss_set_osd_property', { key, value, profile: null }); osd[PROP_OF[key]] = parseInt(value, 10); say(r.value.propApplied) } catch (e) { fail(e) }
}

// 串流监控: the host writes the OSD; the page shows what it writes
const COLORS = [{ key: 'title_color', label: 'titleColor', icon: 'format_color_fill' }, { key: 'label_color', label: 'labelColor', icon: 'label' }, { key: 'value_color', label: 'valueColor', icon: 'pin' }]
const config = reactive(loadMonitoringConfig())
const monitoring = ref(false), available = ref([])
const snapshot = reactive({ active: false, osd_text: '' })
const groups = computed(() => metricGroups(available.value, String(locale.value).startsWith('zh')))
let poll = null
// a style change restarts a running monitor with it; the config read back from the host (loadMonitoring) does not
let adopting = false
watch(config, (v) => { saveMonitoringConfig(JSON.parse(JSON.stringify(v))); if (monitoring.value && !adopting) restartSoon() }, { deep: true })
let restartTimer = null
const restartSoon = () => { clearTimeout(restartTimer); restartTimer = setTimeout(() => startMonitoring(true), 400) }
function setMetrics(list) { config.metrics = list }
async function loadMonitoring() {
  try { available.value = await invoke('rtss_get_available_metrics') } catch { /* no metrics offered */ }
  try {
    const snap = await invoke('rtss_get_monitoring_status')
    if (snap?.active) { if (snap.config) { adopting = true; Object.assign(config, snap.config); await nextTick(); adopting = false } monitoring.value = true; Object.assign(snapshot, snap); startPoll() }
  } catch { /* not running */ }
}
async function startMonitoring(quiet = false) {
  try { await invoke('rtss_start_monitoring', { config: JSON.parse(JSON.stringify(config)) }); monitoring.value = true; startPoll(); if (!quiet) say(r.value.started) } catch (e) { fail(e) }
}
async function stopMonitoring() {
  try { await invoke('rtss_stop_monitoring'); monitoring.value = false; stopPoll(); snapshot.osd_text = ''; say(r.value.stopped) } catch (e) { fail(e) }
}
function startPoll() {
  stopPoll()
  poll = setInterval(async () => {
    try { const snap = await invoke('rtss_get_monitoring_status'); Object.assign(snapshot, snap); if (!snap.active) { monitoring.value = false; stopPoll() } } catch { /* next round */ }
  }, 2000)
}
function stopPoll() { clearInterval(poll); poll = null }

// HWiNFO
const hw = useHwInfoInject()
const query = ref(''), open = ref(new Set())
const shownGroups = computed(() => filterSensors(hw.state.groups, query.value))
const isOpen = (group) => group.match || open.value.has(group.name)
const toggleGroup = (name) => { const s = new Set(open.value); s.has(name) ? s.delete(name) : s.add(name); open.value = s }
function toggleInject(on) {
  if (!on) return hw.stop()
  if (!hw.start()) say(r.value.pick)
}
watch(() => hw.state.error, (e) => { if (e) fail(e) })

onMounted(loadAll)
onUnmounted(() => { stopPoll(); clearTimeout(restartTimer) })
</script>

<style>
.m-rtss__actions { display: flex; flex-wrap: wrap; gap: 8px; padding: 8px 0 4px; }
.m-rtss__toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 2px; padding: 4px 0; }
.m-rtss__color { position: relative; display: inline-grid; place-items: center; width: 40px; height: 40px; border-radius: 20px; cursor: pointer; }
.m-rtss__color:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent); }
.m-rtss__color span { width: 20px; height: 20px; border-radius: 10px; outline: 1px solid var(--md-sys-color-outline); }
.m-rtss__color input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
.m-rtss__sizes { padding: 4px 0 12px; }
.m-rtss__text, .m-rtss__search { width: 100%; }
.m-rtss__search { margin: 4px 0 12px; }
.m-rtss__preview { margin-top: 12px; padding: 12px 16px; border-radius: 16px; background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface-variant); }
.m-rtss__preview pre { margin: 6px 0 0; font: var(--m-body-medium); font-family: ui-monospace, Consolas, monospace; white-space: pre-wrap; color: var(--md-sys-color-on-surface); }
</style>
