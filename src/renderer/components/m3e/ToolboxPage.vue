<template>
  <MPage :title="t.sidebar.toolbox">
    <div class="panel-toolgrid">
      <MTile action icon="sports_esports" :title="t.controllersHub.peripherals.meta.title" :badge="metaBadge" @click="byId('gamepad-test')?.action()">
        <p class="panel-tool__sub">{{ metaLine }}</p>
        <template #foot><span class="panel-tool__go">{{ t.controllersHub.peripherals.meta.launch }}</span></template>
      </MTile>
      <MTile action icon="edit" :title="t.controllersHub.peripherals.stylus.title" @click="byId('stylus-input-probe')?.action()">
        <template #foot><span class="panel-tool__go">{{ t.controllersHub.peripherals.stylus.launch }}</span></template>
      </MTile>
      <MTile v-for="tool in others" :key="tool.id" action :icon="ICONS[tool.id] || 'build'" :title="tool.label" @click="tool.action()">
        <template #foot><MIcon v-if="tool.external" name="open_in_new" /></template>
      </MTile>
    </div>
  </MPage>
</template>

<script setup>
import { computed, onMounted, reactive, ref } from 'vue'
import { MPage, MTile, MIcon } from 'foundation-ui/vue'
import { controllerMeta } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 工具箱: the small tools as tiles (ControllerMeta controller test, stylus probe, stream timer, latency test). The controller test shows whether
// ControllerMeta is installed and its version, as the diagnostics card did.
const props = defineProps({ tools: { type: Array, default: () => [] } })
const { t } = useI18n()
const ICONS = { 'stream-timer': 'timer', 'latency-test': 'speed' }
const byId = (id) => props.tools.find((tool) => tool.id === id)
const others = computed(() => props.tools.filter((tool) => !['gamepad-test', 'stylus-input-probe'].includes(tool.id)))
const meta = reactive({ installed: false, version: '' })
const failed = ref(false)
const metaBadge = computed(() => (failed.value ? '' : meta.installed ? 'ok' : 'info'))
const metaLine = computed(() => (failed.value ? t.value.deviceHub.probeUnavailable : meta.installed ? (meta.version ? `v${meta.version}` : t.value.controllersHub.peripherals.installed) : t.value.controllersHub.peripherals.notInstalled))
onMounted(async () => {
  try { const r = await controllerMeta.probeStatus(); failed.value = !r?.success; if (r?.success) Object.assign(meta, r.data) } catch { failed.value = true }
})
</script>

<style>
.panel-toolgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px; }
.panel-toolgrid .m-tile { min-height: 132px; text-align: start; }
.panel-toolgrid .m-tile__title { white-space: normal; }
.panel-tool__sub { margin: 0; font: var(--m-body-small); color: var(--md-sys-color-on-surface-variant); }
.panel-tool__go { font: var(--m-label-medium); color: color-mix(in srgb, var(--md-sys-color-primary) 65%, var(--md-sys-color-on-surface)); }
</style>
