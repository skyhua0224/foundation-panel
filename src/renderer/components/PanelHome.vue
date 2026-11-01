<script setup>
import { computed, ref } from 'vue'
import { MPage, MTileGrid, MTile, MIconButton, MStatusPill, MBar, MSparkline, MSheet, MItem, MGroup, MDialog, MButton, MChip, MIcon, MContextMenu, useTiles } from 'foundation-ui/vue'
import { usePanelHome, formatUptime } from '../composables/usePanelHome.js'
import { SPONSORS } from '../config/sponsors.js'
import { openExternalUrl } from '../tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'

const props = defineProps({
  runtime: { type: Object, required: true },
  updateAvailable: { type: Boolean, default: false },
})
const emit = defineEmits(['open', 'webui-navigate'])
const { t } = useI18n()
const h = computed(() => t.value.m3e.home)
const { state, session, uptimeMs, perfSession, memoryShare, restart, stopAll, openTool } = usePanelHome()

// Catalogue: only tiles whose data has a real source. CPU and GPU load come from native Windows counters (the `system_load` command);
// temperatures only from HWiNFO when it happens to run; the loss and codec fields need a host that reports them. A tile without data hides itself instead of showing a placeholder.
const catalogue = [
  { id: 'host', icon: 'dns', sizes: ['2x1', '2x2'], size: '2x1' },
  { id: 'session', icon: 'cast_connected', sizes: ['2x1', '2x2'], size: '2x1' },
  { id: 'bitrate', icon: 'speed', sizes: ['1x1', '2x1'], size: '2x1' },
  { id: 'latency', icon: 'timer', sizes: ['1x1', '2x1'], size: '2x1' },
  { id: 'loss', icon: 'network_check', sizes: ['1x1'], size: '1x1' },
  { id: 'memory', icon: 'storage', sizes: ['1x1', '2x1'], size: '2x1' },
  { id: 'cpu', icon: 'memory', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'gpu', icon: 'developer_board', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'clients', icon: 'devices', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'apps', icon: 'apps', sizes: ['1x1'], size: '1x1' },
  { id: 'encoder', icon: 'videocam', sizes: ['1x1'], size: '1x1' },
  { id: 'hdr', icon: 'hdr_on', sizes: ['1x1'], size: '1x1' },
  { id: 'vdd', icon: 'desktop_windows', sizes: ['1x1'], size: '1x1' },
  { id: 'gamepad', icon: 'sports_esports', sizes: ['1x1'], size: '1x1' },
  { id: 'mic', icon: 'mic', sizes: ['1x1'], size: '1x1' },
  { id: 'usb', icon: 'usb', sizes: ['1x1'], size: '1x1' },
  { id: 'errors', icon: 'bug_report', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'pair', icon: 'qr_code_2', sizes: ['1x1'], size: '1x1' },
  { id: 'restart', icon: 'restart_alt', sizes: ['1x1'], size: '1x1' },
  { id: 'logs', icon: 'description', sizes: ['1x1'], size: '1x1' },
  { id: 'downloads', icon: 'download', sizes: ['1x1'], size: '1x1' },
  { id: 'sponsor', icon: 'favorite', sizes: ['1x1'], size: '1x1' },
].map((d) => ({ ...d, get title() { return h.value[d.id] } }))
const DEFAULTS = ['host', 'session', 'bitrate', 'latency', 'loss', 'memory', 'cpu', 'gpu', 'clients', 'apps', 'gamepad', 'mic', 'errors', 'pair', 'downloads', 'sponsor']
const store = useTiles('panel-home', catalogue, DEFAULTS)
const edit = ref(false)
const gridTexts = computed(() => ({ done: h.value.done, add: h.value.add, reset: h.value.reset, remove: h.value.remove, resize: h.value.resize, earlier: h.value.earlier, later: h.value.later, addTitle: h.value.addTitle, nothing: h.value.nothing, edit: h.value.edit }))

const LIVE = new Set(['bitrate', 'latency', 'loss'])
const hasLoss = computed(() => !!session.value && session.value.packet_loss !== undefined)
const isTileVisible = (tile) => {
  if (LIVE.has(tile.id) && !session.value) return false
  if (tile.id === 'loss') return hasLoss.value
  if (tile.id === 'cpu') return state.sys.cpu !== null
  if (tile.id === 'gpu') return state.sys.gpu !== null
  if (tile.id === 'memory') return memoryShare.value !== null
  return true
}
const tileSizeOf = (tile) => (tile.id === 'session' && !session.value ? '2x1' : null)

const contextMenu = ref(null)
function onTileMenu(event, tile, def) {
  const items = [{ id: 'edit', label: h.value.edit, icon: 'edit' }]
  if (def.sizes.length > 1) items.push({ id: 'resize', label: h.value.resize, icon: 'crop_square' })
  items.push({ id: 'remove', label: h.value.remove, icon: 'close', danger: true })
  contextMenu.value?.show(event, items, tile)
}
function onTileContext(entry, tile) {
  if (entry.id === 'edit') edit.value = true
  else if (entry.id === 'resize') store.resize(tile.id)
  else if (entry.id === 'remove') store.remove(tile.id)
}

