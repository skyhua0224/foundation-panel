<script setup>
import { computed, ref } from 'vue'
import { MPage, MTileGrid, MTile, MIconButton, MStatusPill, MBar, MRing, MSparkline, MSheet, MItem, MGroup, MDialog, MButton, MIcon, useTiles } from 'foundation-ui/vue'
import { usePanelHome, formatUptime } from '../composables/usePanelHome.js'
import { SPONSORS } from '../config/sponsors.js'
import { openExternalUrl } from '../tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'
import { ROUTES } from '../composables/useRouter.js'

const props = defineProps({
  runtime: { type: Object, required: true },
  updateAvailable: { type: Boolean, default: false },
})
const emit = defineEmits(['open', 'webui-navigate'])
const { t } = useI18n()
const h = computed(() => t.value.m3e.home)
const { state, session, uptimeMs, perfSession, memoryShare, restart, stopAll, openTool } = usePanelHome()

// Catalogue: only tiles whose data has a real source today. Packet loss, codec and temperature have none and are not listed.
const catalogue = [
  { id: 'host', icon: 'dns', sizes: ['2x1', '2x2'], size: '2x1' },
  { id: 'session', icon: 'cast_connected', sizes: ['2x2', '2x1'], size: '2x2' },
  { id: 'clients', icon: 'devices', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'apps', icon: 'apps', sizes: ['1x1'], size: '1x1' },
  { id: 'bitrate', icon: 'speed', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'latency', icon: 'timer', sizes: ['1x1', '2x1'], size: '1x1' },
  { id: 'encoder', icon: 'memory', sizes: ['1x1'], size: '1x1' },
  { id: 'hdr', icon: 'hdr_on', sizes: ['1x1'], size: '1x1' },
  { id: 'memory', icon: 'storage', sizes: ['2x1'], size: '2x1' },
  { id: 'gpu', icon: 'developer_board', sizes: ['1x1', '2x1'], size: '2x1' },
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
const DEFAULTS = [
  'host', 'session', { id: 'clients' }, { id: 'apps' }, 'memory', { id: 'gamepad' }, { id: 'mic' }, { id: 'errors' },
  { id: 'pair' }, { id: 'downloads' }, { id: 'sponsor' }, { id: 'logs' },
]
const store = useTiles('panel-home', catalogue, DEFAULTS)
const edit = ref(false)
const gridTexts = computed(() => ({ done: h.value.done, add: h.value.add, reset: h.value.reset, remove: h.value.remove, resize: h.value.resize, earlier: h.value.earlier, later: h.value.later, addTitle: h.value.addTitle, nothing: h.value.nothing, edit: h.value.edit }))

const sponsorOpen = ref(false)
const confirmRestart = ref(false)
const confirmStop = ref(false)
const unavailable = computed(() => h.value.unavailable)
const mbps = (kbps) => (kbps ? (kbps / 1000).toFixed(kbps >= 10000 ? 0 : 1) : null)
const dsState = computed(() => props.runtime.ds?.state || '')
const dsLabel = computed(() => (dsState.value ? (t.value.dualSense.status.states[dsState.value] || dsState.value) : ''))
const micLabel = computed(() => t.value.deviceHub.microphone.statusLabels[props.runtime.mic?.state] || '')
const usbCount = computed(() => props.runtime.usb?.attached_devices?.length ?? null)
const memLine = computed(() => {
  const m = state.memory
  if (!m?.system_total_memory) return ''
  const gb = (v) => (v / 1024 ** 3).toFixed(1)
  return `${gb(m.system_total_memory - (m.system_available_memory || 0))} / ${gb(m.system_total_memory)} GB`
})
const hostBadge = computed(() => (props.updateAvailable ? 'warn' : state.loaded ? 'ok' : ''))
const openExternal = (url) => openExternalUrl(url)
const doRestart = async () => { confirmRestart.value = false; await restart() }
const doStop = async () => { confirmStop.value = false; await stopAll() }
</script>

<template>
  <MPage :title="t.m3e.nav.overview" wide>
    <template #actions>
      <MIconButton :icon="edit ? 'check' : 'edit'" :label="edit ? h.done : h.edit" @click="edit = !edit" />
    </template>
    <MTileGrid v-model:edit="edit" :store="store" :texts="gridTexts" :label="t.m3e.nav.overview">
      <template #tile="{ tile, def, size }">
        <!-- host -->
        <MTile v-if="tile.id === 'host'" :title="def.title" icon="dns" tone="primary" :size="size" :badge="hostBadge">
          <div class="m-tile__row"><span class="m-num">{{ uptimeMs ? formatUptime(uptimeMs) : '–' }}</span></div>
          <template #foot>{{ h.uptime }} · {{ state.version || unavailable }}<template v-if="updateAvailable"> · {{ h.update }}</template></template>
        </MTile>
        <!-- session -->
        <MTile v-else-if="tile.id === 'session'" :title="def.title" icon="cast_connected" :tone="session ? 'tertiary' : ''" :size="size" :badge="session ? 'ok' : ''">
          <template v-if="session">
            <MStatusPill icon="play_arrow" :label="session.app_name || session.client_name" :action="false" tone="ok">
              <template #actions><MIconButton icon="stop" :label="h.stop" tonal @click="confirmStop = true" /></template>
            </MStatusPill>
            <div class="m-tile__chips">
              <span class="m-chip">{{ session.client_name }}</span>
              <span class="m-chip">{{ session.width }}×{{ session.height }}@{{ session.fps }}</span>
              <span v-if="session.enable_hdr" class="m-chip m-chip--info">HDR</span>
            </div>
            <MSparkline v-if="state.bitrateSeries.length > 1" class="m-tile__spark" :points="state.bitrateSeries" :width="240" :height="56" />
            <span class="m-num m-num--md">{{ mbps(session.bitrate) }}<small>Mbps</small></span>
          </template>
          <span v-else class="m-num m-num--sm">{{ h.idle }}</span>
        </MTile>
        <!-- clients -->
        <MTile v-else-if="tile.id === 'clients'" :title="def.title" icon="devices" :size="size" action @click="emit('open', 'clients')">
          <span class="m-num">{{ state.clientsTotal ?? '–' }}</span>
          <template #foot>{{ h.paired }}</template>
        </MTile>
        <MTile v-else-if="tile.id === 'apps'" :title="def.title" icon="apps" tone="secondary" :size="size" action @click="emit('open', 'apps')">
          <span class="m-num">{{ state.appsTotal ?? '–' }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'bitrate'" :title="def.title" icon="speed" :size="size">
          <MSparkline v-if="size !== '1x1' && state.bitrateSeries.length > 1" class="m-tile__spark" :points="state.bitrateSeries" :width="240" :height="56" />
          <span class="m-num">{{ session ? mbps(session.bitrate) : '–' }}<small v-if="session">Mbps</small></span>
        </MTile>
        <MTile v-else-if="tile.id === 'latency'" :title="def.title" icon="timer" :size="size">
          <MSparkline v-if="size !== '1x1' && state.latencySeries.length > 1" class="m-tile__spark" :points="state.latencySeries" :width="240" :height="56" />
          <span class="m-num">{{ perfSession?.host_latency?.avg_ms != null ? perfSession.host_latency.avg_ms.toFixed(1) : '–' }}<small v-if="perfSession">ms</small></span>
        </MTile>
        <MTile v-else-if="tile.id === 'encoder'" :title="def.title" icon="memory" tone="secondary" :size="size">
          <span class="m-num m-num--sm">{{ state.config?.active_encoder || '–' }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'hdr'" :title="def.title" icon="hdr_on" :tone="session?.enable_hdr ? 'primary' : ''" :size="size">
          <span class="m-num m-num--md">{{ session ? (session.enable_hdr ? h.hdrOn : h.hdrOff) : '–' }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'memory'" :title="def.title" icon="storage" :size="size">
          <MBar v-if="memoryShare != null" :value="memoryShare" icon="storage" :text="memLine" :label="def.title" />
          <span v-else class="m-num m-num--sm">{{ unavailable }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'gpu'" :title="def.title" icon="developer_board" tone="tertiary" :size="size">
          <ul v-if="state.gpus.length" class="m-tile__list"><li v-for="g in state.gpus.slice(0, 3)" :key="g"><span>{{ g }}</span></li></ul>
          <span v-else class="m-num m-num--sm">{{ unavailable }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'vdd'" :title="def.title" icon="desktop_windows" :size="size" action @click="emit('open', 'vdd')">
          <span class="m-num m-num--sm">{{ state.vdd?.status_text || (state.vdd ? (state.vdd.running ? h.on : h.off) : '–') }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'gamepad'" :title="def.title" icon="sports_esports" :tone="['ready', 'active'].includes(dsState) ? 'success' : ''" :size="size" action @click="emit('open', 'gamepad')">
          <span class="m-num m-num--sm">{{ dsLabel || '–' }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'mic'" :title="def.title" icon="mic" :size="size" action @click="emit('open', 'microphone')">
          <span class="m-num m-num--sm">{{ micLabel || '–' }}</span>
        </MTile>
        <MTile v-else-if="tile.id === 'usb'" :title="def.title" icon="usb" :size="size" action @click="emit('open', 'usb')">
          <span class="m-num">{{ usbCount ?? '–' }}</span>
          <template #foot>{{ runtime.usb?.ready ? h.ready : '' }}</template>
        </MTile>
        <MTile v-else-if="tile.id === 'errors'" :title="def.title" icon="bug_report" :tone="state.logs?.errors ? 'error' : ''" :size="size" action @click="openTool('logs')">
          <span class="m-num">{{ state.logs ? state.logs.errors : '–' }}</span>
          <template #foot>{{ state.logs ? (state.logs.errors ? `${state.logs.warnings} ${h.warnings}` : h.noErrors) : unavailable }}</template>
        </MTile>
        <MTile v-else-if="tile.id === 'pair'" :title="def.title" icon="qr_code_2" tone="primary" :size="size" action @click="emit('open', 'clients')">
          <MIcon name="add_circle" class="m-tile__big" />
        </MTile>
        <MTile v-else-if="tile.id === 'restart'" :title="def.title" icon="restart_alt" :size="size" action @click="confirmRestart = true"><MIcon name="restart_alt" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'logs'" :title="def.title" icon="description" :size="size" action @click="openTool('logs')"><MIcon name="open_in_new" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'downloads'" :title="def.title" icon="download" tone="success" :size="size" action @click="emit('open', 'downloads')"><MIcon name="download" class="m-tile__big" /></MTile>
        <MTile v-else-if="tile.id === 'sponsor'" :title="def.title" icon="favorite" tone="tertiary" :size="size" action @click="sponsorOpen = true"><MIcon name="favorite" class="m-tile__big" /></MTile>
      </template>
    </MTileGrid>

    <MSheet v-model="sponsorOpen" :title="h.sponsorTitle" side="end" :close-label="h.done">
      <MGroup>
        <MItem v-for="s in SPONSORS" :key="s.id" action icon="favorite" :title="s.label" @click="openExternal(s.url)">
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