const sponsorOpen = ref(false)
const confirmRestart = ref(false)
const confirmStop = ref(false)
const mbps = (kbps) => (kbps ? (kbps / 1000).toFixed(kbps >= 10000 ? 0 : 1) : null)
const dsState = computed(() => props.runtime.ds?.state || '')
const dsLabel = computed(() => (dsState.value ? (t.value.dualSense.status.states[dsState.value] || dsState.value) : ''))
const micShort = computed(() => ({ active: h.value.micOn, streaming: h.value.micOn, capturing: h.value.micOn, idle: h.value.ready, waiting: h.value.ready, unknown: '' }[props.runtime.mic?.state] ?? h.value.ready))
const usbCount = computed(() => props.runtime.usb?.attached_devices?.length ?? null)
const memLine = computed(() => {
  const m = state.memory
  if (!m?.system_total_memory) return ''
  const gb = (v) => (v / 1024 ** 3).toFixed(1)
  return `${gb(m.system_total_memory - (m.system_available_memory || 0))} / ${gb(m.system_total_memory)} GB`
})
const pct = (v) => (v === null || v === undefined ? null : Math.round(v))
const hostBadge = computed(() => (props.updateAvailable ? 'warn' : state.loaded ? 'ok' : ''))
const live = computed(() => !!session.value)
const doRestart = async () => { confirmRestart.value = false; await restart() }
const doStop = async () => { confirmStop.value = false; await stopAll() }
</script>

<template>
  <MPage :title="t.m3e.nav.overview" wide>
    <template #actions>
      <MIconButton :icon="edit ? 'check' : 'edit'" :label="edit ? h.done : h.edit" @click="edit = !edit" />
    </template>
    <MTileGrid v-model:edit="edit" :store="store" :texts="gridTexts" :label="t.m3e.nav.overview" :visible="isTileVisible" :size-of="tileSizeOf">
      <template #tile="{ tile, def, size }"><div class="home-cell" @contextmenu="onTileMenu($event, tile, def)">
        <MTile v-if="tile.id === 'host'" :title="def.title" icon="dns" :tone="updateAvailable ? 'primary' : ''" :size="size" :badge="hostBadge">
          <div class="m-num">{{ uptimeMs ? formatUptime(uptimeMs) : '–' }}</div>
          <template #foot>{{ h.uptime }} · {{ state.version || '–' }}<template v-if="updateAvailable"> · {{ h.update }}</template></template>
        </MTile>

        <MTile v-else-if="tile.id === 'session'" :title="def.title" icon="cast_connected" :tone="live ? 'primary' : ''" :size="size" :badge="live ? 'ok' : ''">
          <template v-if="session">
            <MStatusPill icon="play_arrow" :label="session.app_name || session.client_name" :action="false" tone="ok">
              <template #actions><MIconButton icon="stop" :label="h.stop" tonal @click="confirmStop = true" /></template>
            </MStatusPill>
            <div class="m-tile__chips">
              <MChip>{{ session.client_name }}</MChip>
              <MChip>{{ session.width }}×{{ session.height }}@{{ session.fps }}</MChip>
              <MChip v-if="session.codec">{{ String(session.codec).toUpperCase() }}</MChip>
              <MChip v-if="session.enable_hdr" icon="hdr_on">HDR</MChip>
            </div>
            <MSparkline v-if="size === '2x2' && state.series.bitrate.length > 1" class="m-tile__spark" :points="state.series.bitrate" :width="300" :height="64" />
            <div class="m-num m-num--md">{{ mbps(session.bitrate) }}<small>Mbps</small></div>
          </template>
          <div v-else class="home-empty"><MIcon name="cast_connected" /><span>{{ h.idle }}</span></div>
        </MTile>

        <MTile v-else-if="tile.id === 'bitrate'" :title="def.title" icon="speed" :size="size">
          <MSparkline v-if="size !== '1x1' && state.series.bitrate.length > 1" class="m-tile__spark" :points="state.series.bitrate" :width="300" :height="64" />
          <div class="m-num">{{ mbps(session?.bitrate) }}<small>Mbps</small></div>
        </MTile>
        <MTile v-else-if="tile.id === 'latency'" :title="def.title" icon="timer" :size="size">
          <MSparkline v-if="size !== '1x1' && state.series.latency.length > 1" class="m-tile__spark" :points="state.series.latency" :width="300" :height="64" />
          <div class="m-num">{{ perfSession?.host_latency?.avg_ms != null ? perfSession.host_latency.avg_ms.toFixed(1) : '–' }}<small>ms</small></div>
        </MTile>
        <MTile v-else-if="tile.id === 'loss'" :title="def.title" icon="network_check" :tone="(session?.packet_loss ?? 0) > 2 ? 'warn' : ''" :size="size">
          <div class="m-num">{{ session?.packet_loss != null ? session.packet_loss.toFixed(1) : '–' }}<small v-if="session?.packet_loss != null">%</small></div>
          <template #foot>{{ session?.packet_loss != null ? (session.dropped_frames != null ? `${session.dropped_frames} ${h.dropped}` : '') : h.notReported }}</template>
        </MTile>

        <MTile v-else-if="tile.id === 'memory'" :title="def.title" icon="storage" :size="size">
          <MSparkline v-if="size !== '1x1' && state.series.memory.length > 1" class="m-tile__spark" :points="state.series.memory.map((v) => v * 100)" :width="300" :height="44" />
          <MBar :value="memoryShare ?? 0" :text="size === '1x1' ? '' : memLine" :label="def.title" />
        </MTile>
        <MTile v-else-if="tile.id === 'cpu'" :title="def.title" icon="memory" :size="size">
          <MSparkline v-if="size !== '1x1' && state.series.cpu.length > 1" class="m-tile__spark" :points="state.series.cpu" :width="300" :height="44" />
          <MBar :value="(state.sys.cpu ?? 0) / 100" :text="state.hw.cpuTemp != null ? `${Math.round(state.hw.cpuTemp)} °C` : ''" :label="def.title" />
        </MTile>
        <MTile v-else-if="tile.id === 'gpu'" :title="def.title" icon="developer_board" :size="size">
          <MSparkline v-if="size !== '1x1' && state.series.gpu.length > 1" class="m-tile__spark" :points="state.series.gpu" :width="300" :height="44" />
          <MBar :value="(state.sys.gpu ?? 0) / 100" :text="state.hw.gpuTemp != null ? `${Math.round(state.hw.gpuTemp)} °C` : ''" :label="def.title" />
          <template v-if="state.gpus.length" #foot>{{ state.gpus[0] }}</template>
        </MTile>

        <MTile v-else-if="tile.id === 'clients'" :title="def.title" icon="devices" :size="size" action @click="emit('open', 'clients')">
          <div class="m-num">{{ state.clientsTotal ?? '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'apps'" :title="def.title" icon="apps" :size="size" action @click="emit('open', 'apps')">
          <div class="m-num">{{ state.appsTotal ?? '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'encoder'" :title="def.title" icon="videocam" :size="size">
          <div class="m-num m-num--sm home-text">{{ state.config?.active_encoder || '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'hdr'" :title="def.title" icon="hdr_on" :tone="session?.enable_hdr ? 'primary' : ''" :size="size">
          <div class="m-num m-num--sm home-text">{{ session ? (session.enable_hdr ? h.hdrOn : h.hdrOff) : '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'vdd'" :title="def.title" icon="desktop_windows" :size="size" action @click="emit('open', 'vdd')">
          <div class="m-num m-num--sm home-text">{{ state.vdd?.status_text || (state.vdd ? (state.vdd.running ? h.on : h.off) : '–') }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'gamepad'" :title="def.title" icon="sports_esports" :size="size" action @click="emit('open', 'gamepad')">
          <div class="m-num m-num--sm home-text">{{ dsLabel || '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'mic'" :title="def.title" icon="mic" :size="size" action @click="emit('open', 'microphone')">
          <div class="m-num m-num--sm home-text">{{ micShort || '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'usb'" :title="def.title" icon="usb" :size="size" action @click="emit('open', 'usb')">
          <div class="m-num">{{ usbCount ?? '–' }}</div>
        </MTile>
        <MTile v-else-if="tile.id === 'errors'" :title="def.title" icon="bug_report" :tone="state.logs?.errors ? 'error' : state.logs?.warnings ? 'warn' : ''" :size="size" action @click="openTool('logs')">
          <div class="m-num">{{ state.logs ? state.logs.errors : '–' }}</div>
          <template #foot>{{ state.logs ? (state.logs.errors ? `${state.logs.warnings} ${h.warnings}` : h.noErrors) : '' }}</template>
        </MTile>

        <MTile v-else-if="tile.id === 'pair'" :title="def.title" icon="qr_code_2" :size="size" action @click="emit('open', 'clients')"><MIcon name="add_circle" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'restart'" :title="def.title" icon="restart_alt" :size="size" action @click="confirmRestart = true"><MIcon name="restart_alt" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'logs'" :title="def.title" icon="description" :size="size" action @click="openTool('logs')"><MIcon name="open_in_new" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'downloads'" :title="def.title" icon="download" :size="size" action @click="emit('open', 'downloads')"><MIcon name="download" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'sponsor'" :title="def.title" icon="favorite" :size="size" action @click="sponsorOpen = true"><MIcon name="favorite" class="m-tile__big" /></MTile>
      </div></template>
    </MTileGrid>
    <MContextMenu ref="contextMenu" @select="onTileContext" />

    <MSheet v-model="sponsorOpen" :title="h.sponsorTitle" side="end" :close-label="h.done">
      <MGroup>
        <MItem v-for="s in SPONSORS" :key="s.id" action icon="favorite" :title="s.label" @click="openExternalUrl(s.url)">
          <template #trail><MIcon name="open_in_new" /></template>
        </MItem>
      </MGroup>
    </MSheet>
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

<style>
.home-cell { display: contents; }
.home-empty { display: flex; align-items: center; gap: 10px; margin-top: auto; color: var(--md-sys-color-on-surface-variant); font: var(--m-body-medium); }
.home-text { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
